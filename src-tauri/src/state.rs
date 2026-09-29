use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::models::{AgentProfile, ProfileStatus};

pub struct RunningProcess {
    pub profile_id: String,
    pub pid: u32,
    pub manual_stop: Arc<AtomicBool>,
    #[cfg(unix)]
    pub pgid: libc::pid_t,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivePidRecord {
    pub profile_id: String,
    pub pid: u32,
    pub pgid: i32,
}

pub struct AppState {
    pub data_dir: PathBuf,
    pub profiles: Arc<Mutex<Vec<AgentProfile>>>,
    pub selected_profile_id: Arc<Mutex<Option<String>>>,
    pub running_processes: Arc<Mutex<HashMap<String, RunningProcess>>>,
    pub status_texts: Arc<Mutex<HashMap<String, String>>>,
    pub last_errors: Arc<Mutex<HashMap<String, Option<String>>>>,
    pub logs: Arc<Mutex<HashMap<String, Vec<String>>>>,
    pub restart_cancellations: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
}

impl AppState {
    pub fn new(data_dir: PathBuf) -> Self {
        let _ = fs::create_dir_all(&data_dir);

        // Auto clean any orphaned processes from a previous crash/panic/exit
        let pids_file = data_dir.join("active_pids.json");
        if let Ok(content) = fs::read_to_string(&pids_file) {
            if let Ok(records) = serde_json::from_str::<HashMap<String, ActivePidRecord>>(&content) {
                for (_id, rec) in records {
                    #[cfg(unix)]
                    unsafe {
                        if rec.pgid > 1 {
                            libc::kill(-rec.pgid, libc::SIGKILL);
                        }
                        if rec.pid > 1 {
                            libc::kill(rec.pid as libc::pid_t, libc::SIGKILL);
                        }
                    }
                    #[cfg(windows)]
                    {
                        let _ = std::process::Command::new("taskkill")
                            .args(["/F", "/T", "/PID", &rec.pid.to_string()])
                            .output();
                    }
                }
            }
            let _ = fs::remove_file(&pids_file);
        }

        let profiles_file = data_dir.join("profiles.json");

        let mut profiles = Vec::new();
        if profiles_file.exists() {
            if let Ok(content) = fs::read_to_string(&profiles_file) {
                if let Ok(parsed) = serde_json::from_str::<Vec<AgentProfile>>(&content) {
                    if !parsed.is_empty() {
                        profiles = parsed;
                    }
                }
            }
        }

        if profiles.is_empty() {
            profiles = vec![AgentProfile::default_profile()];
            let _ = fs::write(&profiles_file, serde_json::to_string_pretty(&profiles).unwrap_or_default());
        }

        let selected_id_file = data_dir.join("selected_id.txt");
        let selected_id = if selected_id_file.exists() {
            let id = fs::read_to_string(&selected_id_file).unwrap_or_default().trim().to_string();
            if profiles.iter().any(|p| p.id == id) {
                Some(id)
            } else {
                profiles.first().map(|p| p.id.clone())
            }
        } else {
            profiles.first().map(|p| p.id.clone())
        };

        Self {
            data_dir,
            profiles: Arc::new(Mutex::new(profiles)),
            selected_profile_id: Arc::new(Mutex::new(selected_id)),
            running_processes: Arc::new(Mutex::new(HashMap::new())),
            status_texts: Arc::new(Mutex::new(HashMap::new())),
            last_errors: Arc::new(Mutex::new(HashMap::new())),
            logs: Arc::new(Mutex::new(HashMap::new())),
            restart_cancellations: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn cancel_restart(&self, profile_id: &str) {
        let map = self.restart_cancellations.lock().await;
        if let Some(token) = map.get(profile_id) {
            token.store(true, Ordering::SeqCst);
        }
    }

    pub async fn get_or_create_restart_token(&self, profile_id: &str) -> Arc<AtomicBool> {
        let mut map = self.restart_cancellations.lock().await;
        let token = Arc::new(AtomicBool::new(false));
        map.insert(profile_id.to_string(), token.clone());
        token
    }

    pub fn record_active_pid(&self, profile_id: &str, pid: u32, pgid: i32) {
        let pids_file = self.data_dir.join("active_pids.json");
        let mut records: HashMap<String, ActivePidRecord> = fs::read_to_string(&pids_file)
            .ok()
            .and_then(|c| serde_json::from_str(&c).ok())
            .unwrap_or_default();
        records.insert(
            profile_id.to_string(),
            ActivePidRecord {
                profile_id: profile_id.to_string(),
                pid,
                pgid,
            },
        );
        let _ = fs::write(&pids_file, serde_json::to_string_pretty(&records).unwrap_or_default());
    }

    pub fn remove_active_pid(&self, profile_id: &str) {
        let pids_file = self.data_dir.join("active_pids.json");
        if let Ok(content) = fs::read_to_string(&pids_file) {
            if let Ok(mut records) = serde_json::from_str::<HashMap<String, ActivePidRecord>>(&content) {
                records.remove(profile_id);
                let _ = fs::write(&pids_file, serde_json::to_string_pretty(&records).unwrap_or_default());
            }
        }
    }

    pub fn get_active_pids(&self) -> HashMap<String, ActivePidRecord> {
        let pids_file = self.data_dir.join("active_pids.json");
        fs::read_to_string(&pids_file)
            .ok()
            .and_then(|c| serde_json::from_str(&c).ok())
            .unwrap_or_default()
    }

    pub async fn save_profiles(&self) -> Result<(), String> {
        let profiles_file = self.data_dir.join("profiles.json");
        let profiles = self.profiles.lock().await;
        let json = serde_json::to_string_pretty(&*profiles).map_err(|e| e.to_string())?;
        fs::write(profiles_file, json).map_err(|e| e.to_string())?;

        let selected_id_file = self.data_dir.join("selected_id.txt");
        let selected_id = self.selected_profile_id.lock().await;
        if let Some(id) = &*selected_id {
            let _ = fs::write(selected_id_file, id);
        }
        Ok(())
    }

    pub async fn get_profile_status(&self, profile_id: &str) -> ProfileStatus {
        let procs = self.running_processes.lock().await;
        let texts = self.status_texts.lock().await;
        let errors = self.last_errors.lock().await;

        let proc = procs.get(profile_id);
        let is_running = proc.is_some();
        let status_text = texts.get(profile_id).cloned().unwrap_or_else(|| {
            if is_running {
                "运行中".to_string()
            } else {
                "已停止".to_string()
            }
        });
        let pid = proc.map(|p| p.pid);
        let last_error = errors.get(profile_id).cloned().flatten();

        ProfileStatus {
            profile_id: profile_id.to_string(),
            is_running,
            status_text,
            pid,
            last_error,
        }
    }

    pub async fn get_all_statuses(&self) -> HashMap<String, ProfileStatus> {
        let profiles = self.profiles.lock().await.clone();
        let mut map = HashMap::new();
        for p in profiles {
            map.insert(p.id.clone(), self.get_profile_status(&p.id).await);
        }
        map
    }

    pub async fn append_log(&self, profile_id: &str, line: String) {
        let mut logs_map = self.logs.lock().await;
        let list = logs_map.entry(profile_id.to_string()).or_default();
        list.push(line);
        if list.len() > 6_000 {
            let drop_count = list.len() - 5_000;
            list.drain(0..drop_count);
        }
    }

    pub async fn get_logs(&self, profile_id: &str) -> Vec<String> {
        let logs_map = self.logs.lock().await;
        logs_map.get(profile_id).cloned().unwrap_or_default()
    }

    pub async fn clear_logs(&self, profile_id: &str) {
        let mut logs_map = self.logs.lock().await;
        if let Some(list) = logs_map.get_mut(profile_id) {
            list.clear();
        }
    }
}

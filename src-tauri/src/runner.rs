use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::models::{AgentProfile, ProfileLogMessage};
use crate::path_detect::{detect_default_path, expand_tilde};
use crate::state::{AppState, RunningProcess};

#[cfg(unix)]
pub async fn kill_process_tree(pgid: libc::pid_t) {
    if pgid <= 1 {
        return;
    }
    // Query any child processes under pgid
    let pchildren = tokio::process::Command::new("pgrep")
        .args(["-P", &pgid.to_string()])
        .output()
        .await
        .ok();

    unsafe {
        libc::kill(-pgid, libc::SIGTERM);
        libc::kill(pgid, libc::SIGTERM);
    }

    if let Some(out) = pchildren.as_ref() {
        let pids_str = String::from_utf8_lossy(&out.stdout);
        for line in pids_str.lines() {
            if let Ok(cpid) = line.trim().parse::<libc::pid_t>() {
                if cpid > 1 {
                    unsafe {
                        libc::kill(cpid, libc::SIGTERM);
                    }
                }
            }
        }
    }

    tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

    unsafe {
        libc::kill(-pgid, libc::SIGKILL);
        libc::kill(pgid, libc::SIGKILL);
    }

    if let Some(out) = pchildren {
        let pids_str = String::from_utf8_lossy(&out.stdout);
        for line in pids_str.lines() {
            if let Ok(cpid) = line.trim().parse::<libc::pid_t>() {
                if cpid > 1 {
                    unsafe {
                        libc::kill(cpid, libc::SIGKILL);
                    }
                }
            }
        }
    }
}

#[cfg(windows)]
pub async fn kill_process_tree(pid: u32) {
    if pid == 0 {
        return;
    }
    let _ = std::process::Command::new("taskkill")
        .args(["/F", "/T", "/PID", &pid.to_string()])
        .output();
}

pub fn extract_ports(cmd: &str) -> Vec<u16> {
    let mut ports = Vec::new();
    for token in cmd.split_whitespace() {
        let cleaned = token.trim_matches(|c: char| !c.is_numeric());
        if let Ok(p) = cleaned.parse::<u16>() {
            if (1024..=65535).contains(&p) && !ports.contains(&p) {
                ports.push(p);
            }
        }
    }
    ports
}

pub async fn kill_processes_on_ports(ports: &[u16]) {
    for port in ports {
        #[cfg(unix)]
        {
            // 1. Explicitly find all server PIDs LISTENING on this port (ignores client sockets like browsers)
            let lsof_res = tokio::process::Command::new("lsof")
                .args(["-nP", &format!("-tiTCP:{}", port), "-sTCP:LISTEN"])
                .output()
                .await;

            if let Ok(out) = lsof_res {
                let stdout = String::from_utf8_lossy(&out.stdout);
                for line in stdout.lines() {
                    let pid_str = line.trim();
                    if let Ok(pid) = pid_str.parse::<libc::pid_t>() {
                        if pid > 1 {
                            // Find PGID and terminate process group
                            if let Ok(pgid_out) = tokio::process::Command::new("ps")
                                .args(["-o", "pgid=", "-p", pid_str])
                                .output()
                                .await
                            {
                                let pgid_s = String::from_utf8_lossy(&pgid_out.stdout);
                                if let Ok(pgid) = pgid_s.trim().parse::<libc::pid_t>() {
                                    if pgid > 1 {
                                        unsafe {
                                            libc::kill(-pgid, libc::SIGKILL);
                                        }
                                    }
                                }
                            }

                            // Find PPID and terminate parent process if it's an orphan supervisor
                            if let Ok(ppid_out) = tokio::process::Command::new("ps")
                                .args(["-o", "ppid=", "-p", pid_str])
                                .output()
                                .await
                            {
                                let ppid_s = String::from_utf8_lossy(&ppid_out.stdout);
                                if let Ok(ppid) = ppid_s.trim().parse::<libc::pid_t>() {
                                    if ppid > 1 {
                                        unsafe {
                                            libc::kill(ppid, libc::SIGKILL);
                                        }
                                    }
                                }
                            }

                            unsafe {
                                libc::kill(pid, libc::SIGKILL);
                            }
                        }
                    }
                }
            }

            // Fallback: kill anything listening on the port
            let _ = tokio::process::Command::new("sh")
                .arg("-c")
                .arg(format!("lsof -tiTCP:{} -sTCP:LISTEN | xargs kill -9 2>/dev/null || true", port))
                .output()
                .await;
        }
        #[cfg(windows)]
        {
            let script = format!(
                "for /f \"tokens=5\" %a in ('netstat -aon ^| findstr :{}') do taskkill /F /T /PID %a",
                port
            );
            let _ = tokio::process::Command::new("cmd")
                .args(["/C", &script])
                .output()
                .await;
        }
    }
}

pub fn start_agent(
    app: AppHandle,
    profile: AgentProfile,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send>> {
    Box::pin(async move {
        start_agent_impl(app, profile).await
    })
}

async fn start_agent_impl(app: AppHandle, profile: AgentProfile) -> Result<(), String> {
    let state = app.state::<AppState>();
    let profile_id = profile.id.clone();

    // 1. Stop currently registered running process for THIS profile (this cancels any prior run's restart)
    stop_agent(&app, &profile_id).await?;

    // 2. Clean any lingering orphan process from previous app crash/panic
    if let Some(record) = state.get_active_pids().get(&profile_id) {
        #[cfg(unix)]
        kill_process_tree(record.pgid as libc::pid_t).await;
        #[cfg(windows)]
        kill_process_tree(record.pid).await;
        state.remove_active_pid(&profile_id);
    }

    // 3. Clean any lingering process occupying ports mentioned in command
    let ports = extract_ports(&profile.command);
    if !ports.is_empty() {
        kill_processes_on_ports(&ports).await;
    }

    // 4. Create fresh restart token for THIS new run (AFTER stop_agent has executed)
    let restart_token = state.get_or_create_restart_token(&profile_id).await;

    let cmd_text = profile.command.trim();
    if cmd_text.is_empty() {
        let err = "启动命令不能为空".to_string();
        state.last_errors.lock().await.insert(profile_id.clone(), Some(err.clone()));
        state.status_texts.lock().await.insert(profile_id.clone(), "启动失败".to_string());
        let _ = app.emit("profile-status-changed", state.get_profile_status(&profile_id).await);
        return Err(err);
    }

    let home = app.path().home_dir().ok();
    let home_ref = home.as_deref();

    let cwd = expand_tilde(&profile.working_directory, home_ref);
    if !cwd.exists() || !cwd.is_dir() {
        let err = format!("工作目录不存在：{}", cwd.to_string_lossy());
        state.last_errors.lock().await.insert(profile_id.clone(), Some(err.clone()));
        state.status_texts.lock().await.insert(profile_id.clone(), "启动失败".to_string());
        let _ = app.emit("profile-status-changed", state.get_profile_status(&profile_id).await);
        return Err(err);
    }

    // Determine runtime PATH
    let runtime_path = if !profile.custom_path.trim().is_empty() {
        profile.custom_path.trim().to_string()
    } else {
        detect_default_path(home_ref)
    };

    let mut env_map: HashMap<String, String> = std::env::vars().collect();
    env_map.insert("PATH".to_string(), runtime_path);
    env_map.insert("LC_ALL".to_string(), "en_US.UTF-8".to_string());
    env_map.insert("LANG".to_string(), "en_US.UTF-8".to_string());

    state.last_errors.lock().await.insert(profile_id.clone(), None);
    state.status_texts.lock().await.insert(profile_id.clone(), "正在启动…".to_string());
    let _ = app.emit("profile-status-changed", state.get_profile_status(&profile_id).await);

    let start_msg = format!("[服务] 正在启动 Agent: {}", profile.name);
    state.append_log(&profile_id, format!("{}\n", start_msg)).await;
    let _ = app.emit("profile-log-output", ProfileLogMessage {
        profile_id: profile_id.clone(),
        text: start_msg,
        is_error: false,
    });

    // Spawn child process with appropriate shell
    #[cfg(unix)]
    let mut child = {
        let shell_input = profile.shell.trim();
        let shell = if !shell_input.is_empty() && shell_input != "auto" {
            shell_input
        } else if Path::new("/bin/zsh").exists() {
            "/bin/zsh"
        } else if Path::new("/bin/bash").exists() {
            "/bin/bash"
        } else {
            "sh"
        };

        let mut cmd = tokio::process::Command::new(shell);
        cmd.arg("-c").arg(&profile.command);
        cmd.current_dir(&cwd);
        cmd.envs(&env_map);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        unsafe {
            cmd.pre_exec(|| {
                libc::setpgid(0, 0);
                Ok(())
            });
        }

        cmd.spawn().map_err(|e| format!("启动进程失败 (Shell: {}): {}", shell, e))?
    };

    #[cfg(windows)]
    let mut child = {
        let shell_input = profile.shell.trim();
        let (shell, args): (&str, Vec<&str>) = if shell_input.ends_with("cmd.exe") || shell_input.eq_ignore_ascii_case("cmd") {
            (shell_input, vec!["/C", &profile.command])
        } else if !shell_input.is_empty() && shell_input != "auto" {
            (shell_input, vec!["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &profile.command])
        } else {
            ("powershell.exe", vec!["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &profile.command])
        };

        let mut cmd = tokio::process::Command::new(shell);
        cmd.args(args);
        cmd.current_dir(&cwd);
        cmd.envs(&env_map);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        cmd.spawn().map_err(|e| format!("启动进程失败 (Shell: {}): {}", shell, e))?
    };

    let pid = child.id().unwrap_or(0);
    #[cfg(unix)]
    let pgid = pid as libc::pid_t;
    #[cfg(not(unix))]
    let pgid = 0i32;

    let manual_stop = Arc::new(AtomicBool::new(false));

    let running = RunningProcess {
        profile_id: profile_id.clone(),
        pid,
        manual_stop: manual_stop.clone(),
        #[cfg(unix)]
        pgid,
    };

    state.running_processes.lock().await.insert(profile_id.clone(), running);
    state.status_texts.lock().await.insert(profile_id.clone(), "运行中".to_string());
    state.record_active_pid(&profile_id, pid, pgid as i32);
    let _ = app.emit("profile-status-changed", state.get_profile_status(&profile_id).await);

    let started_msg = format!("[服务] Agent '{}' 进程已启动 (PID: {})", profile.name, pid);
    state.append_log(&profile_id, format!("{}\n", started_msg)).await;
    let _ = app.emit("profile-log-output", ProfileLogMessage {
        profile_id: profile_id.clone(),
        text: started_msg,
        is_error: false,
    });

    // Handle stdout stream
    if let Some(stdout) = child.stdout.take() {
        let app_handle = app.clone();
        let pid_clone = profile_id.clone();
        tauri::async_runtime::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let st = app_handle.state::<AppState>();
                st.append_log(&pid_clone, format!("{}\n", line)).await;
                let _ = app_handle.emit("profile-log-output", ProfileLogMessage {
                    profile_id: pid_clone.clone(),
                    text: line,
                    is_error: false,
                });
            }
        });
    }

    // Handle stderr stream
    if let Some(stderr) = child.stderr.take() {
        let app_handle = app.clone();
        let pid_clone = profile_id.clone();
        tauri::async_runtime::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let st = app_handle.state::<AppState>();
                st.append_log(&pid_clone, format!("{}\n", line)).await;
                let is_err = line.to_lowercase().contains("error") || line.to_lowercase().contains("failed");
                if is_err {
                    st.last_errors.lock().await.insert(pid_clone.clone(), Some(line.clone()));
                }
                let _ = app_handle.emit("profile-log-output", ProfileLogMessage {
                    profile_id: pid_clone.clone(),
                    text: line,
                    is_error: true,
                });
            }
        });
    }

    // Handle child exit & restart policy
    {
        let app_handle = app.clone();
        let target_profile_id = profile_id.clone();
        let profile_clone = profile.clone();
        let manual_stop_check = manual_stop.clone();
        let restart_cancel_token = restart_token.clone();
        let this_pid = pid;

        tauri::async_runtime::spawn(async move {
            let status = child.wait().await;
            let st = app_handle.state::<AppState>();

            // Isolate procs lock so it drops IMMEDIATELY before get_profile_status is called!
            let is_current = {
                let mut procs = st.running_processes.lock().await;
                let cur = procs.get(&target_profile_id).map(|p| p.pid) == Some(this_pid);
                if cur {
                    procs.remove(&target_profile_id);
                }
                cur
            };

            st.remove_active_pid(&target_profile_id);

            if is_current {
                st.status_texts.lock().await.insert(target_profile_id.clone(), "已停止".to_string());
                st.last_errors.lock().await.insert(target_profile_id.clone(), None);

                let was_manual_stopped = manual_stop_check.load(Ordering::SeqCst)
                    || restart_cancel_token.load(Ordering::SeqCst);

                let is_failure = match &status {
                    Ok(s) => !s.success(),
                    Err(_) => true,
                };

                let exit_msg = if was_manual_stopped {
                    "[服务] Agent 进程已停止".to_string()
                } else {
                    match status {
                        Ok(s) => format!("[服务] Agent 进程已退出 (代码: {:?})", s.code()),
                        Err(e) => format!("[服务] Agent 进程等待出错: {}", e),
                    }
                };

                st.append_log(&target_profile_id, format!("{}\n", exit_msg)).await;
                let _ = app_handle.emit("profile-log-output", ProfileLogMessage {
                    profile_id: target_profile_id.clone(),
                    text: exit_msg,
                    is_error: false,
                });

                // Safe to call get_profile_status now - procs was already dropped!
                let status_payload = st.get_profile_status(&target_profile_id).await;
                let _ = app_handle.emit("profile-status-changed", status_payload);

                if !was_manual_stopped {
                    let policy = profile_clone.restart_policy.as_str();
                    let should_restart = match policy {
                        "always" => true,
                        "on-failure" => is_failure,
                        _ => false,
                    };

                    if should_restart {
                        let delay = profile_clone.restart_delay.max(1);
                        let restart_notice = format!(
                            "[重启策略] 检测到进程退出，将在 {} 秒后根据策略 '{}' 自动重启…",
                            delay, policy
                        );
                        st.append_log(&target_profile_id, format!("{}\n", restart_notice)).await;
                        let _ = app_handle.emit("profile-log-output", ProfileLogMessage {
                            profile_id: target_profile_id.clone(),
                            text: restart_notice,
                            is_error: false,
                        });

                        let restart_app = app_handle.clone();
                        let target_p = profile_clone.clone();
                        let restart_pid = target_profile_id.clone();
                        let manual_check_timer = manual_stop_check.clone();
                        let token_check_timer = restart_cancel_token.clone();

                        tauri::async_runtime::spawn(async move {
                            tokio::time::sleep(tokio::time::Duration::from_secs(delay)).await;
                            let app_state = restart_app.state::<AppState>();
                            let is_still_stopped = {
                                let procs_guard = app_state.running_processes.lock().await;
                                !procs_guard.contains_key(&restart_pid)
                            };
                            let was_stopped = manual_check_timer.load(Ordering::SeqCst)
                                || token_check_timer.load(Ordering::SeqCst);

                            if is_still_stopped && !was_stopped {
                                let _ = start_agent(restart_app, target_p).await;
                            }
                        });
                    }
                }
            }
        });
    }

    Ok(())
}

pub async fn stop_agent(app: &AppHandle, profile_id: &str) -> Result<(), String> {
    let state = app.state::<AppState>();
    state.cancel_restart(profile_id).await;
    state.remove_active_pid(profile_id);

    let proc_opt = {
        let mut procs = state.running_processes.lock().await;
        procs.remove(profile_id)
    };

    if let Some(proc) = proc_opt {
        proc.manual_stop.store(true, Ordering::SeqCst);

        state.status_texts.lock().await.insert(profile_id.to_string(), "正在停止…".to_string());
        let _ = app.emit("profile-status-changed", state.get_profile_status(profile_id).await);

        let stop_msg = "[服务] 正在停止 Agent…".to_string();
        state.append_log(profile_id, format!("{}\n", stop_msg)).await;
        let _ = app.emit("profile-log-output", ProfileLogMessage {
            profile_id: profile_id.to_string(),
            text: stop_msg,
            is_error: false,
        });

        #[cfg(unix)]
        kill_process_tree(proc.pgid).await;

        #[cfg(windows)]
        kill_process_tree(proc.pid).await;

        state.status_texts.lock().await.insert(profile_id.to_string(), "已停止".to_string());
        let _ = app.emit("profile-status-changed", state.get_profile_status(profile_id).await);

        let stopped_msg = "[服务] Agent 已停止".to_string();
        state.append_log(profile_id, format!("{}\n", stopped_msg)).await;
        let _ = app.emit("profile-log-output", ProfileLogMessage {
            profile_id: profile_id.to_string(),
            text: stopped_msg,
            is_error: false,
        });
    } else {
        state.status_texts.lock().await.insert(profile_id.to_string(), "已停止".to_string());
        let _ = app.emit("profile-status-changed", state.get_profile_status(profile_id).await);
    }

    Ok(())
}

pub async fn clean_residual_for_profile(app: &AppHandle, profile_id: &str) -> Result<String, String> {
    let state = app.state::<AppState>();

    // 0. Cancel any pending auto-restart timers
    state.cancel_restart(profile_id).await;

    // 1. Normal stop
    let _ = stop_agent(app, profile_id).await;

    // 2. Kill by recorded PID/PGID
    if let Some(record) = state.get_active_pids().get(profile_id) {
        #[cfg(unix)]
        kill_process_tree(record.pgid as libc::pid_t).await;
        #[cfg(windows)]
        kill_process_tree(record.pid).await;
        state.remove_active_pid(profile_id);
    }

    // 3. Kill ports if found in command
    let profiles = state.profiles.lock().await;
    let target = profiles.iter().find(|p| p.id == profile_id);
    let mut cleaned_ports = Vec::new();
    if let Some(p) = target {
        let ports = extract_ports(&p.command);
        if !ports.is_empty() {
            cleaned_ports = ports.clone();
            kill_processes_on_ports(&ports).await;
        }
    }

    // 4. Force ensure running_processes and status are clean
    {
        let mut procs = state.running_processes.lock().await;
        procs.remove(profile_id);
    }
    state.last_errors.lock().await.insert(profile_id.to_string(), None);
    state.status_texts.lock().await.insert(profile_id.to_string(), "已停止".to_string());
    let _ = app.emit("profile-status-changed", state.get_profile_status(profile_id).await);

    let clean_msg = if cleaned_ports.is_empty() {
        "[系统] 已成功清理孤儿进程并重置状态".to_string()
    } else {
        format!("[系统] 已强制杀死关联孤儿进程，并释放占用端口: {:?}", cleaned_ports)
    };
    state.append_log(profile_id, format!("{}\n", clean_msg)).await;
    let _ = app.emit("profile-log-output", ProfileLogMessage {
        profile_id: profile_id.to_string(),
        text: clean_msg.clone(),
        is_error: false,
    });

    Ok(clean_msg)
}

pub async fn stop_all_agents(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let profile_ids: Vec<String> = {
        let procs = state.running_processes.lock().await;
        procs.keys().cloned().collect()
    };
    for pid in profile_ids {
        let _ = stop_agent(app, &pid).await;
    }
    Ok(())
}

pub async fn restart_agent(app: AppHandle, profile: AgentProfile) -> Result<(), String> {
    stop_agent(&app, &profile.id).await?;
    start_agent(app, profile).await
}

use std::path::{Path, PathBuf};

pub fn expand_tilde(path: &str, home: Option<&Path>) -> PathBuf {
    if path == "~" {
        return home.map(|h| h.to_path_buf()).unwrap_or_else(|| PathBuf::from(path));
    }
    if let Some(stripped) = path.strip_prefix("~/") {
        if let Some(h) = home {
            return h.join(stripped);
        }
    }
    PathBuf::from(path)
}

pub fn abbreviate_home(path: &str, home: Option<&Path>) -> String {
    if let Some(h) = home {
        let home_str = h.to_string_lossy();
        if path == home_str {
            return "~".to_string();
        }
        let prefix = format!("{}/", home_str);
        if path.starts_with(&prefix) {
            return format!("~{}", &path[home_str.len()..]);
        }
    }
    path.to_string()
}

pub fn detect_default_path(home: Option<&Path>) -> String {
    let current_path = std::env::var("PATH").unwrap_or_default();
    let mut paths: Vec<PathBuf> = std::env::split_paths(&current_path).collect();

    #[cfg(target_os = "macos")]
    {
        if let Some(h) = home {
            paths.push(h.join(".local/bin"));
            paths.push(h.join(".cargo/bin"));
            paths.push(h.join(".pyenv/shims"));
            paths.push(h.join(".npm-global/bin"));
            paths.push(h.join(".volta/bin"));

            let nvm_versions = h.join(".nvm/versions/node");
            if let Ok(entries) = std::fs::read_dir(nvm_versions) {
                for entry in entries.flatten() {
                    let bin_path = entry.path().join("bin");
                    if bin_path.exists() {
                        paths.push(bin_path);
                    }
                }
            }
        }
        paths.push(PathBuf::from("/opt/homebrew/bin"));
        paths.push(PathBuf::from("/usr/local/bin"));
        paths.push(PathBuf::from("/usr/bin"));
        paths.push(PathBuf::from("/bin"));
        paths.push(PathBuf::from("/usr/sbin"));
        paths.push(PathBuf::from("/sbin"));
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(h) = home {
            paths.push(h.join(".local/bin"));
            paths.push(h.join(".cargo/bin"));
            paths.push(h.join(".pyenv/shims"));
            paths.push(h.join(".npm-global/bin"));
            paths.push(h.join(".volta/bin"));

            let nvm_versions = h.join(".nvm/versions/node");
            if let Ok(entries) = std::fs::read_dir(nvm_versions) {
                for entry in entries.flatten() {
                    let bin_path = entry.path().join("bin");
                    if bin_path.exists() {
                        paths.push(bin_path);
                    }
                }
            }
        }
        paths.push(PathBuf::from("/usr/local/bin"));
        paths.push(PathBuf::from("/usr/bin"));
        paths.push(PathBuf::from("/bin"));
        paths.push(PathBuf::from("/usr/local/games"));
        paths.push(PathBuf::from("/usr/games"));
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(h) = home {
            paths.push(h.join(".cargo\\bin"));
            paths.push(h.join("AppData\\Roaming\\npm"));
            paths.push(h.join("AppData\\Local\\Programs\\Python\\Python312\\Scripts"));
            paths.push(h.join("AppData\\Local\\Programs\\Python\\Python311\\Scripts"));
        }
    }

    let mut seen = std::collections::HashSet::new();
    let mut unique_paths = Vec::new();
    for p in paths {
        let s = p.to_string_lossy().to_string();
        if !s.is_empty() && seen.insert(s.clone()) {
            unique_paths.push(p);
        }
    }

    std::env::join_paths(unique_paths)
        .unwrap_or_default()
        .to_string_lossy()
        .to_string()
}

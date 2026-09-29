use serde::{Deserialize, Serialize};

fn default_shell() -> String {
    "auto".to_string()
}

fn default_cwd() -> String {
    "~".to_string()
}

fn default_restart_policy() -> String {
    "no".to_string()
}

fn default_restart_delay() -> u64 {
    3
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentProfile {
    pub id: String,
    pub name: String,
    #[serde(alias = "script")]
    pub command: String,
    #[serde(default = "default_shell")]
    pub shell: String,
    #[serde(default = "default_cwd")]
    pub working_directory: String,
    #[serde(default)]
    pub custom_path: String,
    #[serde(default = "default_restart_policy")]
    pub restart_policy: String,
    #[serde(default = "default_restart_delay")]
    pub restart_delay: u64,
    #[serde(default)]
    pub auto_start: bool,
}

impl AgentProfile {
    pub fn default_profile() -> Self {
        #[cfg(windows)]
        let (default_shell, default_command) = (
            "powershell.exe".to_string(),
            r#"# 在此处编写启动 Agent 的 Shell 命令
Write-Host "AI Agent 启动中..."
Write-Host "当前工作目录: $(Get-Location)"
python -m http.server 8080
"#.to_string(),
        );

        #[cfg(not(windows))]
        let (default_shell, default_command) = (
            "auto".to_string(),
            r#"# 在此处编写启动 Agent 的 Shell 命令
echo "AI Agent 启动中..."
echo "当前工作目录: $(pwd)"
python3 -m http.server 8080
"#.to_string(),
        );

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Demo Agent".to_string(),
            command: default_command,
            shell: default_shell,
            working_directory: "~".to_string(),
            custom_path: "".to_string(),
            restart_policy: "no".to_string(),
            restart_delay: 3,
            auto_start: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileStatus {
    pub profile_id: String,
    pub is_running: bool,
    pub status_text: String,
    pub pid: Option<u32>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileLogMessage {
    pub profile_id: String,
    pub text: String,
    pub is_error: bool,
}

use std::collections::HashMap;
use tauri::{AppHandle, Manager};
use crate::models::{AgentProfile, ProfileStatus};
use crate::path_detect::detect_default_path;
use crate::runner::{restart_agent, start_agent, stop_agent, stop_all_agents};
use crate::state::AppState;

#[tauri::command]
pub async fn get_profiles(app: AppHandle) -> Result<Vec<AgentProfile>, String> {
    let state = app.state::<AppState>();
    let profiles = state.profiles.lock().await;
    Ok(profiles.clone())
}

#[tauri::command]
pub async fn save_profiles(app: AppHandle, profiles: Vec<AgentProfile>) -> Result<(), String> {
    let state = app.state::<AppState>();
    {
        let mut p = state.profiles.lock().await;
        *p = profiles;
    }
    state.save_profiles().await
}

#[tauri::command]
pub async fn get_selected_profile_id(app: AppHandle) -> Result<Option<String>, String> {
    let state = app.state::<AppState>();
    let id = state.selected_profile_id.lock().await;
    Ok(id.clone())
}

#[tauri::command]
pub async fn select_profile(app: AppHandle, id: String) -> Result<(), String> {
    let state = app.state::<AppState>();
    {
        let mut selected = state.selected_profile_id.lock().await;
        *selected = Some(id);
    }
    state.save_profiles().await
}

#[tauri::command]
pub async fn start_agent_by_id(app: AppHandle, profile_id: String) -> Result<(), String> {
    let state = app.state::<AppState>();
    let profile = {
        let profiles = state.profiles.lock().await;
        profiles.iter().find(|p| p.id == profile_id).cloned()
    };

    if let Some(p) = profile {
        start_agent(app, p).await
    } else {
        Err("未找到指定的 Agent 配置".to_string())
    }
}

#[tauri::command]
pub async fn stop_agent_by_id(app: AppHandle, profile_id: String) -> Result<(), String> {
    stop_agent(&app, &profile_id).await
}

#[tauri::command]
pub async fn restart_agent_by_id(app: AppHandle, profile_id: String) -> Result<(), String> {
    let state = app.state::<AppState>();
    let profile = {
        let profiles = state.profiles.lock().await;
        profiles.iter().find(|p| p.id == profile_id).cloned()
    };

    if let Some(p) = profile {
        restart_agent(app, p).await
    } else {
        Err("未找到指定的 Agent 配置".to_string())
    }
}

#[tauri::command]
pub async fn stop_all_agents_command(app: AppHandle) -> Result<(), String> {
    stop_all_agents(&app).await
}

#[tauri::command]
pub async fn exit_app(app: AppHandle) -> Result<(), String> {
    let _ = stop_all_agents(&app).await;
    app.exit(0);
    Ok(())
}

#[tauri::command]
pub async fn get_profile_status(app: AppHandle, profile_id: String) -> Result<ProfileStatus, String> {
    let state = app.state::<AppState>();
    Ok(state.get_profile_status(&profile_id).await)
}

#[tauri::command]
pub async fn get_all_statuses(app: AppHandle) -> Result<HashMap<String, ProfileStatus>, String> {
    let state = app.state::<AppState>();
    Ok(state.get_all_statuses().await)
}

#[tauri::command]
pub async fn get_profile_logs(app: AppHandle, profile_id: String) -> Result<Vec<String>, String> {
    let state = app.state::<AppState>();
    Ok(state.get_logs(&profile_id).await)
}

#[tauri::command]
pub async fn clear_profile_logs(app: AppHandle, profile_id: String) -> Result<(), String> {
    let state = app.state::<AppState>();
    state.clear_logs(&profile_id).await;
    Ok(())
}

#[tauri::command]
pub async fn detect_path(app: AppHandle) -> Result<String, String> {
    let home = app.path().home_dir().ok();
    Ok(detect_default_path(home.as_deref()))
}

#[tauri::command]
pub async fn clean_residual_processes(app: AppHandle, profile_id: String) -> Result<String, String> {
    crate::runner::clean_residual_for_profile(&app, &profile_id).await
}

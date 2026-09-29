pub mod commands;
pub mod menu;
pub mod models;
pub mod path_detect;
pub mod runner;
pub mod state;
pub mod tray;

use std::path::PathBuf;
use tauri::{Manager, WindowEvent};
use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| PathBuf::from(".ai_agent_launcher"));

            let state = AppState::new(app_data_dir);
            app.manage(state);

            // Setup Native Application Menu (with Language switcher)
            if let Err(e) = menu::setup_app_menu(app.handle()) {
                eprintln!("Failed to setup application menu: {}", e);
            }

            // Setup System Tray
            if let Err(e) = tray::setup_tray(app.handle()) {
                eprintln!("Failed to setup tray: {}", e);
            }

            // Check auto-start profiles
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                let state = app_handle.state::<AppState>();
                let profiles = state.profiles.lock().await.clone();

                for profile in profiles {
                    if profile.auto_start {
                        let _ = runner::start_agent(app_handle.clone(), profile).await;
                    }
                }
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                // Hide window instead of closing, so app stays in tray
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_profiles,
            commands::save_profiles,
            commands::get_selected_profile_id,
            commands::select_profile,
            commands::start_agent_by_id,
            commands::stop_agent_by_id,
            commands::restart_agent_by_id,
            commands::stop_all_agents_command,
            commands::get_profile_status,
            commands::get_all_statuses,
            commands::get_profile_logs,
            commands::clear_profile_logs,
            commands::detect_path,
            commands::clean_residual_processes,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}


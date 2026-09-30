pub mod commands;
pub mod menu;
pub mod models;
pub mod path_detect;
pub mod runner;
pub mod state;
pub mod tray;
pub mod updater;

use std::path::PathBuf;
use tauri::{Manager, WindowEvent};
use state::AppState;

#[cfg(target_os = "windows")]
extern "system" {
    fn GetAsyncKeyState(vKey: i32) -> i16;
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--autostart"]),
        ))
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
        .on_menu_event(|app, event| {
            if event.id().as_ref() == "app_quit" {
                let app_clone = app.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = runner::stop_all_agents(&app_clone).await;
                    app_clone.exit(0);
                });
            } else if event.id().as_ref() == "menu_check_update" {
                use tauri::Emitter;
                let _ = app.emit("menu-check-update", ());
            }
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                #[cfg(target_os = "windows")]
                let is_alt_f4 = unsafe {
                    // VK_MENU = 0x12 (Alt key), VK_F4 = 0x73 (F4 key)
                    let alt_down = (GetAsyncKeyState(0x12) as u16 & 0x8000) != 0;
                    let f4_down = (GetAsyncKeyState(0x73) as u16 & 0x8000) != 0;
                    alt_down || f4_down
                };
                #[cfg(not(target_os = "windows"))]
                let is_alt_f4 = false;

                if is_alt_f4 {
                    // User explicitly pressed Alt+F4 on Windows: gracefully stop agents & quit
                    let app_handle = window.app_handle().clone();
                    tauri::async_runtime::spawn(async move {
                        let _ = runner::stop_all_agents(&app_handle).await;
                        app_handle.exit(0);
                    });
                } else {
                    // User clicked close button (X / red traffic light): hide to tray
                    let _ = window.hide();
                    #[cfg(target_os = "macos")]
                    let _ = window.app_handle().set_activation_policy(tauri::ActivationPolicy::Accessory);
                    api.prevent_close();
                }
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
            commands::exit_app,
            commands::check_app_update,
            commands::download_and_install_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}


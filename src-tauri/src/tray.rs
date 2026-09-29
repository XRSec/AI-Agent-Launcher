use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};
use crate::runner::stop_all_agents;

pub fn setup_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let menu = MenuBuilder::new(app)
        .item(&MenuItemBuilder::with_id("show_window", "显示主窗口").build(app)?)
        .separator()
        .item(&MenuItemBuilder::with_id("quit", "退出").build(app)?)
        .build()?;

    let icon = app.default_window_icon().cloned().ok_or("No default icon found")?;

    let _tray = TrayIconBuilder::with_id("main-tray")
        .tooltip("AI Agent Launcher")
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            match event.id().as_ref() {
                "show_window" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.unminimize();
                        let _ = window.set_focus();
                    }
                }
                "quit" => {
                    let app_clone = app.clone();
                    tauri::async_runtime::spawn(async move {
                        let _ = stop_all_agents(&app_clone).await;
                        app_clone.exit(0);
                    });
                }
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                if let Some(window) = tray.app_handle().get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.unminimize();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app)?;

    Ok(())
}

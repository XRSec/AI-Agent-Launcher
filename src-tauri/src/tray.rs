use tauri::{
    menu::{CheckMenuItemBuilder, MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Listener, Manager,
};
use tauri_plugin_autostart::ManagerExt;
use crate::runner::stop_all_agents;

pub fn setup_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let autostart_enabled = app.autolaunch().is_enabled().unwrap_or(false);
    let autostart_item = CheckMenuItemBuilder::with_id("toggle_autostart", "开机自启动")
        .checked(autostart_enabled)
        .build(app)?;

    let autostart_item_listener = autostart_item.clone();
    app.listen("frontend-autostart-changed", move |event| {
        if let Ok(enabled) = serde_json::from_str::<bool>(event.payload()) {
            let _ = autostart_item_listener.set_checked(enabled);
        }
    });

    let menu = MenuBuilder::new(app)
        .item(&MenuItemBuilder::with_id("show_window", "显示主窗口").build(app)?)
        .separator()
        .item(&autostart_item)
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
                "toggle_autostart" => {
                    let autolaunch = app.autolaunch();
                    let is_on = autolaunch.is_enabled().unwrap_or(false);
                    if is_on {
                        let _ = autolaunch.disable();
                    } else {
                        let _ = autolaunch.enable();
                    }
                    let new_state = !is_on;
                    let _ = app.emit("autostart-changed", new_state);
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

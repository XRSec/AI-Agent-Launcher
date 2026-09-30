use tauri::{
    menu::{MenuBuilder, MenuItemBuilder, SubmenuBuilder},
    AppHandle,
};
pub fn setup_app_menu(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let check_update_item = MenuItemBuilder::with_id("menu_check_update", "检查更新...")
        .build(app)?;

    let quit_item = MenuItemBuilder::with_id("app_quit", "退出 AI Agent Launcher")
        .accelerator("CmdOrCtrl+Q")
        .build(app)?;

    // 1. Main App Submenu (macOS AI Agent Launcher 主菜单)
    let app_submenu = SubmenuBuilder::new(app, "AI Agent Launcher")
        .about(None)
        .item(&check_update_item)
        .separator()
        .services()
        .separator()
        .hide()
        .hide_others()
        .show_all()
        .separator()
        .item(&quit_item)
        .build()?;

    // 2. Edit Submenu (Crucial for clipboard and text input shortcuts Cmd+C, Cmd+V, Cmd+A)
    let edit_submenu = SubmenuBuilder::new(app, "Edit")
        .undo()
        .redo()
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .build()?;

    // 3. Window Submenu
    let window_submenu = SubmenuBuilder::new(app, "Window")
        .minimize()
        .separator()
        .close_window()
        .build()?;

    let menu = MenuBuilder::new(app)
        .item(&app_submenu)
        .item(&edit_submenu)
        .item(&window_submenu)
        .build()?;

    app.set_menu(menu)?;

    Ok(())
}

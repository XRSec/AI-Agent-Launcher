use tauri::{
    menu::{MenuBuilder, SubmenuBuilder},
    AppHandle,
};

pub fn setup_app_menu(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Main App Submenu (macOS AI Agent Launcher 主菜单)
    let app_submenu = SubmenuBuilder::new(app, "AI Agent Launcher")
        .about(None)
        .separator()
        .services()
        .separator()
        .hide()
        .hide_others()
        .show_all()
        .separator()
        .quit()
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

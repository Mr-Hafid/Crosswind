use tauri::image::Image;
use tauri::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager};

const MAIN_WINDOW_LABEL: &str = "main";
const OPEN_ITEM_ID: &str = "open";
const PREVIEW_GLOW_ITEM_ID: &str = "preview-glow";
const QUIT_ITEM_ID: &str = "quit";

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let open_item = MenuItem::with_id(app, OPEN_ITEM_ID, "Open Crosswind", true, None::<&str>)?;
    let preview_item = MenuItem::with_id(
        app,
        PREVIEW_GLOW_ITEM_ID,
        "Preview the Glow",
        true,
        None::<&str>,
    )?;
    let quit_item = MenuItem::with_id(app, QUIT_ITEM_ID, "Quit Crosswind", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&open_item, &preview_item, &separator, &quit_item])?;

    TrayIconBuilder::with_id("crosswind")
        .icon(Image::from_bytes(include_bytes!("../icons/tray-template.png"))?)
        .icon_as_template(true)
        .tooltip("Crosswind")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(handle_menu_event)
        .build(app)?;
    Ok(())
}

pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn handle_menu_event(app: &AppHandle, event: MenuEvent) {
    match event.id.as_ref() {
        OPEN_ITEM_ID => show_main_window(app),
        PREVIEW_GLOW_ITEM_ID => {
            let _ = app.emit_to(MAIN_WINDOW_LABEL, "preview-glow-requested", ());
        }
        QUIT_ITEM_ID => app.exit(0),
        _ => {}
    }
}

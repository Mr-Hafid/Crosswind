use tauri::{AppHandle, WebviewUrl, WebviewWindowBuilder};

pub const OVERLAY_LABEL: &str = "overlay";

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let overlay = WebviewWindowBuilder::new(app, OVERLAY_LABEL, WebviewUrl::App("overlay.html".into()))
        .title("Crosswind Glow")
        .transparent(true)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .focused(false)
        .shadow(false)
        .visible_on_all_workspaces(true)
        .visible(false)
        .build()?;
    if let Some(monitor) = overlay.primary_monitor()? {
        overlay.set_position(*monitor.position())?;
        overlay.set_size(*monitor.size())?;
    }
    overlay.set_ignore_cursor_events(true)?;
    overlay.show()?;
    Ok(())
}

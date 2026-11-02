use tauri::AppHandle;

#[cfg(desktop)]
mod desktop {
    use tauri::{AppHandle, Manager};
    use tauri_plugin_autostart::{MacosLauncher, ManagerExt};

    const STARTED_AT_LOGIN_ARGUMENT: &str = "--started-at-login";

    pub fn plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
        tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec![STARTED_AT_LOGIN_ARGUMENT]),
        )
    }

    pub fn hide_main_window_if_started_at_login(app: &AppHandle) {
        if !std::env::args().any(|argument| argument == STARTED_AT_LOGIN_ARGUMENT) {
            return;
        }
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.hide();
        }
    }

    pub fn is_enabled(app: &AppHandle) -> Result<bool, String> {
        app.autolaunch().is_enabled().map_err(|error| error.to_string())
    }

    pub fn set_enabled(app: &AppHandle, enabled: bool) -> Result<bool, String> {
        let autolaunch = app.autolaunch();
        let result = if enabled {
            autolaunch.enable()
        } else {
            autolaunch.disable()
        };
        result.map_err(|error| error.to_string())?;
        autolaunch.is_enabled().map_err(|error| error.to_string())
    }
}

#[cfg(desktop)]
pub use desktop::{hide_main_window_if_started_at_login, plugin};

#[tauri::command]
pub fn launch_at_login_enabled(app: AppHandle) -> Result<bool, String> {
    #[cfg(desktop)]
    return desktop::is_enabled(&app);
    #[cfg(mobile)]
    {
        let _ = app;
        Err("launching at login is only available on desktop".to_owned())
    }
}

#[tauri::command]
pub fn set_launch_at_login(app: AppHandle, enabled: bool) -> Result<bool, String> {
    #[cfg(desktop)]
    return desktop::set_enabled(&app, enabled);
    #[cfg(mobile)]
    {
        let _ = (app, enabled);
        Err("launching at login is only available on desktop".to_owned())
    }
}

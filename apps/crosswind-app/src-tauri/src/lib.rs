mod launch_at_login;
#[cfg(desktop)]
mod overlay;
mod receiver;
#[cfg(target_os = "macos")]
mod system_airdrop;
mod thumbnail;
#[cfg(desktop)]
mod tray;

#[cfg(desktop)]
use tauri::WindowEvent;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let builder = tauri::Builder::default();
    #[cfg(desktop)]
    let builder = builder
        .on_window_event(hide_main_window_instead_of_closing)
        .plugin(launch_at_login::plugin());
    builder
        .plugin(tauri_plugin_opener::init())
        .manage(receiver::ReceiverState::default())
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            #[cfg(desktop)]
            {
                overlay::create(app.handle())?;
                tray::create(app.handle())?;
                launch_at_login::hide_main_window_if_started_at_login(app.handle());
            }
            #[cfg(target_os = "macos")]
            system_airdrop::watch_downloads(app.handle());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            receiver::list_interfaces,
            receiver::default_download_dir,
            receiver::receiver_status,
            receiver::start_receiving,
            receiver::stop_receiving,
            receiver::answer_request,
            receiver::preview_glow,
            receiver::reveal_received,
            launch_at_login::launch_at_login_enabled,
            launch_at_login::set_launch_at_login,
        ])
        .run(tauri::generate_context!())
        .expect("Crosswind failed to start");
}

#[cfg(desktop)]
fn hide_main_window_instead_of_closing(window: &tauri::Window, event: &WindowEvent) {
    if let WindowEvent::CloseRequested { api, .. } = event {
        if window.label() == "main" {
            api.prevent_close();
            let _ = window.hide();
        }
    }
}

mod server_runner;
mod tray;

use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Default)]
pub struct AppState {
    pub server_port: Arc<RwLock<Option<u16>>>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_state = AppState::default();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(app_state)
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            tray::setup_tray(app)?;

            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                server_runner::spawn_server(handle).await;
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

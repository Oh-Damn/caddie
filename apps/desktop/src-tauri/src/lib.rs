mod agents;
mod agentwatch;
mod call;
mod chords;
mod clipboard;
mod commands;
mod context;
mod cursor;
mod devlog;
mod dto;
mod mdns;
mod net;
mod os;
mod persist;
mod plan_usage;
mod plugins;
mod profiles;
mod protocol;
mod providers;
mod server;
mod state;
mod tray;
mod uninstall;

use state::AppState;
use tauri::Manager;
use tauri_plugin_autostart::MacosLauncher;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    devlog::init_tracing();

    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let state = AppState::new(app.handle())?;
            let show_onboard = !state.onboarding_complete();
            app.manage(state.clone());
            devlog::attach(app.handle());
            tray::attach(app)?;
            if show_onboard {
                tray::show_main(app.handle());
            }
            let spawned = state.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(err) = server::run(spawned).await {
                    tracing::error!("server stopped: {err}");
                    crate::devlog::record_crash(format!("server stopped: {err}"));
                }
            });
            Ok(())
        })
        .on_menu_event(tray::on_menu)
        .on_tray_icon_event(tray::on_tray)
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::session,
            commands::complete_onboarding,
            commands::permission_status,
            commands::request_accessibility,
            commands::request_automation,
            commands::cancel_uninstall,
            commands::uninstall,
            commands::is_debug,
            commands::dev_logs,
            commands::dev_logs_clear,
            commands::dev_client_error,
            commands::providers,
            commands::set_provider_enabled,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri");
}

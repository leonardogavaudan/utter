#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(target_os = "macos")]
mod activation;
mod commands;
mod delivery;
mod dictation;
mod error;
mod events;
mod overlay;
mod secrets;
mod shortcut;
mod state;
mod tray;
mod windows;

use tauri::{AppHandle, Manager, RunEvent};
use utter_core::settings::Settings;

use crate::state::AppState;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            windows::show_settings(app);
        }))
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(shortcut::handle)
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::list_providers,
            commands::list_models,
            commands::has_api_key,
            commands::set_api_key,
            commands::delete_api_key,
            commands::get_permissions,
            commands::request_accessibility,
            commands::request_microphone,
            commands::open_permission_settings,
            commands::get_shortcut_error,
            commands::get_phase,
            commands::get_last_dictation,
        ])
        .setup(|app| {
            let settings_path = app.path().app_config_dir()?.join("settings.json");
            let settings = Settings::load(&settings_path).unwrap_or_else(|error| {
                log::error!("using default settings: {error}");
                Settings::default()
            });
            let state = AppState::new(settings.clone(), settings_path)?;
            let mut needs_setup = !state.secrets.has_keys_for(&settings)
                || !utter_os::permissions::accessibility_granted();
            app.manage(state);
            if let Err(error) = shortcut::register(app.handle(), None, &settings.shortcut) {
                log::error!("could not register the dictation shortcut: {error}");
                *app.state::<AppState>().shortcut_error() = Some(error.to_string());
                needs_setup = true;
            }

            tray::create(app.handle())?;
            #[cfg(target_os = "macos")]
            activation::show_settings_when_activated(app.handle());
            overlay::prepare(app.handle());
            if needs_setup {
                windows::show_settings(app.handle());
            }
            Ok(())
        })
        .on_window_event(windows::handle_event)
        .build(tauri::generate_context!())
        .expect("the Tauri application could not be built")
        .run(handle_run_event);
}

#[cfg_attr(not(target_os = "macos"), expect(unused_variables))]
fn handle_run_event(app: &AppHandle, event: RunEvent) {
    match event {
        RunEvent::ExitRequested {
            api, code: None, ..
        } => api.prevent_exit(),
        #[cfg(target_os = "macos")]
        RunEvent::Reopen { .. } => windows::show_settings(app),
        _ => {}
    }
}

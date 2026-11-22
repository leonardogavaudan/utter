#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(target_os = "macos")]
mod activation;
mod cli;
mod commands;
mod delivery;
mod dictation;
mod error;
mod events;
mod overlay;
mod secrets;
mod settings_file;
mod shortcut;
mod state;
mod tray;
mod updates;
mod windows;

use tauri::{AppHandle, Manager, RunEvent};
use utter_core::settings::Settings;

use crate::state::AppState;

fn main() {
    let context = tauri::generate_context!();
    if let Some(code) = cli::run_if_requested(&context.config().identifier) {
        std::process::exit(code);
    }
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
        .plugin(tauri_plugin_updater::Builder::new().build())
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
            commands::get_api_key_source,
            commands::set_api_key,
            commands::delete_api_key,
            commands::list_microphones,
            commands::get_permissions,
            commands::request_accessibility,
            commands::request_microphone,
            commands::open_permission_settings,
            commands::get_shortcut_error,
            commands::get_settings_error,
            commands::get_pending_update,
            commands::restart_to_update,
            commands::get_phase,
            commands::get_last_dictation,
            commands::copy_last_dictation,
        ])
        .setup(|app| {
            let settings_path = app.path().app_config_dir()?.join("settings.json");
            let (settings, settings_error) = match Settings::load_and_validate(&settings_path) {
                Ok(settings) => (settings, None),
                Err(error) => {
                    log::error!("using default settings until the file is fixed: {error}");
                    (Settings::default(), Some(error.to_string()))
                }
            };
            let state = AppState::new(
                settings.clone(),
                settings_path,
                app.config().identifier.clone(),
            )?;
            let mut needs_setup = settings_error.is_some()
                || !state.secrets.has_keys_for(&settings)
                || !utter_os::permissions::accessibility_granted();
            *state.settings_error() = settings_error;
            app.manage(state);
            if let Err(error) = settings_file::watch(app.handle()) {
                log::warn!("Utter will not notice edits to its settings file: {error}");
            }
            if let Err(error) = shortcut::register(app.handle(), None, &settings.shortcut) {
                log::error!("could not register the dictation shortcut: {error}");
                *app.state::<AppState>().shortcut_error() = Some(error.to_string());
                needs_setup = true;
            }

            tray::create(app.handle())?;
            #[cfg(target_os = "macos")]
            activation::show_settings_when_activated(app.handle());
            overlay::prepare(app.handle());
            updates::check_periodically(app.handle());
            if needs_setup {
                windows::show_settings(app.handle());
            }
            Ok(())
        })
        .on_window_event(windows::handle_event)
        .build(context)
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

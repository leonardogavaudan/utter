use std::fs;
use std::time::Duration;

use notify_debouncer_mini::notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{DebounceEventResult, DebouncedEventBatch, Debouncer, new_debouncer};
use tauri::{AppHandle, Manager};
use utter_core::settings::Settings;

use crate::error::{AppError, AppResult};
use crate::events;
use crate::shortcut;
use crate::state::AppState;

const DEBOUNCE: Duration = Duration::from_millis(250);

pub struct SettingsWatcher {
    _debouncer: Debouncer<RecommendedWatcher>,
}

pub fn watch(app: &AppHandle) -> AppResult<()> {
    let path = app.state::<AppState>().settings_path.clone();
    let directory = path
        .parent()
        .ok_or_else(|| AppError::Internal(format!("{} has no folder", path.display())))?
        .to_path_buf();
    fs::create_dir_all(&directory).map_err(|error| AppError::Internal(error.to_string()))?;
    let file_name = path.file_name().map(ToOwned::to_owned);
    let handle = app.clone();
    let mut debouncer = new_debouncer(DEBOUNCE, move |result: DebounceEventResult| {
        let touches_settings = match result {
            Ok(DebouncedEventBatch::Events(changes)) => changes
                .iter()
                .any(|change| change.path.file_name() == file_name.as_deref()),
            Ok(_) => true,
            Err(error) => {
                log::warn!("the settings watcher failed: {error}");
                false
            }
        };
        if touches_settings {
            let app = handle.clone();
            if let Err(error) = handle.run_on_main_thread(move || reload(&app)) {
                log::warn!("could not reload the settings: {error}");
            }
        }
    })
    .map_err(|error| AppError::Internal(error.to_string()))?;
    debouncer
        .watcher()
        .watch(&directory, RecursiveMode::NonRecursive)
        .map_err(|error| AppError::Internal(error.to_string()))?;
    app.manage(SettingsWatcher {
        _debouncer: debouncer,
    });
    Ok(())
}

pub fn clear_error(app: &AppHandle) {
    record_error(app, None);
}

pub fn record_error(app: &AppHandle, error: Option<String>) {
    let state = app.state::<AppState>();
    let mut current = state.settings_error();
    if *current == error {
        return;
    }
    current.clone_from(&error);
    drop(current);
    events::settings_error(app, error);
}

fn reload(app: &AppHandle) {
    let state = app.state::<AppState>();
    match Settings::load_and_validate(&state.settings_path) {
        Ok(next) => {
            clear_error(app);
            if *state.settings() != next {
                log::info!("applying settings changed outside the settings window");
                apply(app, next);
            }
        }
        Err(error) => {
            log::warn!("keeping the last good settings: {error}");
            record_error(app, Some(error.to_string()));
        }
    }
}

fn apply(app: &AppHandle, next: Settings) {
    let state = app.state::<AppState>();
    let previous_shortcut = state.settings().shortcut.clone();
    if next.shortcut != previous_shortcut {
        match shortcut::register(app, Some(&previous_shortcut), &next.shortcut) {
            Ok(()) => *state.shortcut_error() = None,
            Err(error) => {
                log::warn!("could not register the new dictation shortcut: {error}");
                *state.shortcut_error() = Some(error.to_string());
            }
        }
    }
    *state.settings_mut() = next.clone();
    events::settings_changed(app, &next);
}

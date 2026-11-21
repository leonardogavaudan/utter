use std::str::FromStr;
use std::sync::{Mutex, PoisonError};

use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{
    Code, GlobalShortcutExt, Shortcut, ShortcutEvent, ShortcutState,
};
use utter_core::settings::TriggerMode;

use crate::dictation;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

static ACTIVE_CANCEL_SHORTCUTS: Mutex<Vec<Shortcut>> = Mutex::new(Vec::new());

pub fn parse(shortcut: &str) -> AppResult<Shortcut> {
    Shortcut::from_str(shortcut).map_err(|error| AppError::Shortcut {
        shortcut: shortcut.to_owned(),
        reason: error.to_string(),
    })
}

pub fn register(app: &AppHandle, previous: Option<&str>, next: &str) -> AppResult<()> {
    let next_shortcut = parse(next)?;
    if next_shortcut.key == Code::Escape {
        return Err(AppError::Shortcut {
            shortcut: next.to_owned(),
            reason: "Escape is reserved for cancelling a dictation".to_owned(),
        });
    }
    let shortcuts = app.global_shortcut();
    let previous_shortcut = previous.and_then(|previous| parse(previous).ok());
    if previous_shortcut == Some(next_shortcut) && shortcuts.is_registered(next_shortcut) {
        return Ok(());
    }
    if let Some(previous_shortcut) = previous_shortcut {
        let _ = shortcuts.unregister(previous_shortcut);
    }
    shortcuts.register(next_shortcut).map_err(|error| {
        if let Some(previous_shortcut) = previous_shortcut {
            let _ = shortcuts.register(previous_shortcut);
        }
        AppError::Shortcut {
            shortcut: next.to_owned(),
            reason: error.to_string(),
        }
    })
}

pub fn enable_cancel(app: &AppHandle) {
    let mut cancel = vec![Shortcut::new(None, Code::Escape)];
    let dictation_shortcut = app.state::<AppState>().settings().shortcut.clone();
    if let Ok(dictation_shortcut) = parse(&dictation_shortcut)
        && !dictation_shortcut.mods.is_empty()
    {
        cancel.push(Shortcut::new(Some(dictation_shortcut.mods), Code::Escape));
    }
    let mut active = ACTIVE_CANCEL_SHORTCUTS
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    for shortcut in cancel {
        match app.global_shortcut().register(shortcut) {
            Ok(()) => active.push(shortcut),
            Err(error) => log::warn!("{shortcut} will not cancel this dictation: {error}"),
        }
    }
}

pub fn disable_cancel(app: &AppHandle) {
    let mut active = ACTIVE_CANCEL_SHORTCUTS
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    for shortcut in active.drain(..) {
        let _ = app.global_shortcut().unregister(shortcut);
    }
}

pub fn handle(app: &AppHandle, shortcut: &Shortcut, event: ShortcutEvent) {
    if shortcut.key == Code::Escape {
        if event.state() == ShortcutState::Pressed {
            dictation::cancel(app);
        }
        return;
    }
    let trigger = app.state::<AppState>().settings().trigger;
    match (trigger, event.state()) {
        (TriggerMode::Hold, ShortcutState::Pressed) => dictation::start(app),
        (TriggerMode::Hold, ShortcutState::Released) => dictation::finish(app),
        (TriggerMode::Toggle, ShortcutState::Pressed) => dictation::toggle(app),
        (TriggerMode::Toggle, ShortcutState::Released) => {}
    }
}

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use ts_rs::TS;
use utter_core::pipeline::Dictation;

use crate::{overlay, tray};

pub const PHASE_EVENT: &str = "dictation-phase";
pub const NOTICE_EVENT: &str = "dictation-notice";
pub const FINISHED_EVENT: &str = "dictation-finished";
pub const UPDATE_READY_EVENT: &str = "update-ready";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Phase {
    Idle,
    Recording,
    Processing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum CopyReason {
    AccessibilityMissing,
    AppChanged,
    PasteFailed,
    NoTextField,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase", tag = "kind")]
#[ts(export)]
pub enum Notice {
    NoSpeech,
    Copied { reason: CopyReason },
    CleanupSkipped { reason: String },
    Failed { message: String },
}

pub fn phase_changed(app: &AppHandle, phase: Phase) {
    tray::show_phase(app, phase);
    match phase {
        Phase::Recording | Phase::Processing => overlay::show(app),
        Phase::Idle => {}
    }
    emit(app, PHASE_EVENT, phase);
}

pub fn finished(app: &AppHandle, notice: Option<Notice>) {
    phase_changed(app, Phase::Idle);
    match notice {
        Some(notice) => show_notice(app, notice),
        None => overlay::hide(app),
    }
}

pub fn show_notice(app: &AppHandle, notice: Notice) {
    overlay::show(app);
    emit(app, NOTICE_EVENT, notice);
    overlay::hide_later(app);
}

pub fn update_ready(app: &AppHandle, version: &str) {
    emit(app, UPDATE_READY_EVENT, version.to_owned());
}

pub fn dictated(app: &AppHandle, dictation: &Dictation) {
    emit(app, FINISHED_EVENT, dictation);
}

fn emit(app: &AppHandle, event: &str, payload: impl Serialize + Clone) {
    if let Err(error) = app.emit(event, payload) {
        log::warn!("could not emit {event}: {error}");
    }
}

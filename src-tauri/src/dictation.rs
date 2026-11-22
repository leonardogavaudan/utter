use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use tauri::{AppHandle, Manager};
use utter_core::audio::MAX_RECORDING_DURATION;
use utter_core::context::{ContextMode, TextContext};
use utter_core::pipeline;
use utter_os::recorder::ActiveRecording;

use crate::delivery::{self, Delivery};
use crate::error::{AppError, AppResult};
use crate::events::{self, Notice, Phase};
use crate::shortcut;
use crate::state::AppState;

const SILENT_MICROPHONE_AFTER: Duration = Duration::from_millis(500);
const CONTEXT_DEADLINE: Duration = Duration::from_millis(500);

static NEXT_RECORDING_ID: AtomicU64 = AtomicU64::new(1);

pub enum Session {
    Idle,
    Recording(Recording),
    Processing,
}

pub struct Recording {
    id: u64,
    audio: ActiveRecording,
    context: mpsc::Receiver<TextContext>,
    target_pid: Option<i32>,
}

impl Session {
    pub fn phase(&self) -> Phase {
        match self {
            Session::Idle => Phase::Idle,
            Session::Recording(_) => Phase::Recording,
            Session::Processing => Phase::Processing,
        }
    }
}

pub fn start(app: &AppHandle) {
    let state = app.state::<AppState>();
    let mut session = state.session();
    if !matches!(*session, Session::Idle) {
        return;
    }
    let target_pid = utter_os::context::focused_app_pid();
    let (context_mode, microphone) = {
        let settings = state.settings();
        (settings.context, settings.microphone.clone())
    };
    let context = capture_context_in_background(context_mode);
    match ActiveRecording::start(microphone) {
        Ok(audio) => {
            let id = NEXT_RECORDING_ID.fetch_add(1, Ordering::Relaxed);
            *session = Session::Recording(Recording {
                id,
                audio,
                context,
                target_pid,
            });
            drop(session);
            shortcut::enable_cancel(app);
            events::phase_changed(app, Phase::Recording);
            stop_after_max_duration(app, id);
        }
        Err(error) => {
            drop(session);
            log::error!("could not start recording: {error}");
            events::show_notice(
                app,
                Notice::Failed {
                    message: AppError::from(error).to_string(),
                },
            );
        }
    }
}

pub fn finish(app: &AppHandle) {
    finish_recording(app, None);
}

pub fn cancel(app: &AppHandle) {
    let Some(recording) = take_recording(app, None, Session::Idle) else {
        return;
    };
    shortcut::disable_cancel(app);
    thread::spawn(move || drop(recording.audio.stop()));
    events::finished(app, None);
}

pub fn toggle(app: &AppHandle) {
    let phase = app.state::<AppState>().session().phase();
    match phase {
        Phase::Idle => start(app),
        Phase::Recording => finish(app),
        Phase::Processing => {}
    }
}

fn finish_recording(app: &AppHandle, only_id: Option<u64>) {
    let Some(recording) = take_recording(app, only_id, Session::Processing) else {
        return;
    };
    shortcut::disable_cancel(app);
    events::phase_changed(app, Phase::Processing);
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let work = tauri::async_runtime::spawn(transcribe_and_deliver(app.clone(), recording));
        let notice = match work.await {
            Ok(Ok(notice)) => notice,
            Ok(Err(error)) => {
                log::warn!("dictation failed: {error}");
                Some(Notice::Failed {
                    message: error.to_string(),
                })
            }
            Err(error) => {
                log::error!("the dictation task stopped unexpectedly: {error}");
                Some(Notice::Failed {
                    message: "Utter hit an internal error; please try again".to_owned(),
                })
            }
        };
        events::finished(&app, notice);
        *app.state::<AppState>().session() = Session::Idle;
    });
}

fn stop_after_max_duration(app: &AppHandle, id: u64) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(MAX_RECORDING_DURATION).await;
        finish_recording(&app, Some(id));
    });
}

fn take_recording(app: &AppHandle, only_id: Option<u64>, next: Session) -> Option<Recording> {
    let state = app.state::<AppState>();
    let mut session = state.session();
    let matches = match &*session {
        Session::Recording(recording) => only_id.is_none_or(|id| id == recording.id),
        _ => false,
    };
    if !matches {
        return None;
    }
    match std::mem::replace(&mut *session, next) {
        Session::Recording(recording) => Some(recording),
        _ => None,
    }
}

fn capture_context_in_background(mode: ContextMode) -> mpsc::Receiver<TextContext> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let _ = sender.send(utter_os::context::capture(mode));
    });
    receiver
}

async fn transcribe_and_deliver(app: AppHandle, recording: Recording) -> AppResult<Option<Notice>> {
    let Recording {
        audio,
        context,
        target_pid,
        ..
    } = recording;
    let (recording, context) = tauri::async_runtime::spawn_blocking(move || {
        let recording = audio.stop();
        let context = context.recv_timeout(CONTEXT_DEADLINE).unwrap_or_else(|_| {
            log::warn!("continuing without context: the app did not answer in time");
            TextContext::default()
        });
        (recording, context)
    })
    .await
    .map_err(|error| AppError::Internal(error.to_string()))?;
    if recording.duration() >= SILENT_MICROPHONE_AFTER && recording.peak() == 0.0 {
        return Err(AppError::SilentMicrophone);
    }
    if !recording.has_speech() {
        log::info!(
            "no speech heard in {:.1} s of audio (peak level {:.3})",
            recording.duration().as_secs_f64(),
            recording.peak()
        );
        return Ok(Some(Notice::NoSpeech));
    }

    let state = app.state::<AppState>();
    let settings = state.settings().clone();
    let keys = state.secrets.keys_for(&settings);
    let dictation = pipeline::run(&state.http, &settings, &keys, &recording, &context).await?;
    log::info!(
        "dictated {} characters from {:.1} s of audio (speech-to-text {} ms, cleanup {:?} ms)",
        dictation.text.chars().count(),
        dictation.audio_seconds,
        dictation.transcription_ms,
        dictation.cleanup_ms,
    );
    *state.last_dictation() = Some(dictation.clone());
    events::dictated(&app, &dictation);

    if dictation.text.trim().is_empty() {
        log::info!("the speech-to-text model returned no words");
        return Ok(Some(Notice::NoSpeech));
    }
    let delivery = delivery::deliver(&app, dictation.text, target_pid).await?;
    Ok(match (delivery, dictation.cleanup_error) {
        (Delivery::Copied(reason), _) => Some(Notice::Copied { reason }),
        (Delivery::Pasted, Some(reason)) => Some(Notice::CleanupSkipped { reason }),
        (Delivery::Pasted, None) => None,
    })
}

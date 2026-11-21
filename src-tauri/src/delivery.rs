use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use tauri::AppHandle;
use utter_os::insert::{self, StagedText};
use utter_os::{context, permissions};

use crate::error::{AppError, AppResult};
use crate::events::CopyReason;

const CLIPBOARD_SETTLE: Duration = Duration::from_millis(40);
const RESTORE_CLIPBOARD_AFTER: Duration = Duration::from_millis(600);

pub enum Delivery {
    Pasted,
    Copied(CopyReason),
}

pub async fn deliver(
    app: &AppHandle,
    text: String,
    target_pid: Option<i32>,
) -> AppResult<Delivery> {
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || deliver_blocking(&app, &text, target_pid))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?
}

fn deliver_blocking(app: &AppHandle, text: &str, target_pid: Option<i32>) -> AppResult<Delivery> {
    if !permissions::accessibility_granted() {
        insert::copy_to_clipboard(text)?;
        return Ok(Delivery::Copied(CopyReason::AccessibilityMissing));
    }
    if target_pid.is_some() && context::focused_app_pid() != target_pid {
        insert::copy_to_clipboard(text)?;
        return Ok(Delivery::Copied(CopyReason::AppChanged));
    }
    let staged = insert::stage_text(text)?;
    thread::sleep(CLIPBOARD_SETTLE);
    if let Err(error) = send_paste_keystroke_on_main_thread(app) {
        log::warn!("the paste keystroke failed, leaving the text on the clipboard: {error}");
        return Ok(Delivery::Copied(CopyReason::PasteFailed));
    }
    restore_clipboard_later(staged);
    Ok(Delivery::Pasted)
}

fn send_paste_keystroke_on_main_thread(app: &AppHandle) -> AppResult<()> {
    let (sender, receiver) = mpsc::channel();
    app.run_on_main_thread(move || {
        let _ = sender.send(insert::send_paste_keystroke());
    })?;
    receiver
        .recv()
        .map_err(|error| AppError::Internal(error.to_string()))??;
    Ok(())
}

fn restore_clipboard_later(staged: StagedText) {
    thread::spawn(move || {
        thread::sleep(RESTORE_CLIPBOARD_AFTER);
        if let Err(error) = staged.restore_clipboard() {
            log::warn!("could not restore the previous clipboard text: {error}");
        }
    });
}

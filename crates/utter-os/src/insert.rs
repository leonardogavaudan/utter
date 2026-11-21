use enigo::{Direction, Enigo, Key, Keyboard, Settings};

#[cfg(target_os = "macos")]
use arboard::SetExtApple;
#[cfg(target_os = "linux")]
use arboard::SetExtLinux;
#[cfg(target_os = "windows")]
use arboard::SetExtWindows;

#[derive(Debug, thiserror::Error)]
pub enum InsertError {
    #[error("clipboard error: {0}")]
    Clipboard(#[from] arboard::Error),
    #[error("could not send the paste shortcut: {0}")]
    Keyboard(String),
}

pub struct StagedText {
    clipboard: arboard::Clipboard,
    previous: Option<String>,
    text: String,
}

pub fn stage_text(text: &str) -> Result<StagedText, InsertError> {
    let mut clipboard = arboard::Clipboard::new()?;
    let previous = clipboard.get_text().ok();
    clipboard
        .set()
        .exclude_from_history()
        .text(text.to_owned())?;
    Ok(StagedText {
        clipboard,
        previous,
        text: text.to_owned(),
    })
}

pub fn copy_to_clipboard(text: &str) -> Result<(), InsertError> {
    arboard::Clipboard::new()?.set_text(text.to_owned())?;
    Ok(())
}

impl StagedText {
    pub fn restore_clipboard(mut self) -> Result<(), InsertError> {
        let Some(previous) = self.previous.take() else {
            return Ok(());
        };
        if self.clipboard.get_text().ok().as_deref() == Some(self.text.as_str()) {
            self.clipboard.set_text(previous)?;
        }
        Ok(())
    }
}

#[cfg(target_os = "macos")]
const PASTE_SHORTCUT: (Key, Key) = (Key::Meta, Key::Other(9));
#[cfg(target_os = "windows")]
const PASTE_SHORTCUT: (Key, Key) = (Key::Control, Key::Other(0x56));
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
const PASTE_SHORTCUT: (Key, Key) = (Key::Control, Key::Unicode('v'));

pub fn send_paste_keystroke() -> Result<(), InsertError> {
    let keyboard_error = |error: enigo::InputError| InsertError::Keyboard(error.to_string());
    let mut enigo = Enigo::new(&Settings::default())
        .map_err(|error| InsertError::Keyboard(error.to_string()))?;
    let (modifier, key) = PASTE_SHORTCUT;
    enigo
        .key(modifier, Direction::Press)
        .map_err(keyboard_error)?;
    let pasted = enigo.key(key, Direction::Click).map_err(keyboard_error);
    enigo
        .key(modifier, Direction::Release)
        .map_err(keyboard_error)?;
    pasted
}

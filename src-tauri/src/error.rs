use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error(transparent)]
    Core(#[from] utter_core::Error),
    #[error(transparent)]
    Insert(#[from] utter_os::insert::InsertError),
    #[error(transparent)]
    Recorder(#[from] utter_os::recorder::RecorderError),
    #[error("keychain error: {0}")]
    Keychain(#[from] keyring::Error),
    #[error(transparent)]
    Tauri(#[from] tauri::Error),
    #[error("the shortcut \"{shortcut}\" could not be used: {reason}")]
    Shortcut { shortcut: String, reason: String },
    #[error(
        "the microphone recorded only silence; check that Utter has microphone access in System Settings"
    )]
    SilentMicrophone,
    #[error("{0}")]
    Internal(String),
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;

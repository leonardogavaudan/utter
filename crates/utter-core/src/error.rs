pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("the provider returned HTTP {status}: {message}")]
    Api { status: u16, message: String },
    #[error("the provider sent an unexpected response: {0}")]
    InvalidResponse(String),
    #[error("the model declined to process this dictation")]
    Refused,
    #[error("no API key is saved for {0}")]
    MissingApiKey(String),
    #[error("audio could not be encoded: {0}")]
    Audio(String),
    #[error("settings error: {0}")]
    Settings(String),
}

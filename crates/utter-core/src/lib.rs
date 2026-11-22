pub mod audio;
pub mod cleanup;
pub mod context;
mod error;
mod http;
pub mod microphone;
pub mod pipeline;
pub mod providers;
pub mod screen;
pub mod settings;
pub mod transcription;

pub use error::{Error, Result};

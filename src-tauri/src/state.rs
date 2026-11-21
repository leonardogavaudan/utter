use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::time::Duration;

use utter_core::pipeline::Dictation;
use utter_core::settings::Settings;

use crate::dictation::Session;
use crate::error::AppResult;
use crate::secrets::Secrets;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const KEYCHAIN_SERVICE: &str = "dev.utter.desktop";

pub struct AppState {
    pub settings: RwLock<Settings>,
    pub settings_path: PathBuf,
    pub http: reqwest::Client,
    pub secrets: Secrets,
    pub session: Mutex<Session>,
    pub last_dictation: Mutex<Option<Dictation>>,
    pub shortcut_error: Mutex<Option<String>>,
    pub pending_update: Mutex<Option<String>>,
}

impl AppState {
    pub fn new(settings: Settings, settings_path: PathBuf) -> AppResult<AppState> {
        let http = reqwest::Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .build()
            .map_err(utter_core::Error::from)?;
        Ok(AppState {
            settings: RwLock::new(settings),
            settings_path,
            http,
            secrets: Secrets::new(KEYCHAIN_SERVICE),
            session: Mutex::new(Session::Idle),
            last_dictation: Mutex::new(None),
            shortcut_error: Mutex::new(None),
            pending_update: Mutex::new(None),
        })
    }

    pub fn settings(&self) -> RwLockReadGuard<'_, Settings> {
        self.settings.read().unwrap_or_else(PoisonError::into_inner)
    }

    pub fn settings_mut(&self) -> RwLockWriteGuard<'_, Settings> {
        self.settings
            .write()
            .unwrap_or_else(PoisonError::into_inner)
    }

    pub fn session(&self) -> MutexGuard<'_, Session> {
        self.session.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub fn last_dictation(&self) -> MutexGuard<'_, Option<Dictation>> {
        self.last_dictation
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    pub fn shortcut_error(&self) -> MutexGuard<'_, Option<String>> {
        self.shortcut_error
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    pub fn pending_update(&self) -> MutexGuard<'_, Option<String>> {
        self.pending_update
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};

use utter_core::pipeline::ApiKeys;
use utter_core::providers::{ProviderId, Role};
use utter_core::settings::Settings;

use crate::error::AppResult;

pub struct Secrets {
    service: &'static str,
    slot_to_key: Mutex<HashMap<String, Option<String>>>,
}

impl Secrets {
    pub fn new(service: &'static str) -> Secrets {
        Secrets {
            service,
            slot_to_key: Mutex::new(HashMap::new()),
        }
    }

    pub fn get(&self, provider: ProviderId, role: Role) -> Option<String> {
        let slot = provider.api_key_slot(role);
        let mut slot_to_key = self
            .slot_to_key
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if let Some(key) = slot_to_key.get(&slot) {
            return key.clone();
        }
        let key =
            match keyring::Entry::new(self.service, &slot).and_then(|entry| entry.get_password()) {
                Ok(key) => Some(key),
                Err(keyring::Error::NoEntry) => None,
                Err(error) => {
                    log::warn!("could not read the {slot} API key from the keychain: {error}");
                    return None;
                }
            };
        slot_to_key.insert(slot, key.clone());
        key
    }

    pub fn set(&self, provider: ProviderId, role: Role, key: &str) -> AppResult<()> {
        let slot = provider.api_key_slot(role);
        keyring::Entry::new(self.service, &slot)?.set_password(key)?;
        self.remember(slot, Some(key.to_owned()));
        Ok(())
    }

    pub fn delete(&self, provider: ProviderId, role: Role) -> AppResult<()> {
        let slot = provider.api_key_slot(role);
        match keyring::Entry::new(self.service, &slot)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => {}
            Err(error) => return Err(error.into()),
        }
        self.remember(slot, None);
        Ok(())
    }

    pub fn keys_for(&self, settings: &Settings) -> ApiKeys {
        ApiKeys {
            transcription: self.get(settings.transcription.provider, Role::Transcription),
            cleanup: settings
                .cleanup
                .enabled
                .then(|| self.get(settings.cleanup.provider, Role::Cleanup))
                .flatten(),
        }
    }

    pub fn has_keys_for(&self, settings: &Settings) -> bool {
        let provider = settings.transcription.provider;
        !provider.preset().requires_api_key || self.get(provider, Role::Transcription).is_some()
    }

    fn remember(&self, slot: String, key: Option<String>) {
        self.slot_to_key
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(slot, key);
    }
}

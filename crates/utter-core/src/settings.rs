use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::context::ContextMode;
use crate::providers::{ProviderId, Role};
use crate::{Error, Result};

pub const MAX_VOCABULARY_TERMS: usize = 200;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum TriggerMode {
    #[default]
    Hold,
    Toggle,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct TranscriptionSettings {
    pub provider: ProviderId,
    pub base_url: String,
    pub model: String,
    pub language: Option<String>,
}

impl Default for TranscriptionSettings {
    fn default() -> Self {
        let preset = ProviderId::Groq.preset();
        TranscriptionSettings {
            provider: preset.id,
            base_url: preset.base_url.to_owned(),
            model: preset.transcription_model.unwrap_or_default().to_owned(),
            language: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct CleanupSettings {
    pub enabled: bool,
    pub provider: ProviderId,
    pub base_url: String,
    pub model: String,
    pub instructions: String,
}

impl Default for CleanupSettings {
    fn default() -> Self {
        let preset = ProviderId::Groq.preset();
        CleanupSettings {
            enabled: true,
            provider: preset.id,
            base_url: preset.base_url.to_owned(),
            model: preset.cleanup_model.unwrap_or_default().to_owned(),
            instructions: String::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct Settings {
    pub shortcut: String,
    pub trigger: TriggerMode,
    pub context: ContextMode,
    pub transcription: TranscriptionSettings,
    pub cleanup: CleanupSettings,
    pub vocabulary: Vec<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            shortcut: "Alt+Space".to_owned(),
            trigger: TriggerMode::default(),
            context: ContextMode::default(),
            transcription: TranscriptionSettings::default(),
            cleanup: CleanupSettings::default(),
            vocabulary: Vec::new(),
        }
    }
}

impl Settings {
    pub fn load(path: &Path) -> Result<Settings> {
        match fs::read_to_string(path) {
            Ok(json) => serde_json::from_str(&json).map_err(|error| {
                Error::Settings(format!("{} is not valid: {error}", path.display()))
            }),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(Settings::default()),
            Err(error) => Err(Error::Settings(format!(
                "could not read {}: {error}",
                path.display()
            ))),
        }
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|error| Error::Settings(error.to_string()))?;
        if let Some(directory) = path.parent() {
            fs::create_dir_all(directory).map_err(|error| {
                Error::Settings(format!("could not create {}: {error}", directory.display()))
            })?;
        }
        let staging = path.with_extension("json.tmp");
        fs::write(&staging, json)
            .and_then(|()| fs::rename(&staging, path))
            .map_err(|error| {
                Error::Settings(format!("could not write {}: {error}", path.display()))
            })
    }

    pub fn normalized(mut self) -> Settings {
        self.shortcut = self.shortcut.trim().to_owned();
        self.transcription.base_url = self
            .transcription
            .base_url
            .trim()
            .trim_end_matches('/')
            .to_owned();
        self.transcription.model = self.transcription.model.trim().to_owned();
        self.transcription.language = self
            .transcription
            .language
            .map(|language| language.trim().to_lowercase())
            .filter(|language| !language.is_empty());
        self.cleanup.base_url = self
            .cleanup
            .base_url
            .trim()
            .trim_end_matches('/')
            .to_owned();
        self.cleanup.model = self.cleanup.model.trim().to_owned();
        self.cleanup.instructions = self.cleanup.instructions.trim().to_owned();
        let mut seen = std::collections::HashSet::new();
        self.vocabulary = self
            .vocabulary
            .into_iter()
            .map(|term| term.trim().to_owned())
            .filter(|term| !term.is_empty() && seen.insert(term.to_lowercase()))
            .take(MAX_VOCABULARY_TERMS)
            .collect();
        self
    }

    pub fn validate(&self) -> Result<()> {
        if self.shortcut.is_empty() {
            return Err(Error::Settings("choose a keyboard shortcut".to_owned()));
        }
        if !self.transcription.provider.supports(Role::Transcription) {
            return Err(Error::Settings(format!(
                "{} does not offer speech-to-text",
                self.transcription.provider.preset().label
            )));
        }
        validate_endpoint(
            "speech-to-text",
            &self.transcription.base_url,
            &self.transcription.model,
        )?;
        if self.cleanup.enabled {
            validate_endpoint("cleanup", &self.cleanup.base_url, &self.cleanup.model)?;
        }
        Ok(())
    }
}

fn validate_endpoint(name: &str, base_url: &str, model: &str) -> Result<()> {
    if !(base_url.starts_with("https://") || base_url.starts_with("http://")) {
        return Err(Error::Settings(format!(
            "the {name} base URL must start with https:// or http://"
        )));
    }
    if model.is_empty() {
        return Err(Error::Settings(format!("choose a {name} model")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_valid() {
        Settings::default()
            .validate()
            .expect("default settings are valid");
    }

    #[test]
    fn missing_file_loads_defaults() {
        let directory = tempfile::tempdir().expect("temp dir");
        let settings = Settings::load(&directory.path().join("settings.json")).expect("loads");
        assert_eq!(settings, Settings::default());
    }

    #[test]
    fn saves_and_loads_round_trip() {
        let directory = tempfile::tempdir().expect("temp dir");
        let path = directory.path().join("nested").join("settings.json");
        let settings = Settings {
            trigger: TriggerMode::Toggle,
            vocabulary: vec!["Tauri".into()],
            ..Settings::default()
        };
        settings.save(&path).expect("saves");
        assert_eq!(Settings::load(&path).expect("loads"), settings);
        assert!(!path.with_extension("json.tmp").exists());
    }

    #[test]
    fn partial_files_fill_in_defaults_and_ignore_unknown_fields() {
        let json = r#"{"trigger":"toggle","cleanup":{"enabled":false},"retired":1}"#;
        let settings: Settings = serde_json::from_str(json).expect("parses");
        assert_eq!(settings.trigger, TriggerMode::Toggle);
        assert!(!settings.cleanup.enabled);
        assert_eq!(settings.cleanup.model, CleanupSettings::default().model);
        assert_eq!(settings.shortcut, Settings::default().shortcut);
    }

    #[test]
    fn corrupt_files_are_reported_instead_of_silently_replaced() {
        let directory = tempfile::tempdir().expect("temp dir");
        let path = directory.path().join("settings.json");
        fs::write(&path, "{not json").expect("writes");
        assert!(matches!(Settings::load(&path), Err(Error::Settings(_))));
    }

    #[test]
    fn normalizing_trims_values_and_dedupes_vocabulary() {
        let mut settings = Settings::default();
        settings.transcription.base_url = " https://api.groq.com/openai/v1/ ".into();
        settings.transcription.language = Some(" FR ".into());
        settings.vocabulary = vec![
            " Tauri ".into(),
            "tauri".into(),
            "".into(),
            "Granola".into(),
        ];
        let settings = settings.normalized();
        assert_eq!(
            settings.transcription.base_url,
            "https://api.groq.com/openai/v1"
        );
        assert_eq!(settings.transcription.language.as_deref(), Some("fr"));
        assert_eq!(settings.vocabulary, vec!["Tauri", "Granola"]);
    }

    #[test]
    fn rejects_providers_and_urls_that_cannot_work() {
        let mut settings = Settings::default();
        settings.transcription.provider = ProviderId::Anthropic;
        assert!(settings.validate().is_err());

        let mut settings = Settings::default();
        settings.cleanup.base_url = "api.groq.com".into();
        assert!(settings.validate().is_err());

        settings.cleanup.enabled = false;
        settings
            .validate()
            .expect("disabled cleanup is not validated");
    }
}

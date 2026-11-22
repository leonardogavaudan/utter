use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;
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
    pub api_key_variable: Option<String>,
}

impl TranscriptionSettings {
    pub fn for_provider(provider: ProviderId) -> TranscriptionSettings {
        let preset = provider.preset();
        TranscriptionSettings {
            provider,
            base_url: preset.base_url.to_owned(),
            model: preset.transcription_model.unwrap_or_default().to_owned(),
            language: None,
            api_key_variable: None,
        }
    }

    pub fn switch_provider(&mut self, provider: ProviderId) {
        let defaults = TranscriptionSettings::for_provider(provider);
        self.provider = defaults.provider;
        self.base_url = defaults.base_url;
        self.model = defaults.model;
        self.api_key_variable = None;
    }

    pub fn resolve_api_key_variable(&self) -> Option<String> {
        self.provider
            .resolve_api_key_variable(self.api_key_variable.as_deref())
    }
}

impl Default for TranscriptionSettings {
    fn default() -> Self {
        TranscriptionSettings::for_provider(ProviderId::Groq)
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
    pub api_key_variable: Option<String>,
}

impl CleanupSettings {
    pub fn switch_provider(&mut self, provider: ProviderId) {
        let preset = provider.preset();
        self.provider = provider;
        self.base_url = preset.base_url.to_owned();
        self.model = preset.cleanup_model.unwrap_or_default().to_owned();
        self.api_key_variable = None;
    }

    pub fn resolve_api_key_variable(&self) -> Option<String> {
        self.provider
            .resolve_api_key_variable(self.api_key_variable.as_deref())
    }
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
            api_key_variable: None,
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
    pub microphone: Option<String>,
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
            microphone: None,
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

    pub fn load_and_validate(path: &Path) -> Result<Settings> {
        let settings = Settings::load(path)?.normalized();
        settings.validate()?;
        Ok(settings)
    }

    pub fn value_at(&self, path: &str) -> Result<Value> {
        let root =
            serde_json::to_value(self).map_err(|error| Error::Settings(error.to_string()))?;
        Ok(find_setting(&root, path)?.clone())
    }

    pub fn with_value(&self, path: &str, raw: &str) -> Result<Settings> {
        let mut root =
            serde_json::to_value(self).map_err(|error| Error::Settings(error.to_string()))?;
        find_setting(&root, path)?;
        set_setting(&mut root, path, parse_setting_value(raw));
        let mut next: Settings = serde_json::from_value(root)
            .map_err(|error| Error::Settings(format!("{path}: {error}")))?;
        if path == "transcription.provider"
            && next.transcription.provider != self.transcription.provider
        {
            next.transcription
                .switch_provider(next.transcription.provider);
        }
        if path == "cleanup.provider" && next.cleanup.provider != self.cleanup.provider {
            next.cleanup.switch_provider(next.cleanup.provider);
        }
        let next = next.normalized();
        next.validate()?;
        Ok(next)
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
        self.transcription.api_key_variable =
            normalize_variable(self.transcription.api_key_variable);
        self.cleanup.api_key_variable = normalize_variable(self.cleanup.api_key_variable);
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
        validate_variable(
            "speech-to-text",
            self.transcription.api_key_variable.as_deref(),
        )?;
        if self.cleanup.enabled {
            if !self.cleanup.provider.supports(Role::Cleanup) {
                return Err(Error::Settings(format!(
                    "{} does not offer cleanup",
                    self.cleanup.provider.preset().label
                )));
            }
            validate_endpoint("cleanup", &self.cleanup.base_url, &self.cleanup.model)?;
            validate_variable("cleanup", self.cleanup.api_key_variable.as_deref())?;
        }
        Ok(())
    }
}

fn normalize_variable(variable: Option<String>) -> Option<String> {
    variable
        .map(|variable| variable.trim().to_owned())
        .filter(|variable| !variable.is_empty())
}

fn validate_variable(name: &str, variable: Option<&str>) -> Result<()> {
    let Some(variable) = variable else {
        return Ok(());
    };
    let mut characters = variable.chars();
    let starts_well = characters
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_');
    if !starts_well || !characters.all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(Error::Settings(format!(
            "the {name} API key variable `{variable}` must use only letters, digits and underscores, and not start with a digit"
        )));
    }
    Ok(())
}

fn find_setting<'a>(root: &'a Value, path: &str) -> Result<&'a Value> {
    let mut current = root;
    let mut walked = Vec::new();
    for segment in path.split('.') {
        let Value::Object(fields) = current else {
            return Err(Error::Settings(format!(
                "`{}` has no settings inside it",
                walked.join(".")
            )));
        };
        let Some(next) = fields.get(segment) else {
            let parent = if walked.is_empty() {
                "Settings".to_owned()
            } else {
                format!("`{}`", walked.join("."))
            };
            let names: Vec<&str> = fields.keys().map(String::as_str).collect();
            return Err(Error::Settings(format!(
                "unknown setting `{path}`. {parent} has: {}",
                names.join(", ")
            )));
        };
        walked.push(segment);
        current = next;
    }
    Ok(current)
}

fn set_setting(root: &mut Value, path: &str, value: Value) {
    let mut current = root;
    for segment in path.split('.') {
        current = &mut current[segment];
    }
    *current = value;
}

fn parse_setting_value(raw: &str) -> Value {
    serde_json::from_str(raw).unwrap_or_else(|_| Value::String(raw.to_owned()))
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
        let mut cleanup_without_chat = Settings::default();
        cleanup_without_chat.cleanup.provider = ProviderId::ElevenLabs;
        assert!(cleanup_without_chat.validate().is_err());
        let elevenlabs = Settings {
            transcription: TranscriptionSettings::for_provider(ProviderId::ElevenLabs),
            ..Settings::default()
        };
        elevenlabs.validate().expect("ElevenLabs transcribes");

        let mut settings = Settings::default();
        settings.cleanup.base_url = "api.groq.com".into();
        assert!(settings.validate().is_err());

        settings.cleanup.enabled = false;
        settings
            .validate()
            .expect("disabled cleanup is not validated");
    }

    #[test]
    fn sets_values_by_name_and_parses_json_or_plain_text() {
        let settings = Settings::default();
        let toggled = settings.with_value("trigger", "toggle").expect("sets");
        assert_eq!(toggled.trigger, TriggerMode::Toggle);
        let disabled = settings
            .with_value("cleanup.enabled", "false")
            .expect("sets");
        assert!(!disabled.cleanup.enabled);
        let french = settings
            .with_value("transcription.language", "FR")
            .expect("sets");
        assert_eq!(french.transcription.language.as_deref(), Some("fr"));
        let automatic = french
            .with_value("transcription.language", "null")
            .expect("sets");
        assert_eq!(automatic.transcription.language, None);
        let terms = settings
            .with_value("vocabulary", r#"["Tauri", "Siobhan"]"#)
            .expect("sets");
        assert_eq!(terms.vocabulary, vec!["Tauri", "Siobhan"]);
        assert_eq!(
            terms.value_at("vocabulary").expect("reads"),
            serde_json::json!(["Tauri", "Siobhan"])
        );
    }

    #[test]
    fn switching_provider_by_name_brings_its_address_and_model() {
        let settings = Settings::default()
            .with_value("transcription.apiKeyVariable", "MY_GROQ_KEY")
            .expect("sets");
        let switched = settings
            .with_value("transcription.provider", "elevenlabs")
            .expect("sets");
        assert_eq!(
            switched.transcription,
            TranscriptionSettings::for_provider(ProviderId::ElevenLabs)
        );
        let same = settings
            .with_value("transcription.provider", "groq")
            .expect("sets");
        assert_eq!(
            same.transcription.api_key_variable.as_deref(),
            Some("MY_GROQ_KEY")
        );
    }

    #[test]
    fn rejects_unknown_names_wrong_types_and_impossible_settings() {
        let settings = Settings::default();
        let unknown = settings
            .with_value("transcription.modle", "x")
            .expect_err("fails");
        assert!(unknown.to_string().contains("transcription.modle"));
        assert!(unknown.to_string().contains("model"));
        let typo = settings
            .with_value("transcription.provider", "elevenlab")
            .expect_err("fails");
        assert!(typo.to_string().contains("elevenlabs"));
        assert!(settings.with_value("cleanup.enabled", "maybe").is_err());
        assert!(
            settings
                .with_value("cleanup.provider", "elevenlabs")
                .is_err()
        );
        assert!(settings.with_value("shortcut.key", "x").is_err());
        assert!(
            settings
                .with_value("cleanup.apiKeyVariable", "1BAD-NAME")
                .is_err()
        );
    }

    #[test]
    fn resolves_the_api_key_variable_for_each_endpoint() {
        let settings = Settings::default()
            .with_value("transcription.provider", "elevenlabs")
            .and_then(|settings| settings.with_value("cleanup.apiKeyVariable", "  WORK_GROQ_KEY "))
            .expect("sets");
        assert_eq!(
            settings.transcription.resolve_api_key_variable().as_deref(),
            Some("ELEVENLABS_API_KEY")
        );
        assert_eq!(
            settings.cleanup.resolve_api_key_variable().as_deref(),
            Some("WORK_GROQ_KEY")
        );
    }

    #[test]
    fn load_and_validate_rejects_a_file_that_parses_but_cannot_work() {
        let directory = tempfile::tempdir().expect("temp dir");
        let path = directory.path().join("settings.json");
        fs::write(&path, r#"{"transcription": {"provider": "anthropic"}}"#).expect("writes");
        assert!(Settings::load(&path).is_ok());
        assert!(Settings::load_and_validate(&path).is_err());
    }
}

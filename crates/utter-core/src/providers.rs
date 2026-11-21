use std::time::Duration;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::Result;
use crate::http::{authorize, endpoint, parse_json};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum ProviderId {
    Groq,
    OpenAi,
    OpenRouter,
    Anthropic,
    Ollama,
    LmStudio,
    Custom,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ApiFlavor {
    OpenAiCompatible,
    Anthropic,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Role {
    Transcription,
    Cleanup,
}

#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ProviderPreset {
    pub id: ProviderId,
    pub label: &'static str,
    pub api: ApiFlavor,
    pub base_url: &'static str,
    pub transcription_model: Option<&'static str>,
    pub cleanup_model: Option<&'static str>,
    pub requires_api_key: bool,
    pub api_key_url: Option<&'static str>,
}

pub const PRESETS: &[ProviderPreset] = &[
    ProviderPreset {
        id: ProviderId::Groq,
        label: "Groq",
        api: ApiFlavor::OpenAiCompatible,
        base_url: "https://api.groq.com/openai/v1",
        transcription_model: Some("whisper-large-v3"),
        cleanup_model: Some("openai/gpt-oss-20b"),
        requires_api_key: true,
        api_key_url: Some("https://console.groq.com/keys"),
    },
    ProviderPreset {
        id: ProviderId::OpenAi,
        label: "OpenAI",
        api: ApiFlavor::OpenAiCompatible,
        base_url: "https://api.openai.com/v1",
        transcription_model: Some("gpt-4o-transcribe"),
        cleanup_model: Some("gpt-5-mini"),
        requires_api_key: true,
        api_key_url: Some("https://platform.openai.com/api-keys"),
    },
    ProviderPreset {
        id: ProviderId::OpenRouter,
        label: "OpenRouter",
        api: ApiFlavor::OpenAiCompatible,
        base_url: "https://openrouter.ai/api/v1",
        transcription_model: None,
        cleanup_model: Some("openai/gpt-oss-20b"),
        requires_api_key: true,
        api_key_url: Some("https://openrouter.ai/settings/keys"),
    },
    ProviderPreset {
        id: ProviderId::Anthropic,
        label: "Anthropic",
        api: ApiFlavor::Anthropic,
        base_url: "https://api.anthropic.com/v1",
        transcription_model: None,
        cleanup_model: Some("claude-opus-5-5"),
        requires_api_key: true,
        api_key_url: Some("https://platform.claude.com/settings/keys"),
    },
    ProviderPreset {
        id: ProviderId::Ollama,
        label: "Ollama (local)",
        api: ApiFlavor::OpenAiCompatible,
        base_url: "http://localhost:11434/v1",
        transcription_model: None,
        cleanup_model: Some("llama3.2"),
        requires_api_key: false,
        api_key_url: None,
    },
    ProviderPreset {
        id: ProviderId::LmStudio,
        label: "LM Studio (local)",
        api: ApiFlavor::OpenAiCompatible,
        base_url: "http://localhost:1234/v1",
        transcription_model: None,
        cleanup_model: None,
        requires_api_key: false,
        api_key_url: None,
    },
    ProviderPreset {
        id: ProviderId::Custom,
        label: "Custom (OpenAI-compatible)",
        api: ApiFlavor::OpenAiCompatible,
        base_url: "",
        transcription_model: None,
        cleanup_model: None,
        requires_api_key: false,
        api_key_url: None,
    },
];

impl ProviderId {
    pub fn preset(self) -> &'static ProviderPreset {
        PRESETS
            .iter()
            .find(|preset| preset.id == self)
            .unwrap_or_else(|| unreachable!("every provider has a preset"))
    }

    pub fn supports(self, role: Role) -> bool {
        match role {
            Role::Transcription => {
                self == ProviderId::Custom || self.preset().transcription_model.is_some()
            }
            Role::Cleanup => true,
        }
    }

    pub fn api_key_slot(self, role: Role) -> String {
        match (self, role) {
            (ProviderId::Custom, Role::Transcription) => "custom-transcription".to_owned(),
            (ProviderId::Custom, Role::Cleanup) => "custom-cleanup".to_owned(),
            (provider, _) => provider.as_str().to_owned(),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            ProviderId::Groq => "groq",
            ProviderId::OpenAi => "openai",
            ProviderId::OpenRouter => "openrouter",
            ProviderId::Anthropic => "anthropic",
            ProviderId::Ollama => "ollama",
            ProviderId::LmStudio => "lmstudio",
            ProviderId::Custom => "custom",
        }
    }
}

const MODELS_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Deserialize)]
struct ModelList {
    data: Vec<ModelEntry>,
}

#[derive(Deserialize)]
struct ModelEntry {
    id: String,
}

pub async fn list_models(
    client: &reqwest::Client,
    api: ApiFlavor,
    base_url: &str,
    api_key: Option<&str>,
) -> Result<Vec<String>> {
    let request = authorize(client.get(endpoint(base_url, "models")), api, api_key);
    let list: ModelList = parse_json(request.timeout(MODELS_TIMEOUT).send().await?).await?;
    let mut models: Vec<String> = list.data.into_iter().map(|model| model.id).collect();
    models.sort();
    models.dedup();
    Ok(models)
}

#[cfg(test)]
mod tests {
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;
    use crate::http::ANTHROPIC_VERSION;

    #[tokio::test]
    async fn lists_models_sorted_with_the_right_auth_header() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .and(header("authorization", "Bearer key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": [{"id": "whisper-large-v3"}, {"id": "llama-3.3-70b"}, {"id": "llama-3.3-70b"}]
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/anthropic/v1/models"))
            .and(header("x-api-key", "key"))
            .and(header("anthropic-version", ANTHROPIC_VERSION))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [{"id": "claude-opus-5-5", "type": "model"}], "has_more": false
            })))
            .mount(&server)
            .await;

        let client = reqwest::Client::new();
        let openai = list_models(
            &client,
            ApiFlavor::OpenAiCompatible,
            &format!("{}/v1", server.uri()),
            Some("key"),
        )
        .await
        .expect("lists");
        assert_eq!(openai, vec!["llama-3.3-70b", "whisper-large-v3"]);
        let anthropic = list_models(
            &client,
            ApiFlavor::Anthropic,
            &format!("{}/anthropic/v1", server.uri()),
            Some("key"),
        )
        .await
        .expect("lists");
        assert_eq!(anthropic, vec!["claude-opus-5-5"]);
    }

    const ALL: [ProviderId; 7] = [
        ProviderId::Groq,
        ProviderId::OpenAi,
        ProviderId::OpenRouter,
        ProviderId::Anthropic,
        ProviderId::Ollama,
        ProviderId::LmStudio,
        ProviderId::Custom,
    ];

    #[test]
    fn every_provider_has_exactly_one_preset() {
        assert_eq!(PRESETS.len(), ALL.len());
        for provider in ALL {
            assert_eq!(provider.preset().id, provider);
        }
    }

    #[test]
    fn serialized_ids_match_key_slots() {
        for provider in ALL {
            let json = serde_json::to_string(&provider).expect("provider ids serialize");
            assert_eq!(json, format!("\"{}\"", provider.as_str()));
        }
    }

    #[test]
    fn only_openai_compatible_providers_transcribe() {
        for provider in ALL.into_iter().filter(|p| p.supports(Role::Transcription)) {
            assert_eq!(provider.preset().api, ApiFlavor::OpenAiCompatible);
        }
        assert!(!ProviderId::Anthropic.supports(Role::Transcription));
        assert!(ProviderId::Groq.supports(Role::Transcription));
    }

    #[test]
    fn known_providers_share_a_key_across_roles_and_custom_does_not() {
        assert_eq!(
            ProviderId::Groq.api_key_slot(Role::Transcription),
            ProviderId::Groq.api_key_slot(Role::Cleanup)
        );
        assert_ne!(
            ProviderId::Custom.api_key_slot(Role::Transcription),
            ProviderId::Custom.api_key_slot(Role::Cleanup)
        );
    }
}

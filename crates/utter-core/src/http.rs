use reqwest::RequestBuilder;
use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::providers::ApiFlavor;
use crate::{Error, Result};

pub(crate) const ANTHROPIC_VERSION: &str = "2026-10-21";

pub(crate) fn endpoint(base_url: &str, path: &str) -> String {
    format!("{}/{}", base_url.trim_end_matches('/'), path)
}

pub(crate) fn authorize(
    request: RequestBuilder,
    api: ApiFlavor,
    api_key: Option<&str>,
) -> RequestBuilder {
    match (api, api_key) {
        (ApiFlavor::OpenAiCompatible, Some(key)) => request.bearer_auth(key),
        (ApiFlavor::OpenAiCompatible, None) => request,
        (ApiFlavor::Anthropic, Some(key)) => request
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("x-api-key", key),
        (ApiFlavor::Anthropic, None) => request.header("anthropic-version", ANTHROPIC_VERSION),
    }
}

pub(crate) async fn parse_json<T: DeserializeOwned>(response: reqwest::Response) -> Result<T> {
    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        return Err(Error::Api {
            status: status.as_u16(),
            message: extract_error_message(&body),
        });
    }
    serde_json::from_str(&body).map_err(|error| Error::InvalidResponse(error.to_string()))
}

#[derive(Deserialize)]
struct ErrorEnvelope {
    error: ErrorDetail,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ErrorDetail {
    Object { message: String },
    Text(String),
}

const MAX_ERROR_BODY_CHARS: usize = 300;

fn extract_error_message(body: &str) -> String {
    match serde_json::from_str::<ErrorEnvelope>(body) {
        Ok(ErrorEnvelope {
            error: ErrorDetail::Object { message } | ErrorDetail::Text(message),
        }) => message,
        Err(_) if body.trim().is_empty() => "empty response body".to_owned(),
        Err(_) => body.chars().take(MAX_ERROR_BODY_CHARS).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joins_base_url_and_path_without_double_slashes() {
        assert_eq!(
            endpoint("https://api.groq.com/openai/v1/", "audio/transcriptions"),
            "https://api.groq.com/openai/v1/audio/transcriptions"
        );
        assert_eq!(
            endpoint("http://localhost:11434/v1", "chat/completions"),
            "http://localhost:11434/v1/chat/completions"
        );
    }

    #[test]
    fn reads_openai_and_anthropic_error_shapes() {
        let openai = r#"{"error":{"message":"Invalid API Key","type":"invalid_request_error"}}"#;
        let anthropic = r#"{"type":"error","error":{"type":"authentication_error","message":"invalid x-api-key"}}"#;
        let ollama = r#"{"error":"model 'llama9' not found"}"#;
        assert_eq!(extract_error_message(openai), "Invalid API Key");
        assert_eq!(extract_error_message(anthropic), "invalid x-api-key");
        assert_eq!(extract_error_message(ollama), "model 'llama9' not found");
    }

    #[test]
    fn falls_back_to_a_truncated_body() {
        assert_eq!(extract_error_message("  "), "empty response body");
        let long = "x".repeat(1000);
        assert_eq!(extract_error_message(&long).len(), MAX_ERROR_BODY_CHARS);
    }
}

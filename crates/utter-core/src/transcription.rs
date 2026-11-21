use std::time::Duration;

use reqwest::multipart::{Form, Part};
use serde::Deserialize;

use crate::Result;
use crate::http::{authorize, endpoint, parse_json};
use crate::providers::ApiFlavor;

const MAX_PROMPT_CHARS: usize = 600;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(300);

pub struct TranscriptionRequest<'a> {
    pub base_url: &'a str,
    pub api_key: Option<&'a str>,
    pub model: &'a str,
    pub language: Option<&'a str>,
    pub vocabulary: &'a [String],
    pub wav: Vec<u8>,
}

#[derive(Deserialize)]
struct TranscriptionResponse {
    text: String,
}

pub async fn transcribe(
    client: &reqwest::Client,
    request: TranscriptionRequest<'_>,
) -> Result<String> {
    let audio = Part::bytes(request.wav)
        .file_name("dictation.wav")
        .mime_str("audio/wav")?;
    let mut form = Form::new()
        .part("file", audio)
        .text("model", request.model.to_owned())
        .text("response_format", "json")
        .text("temperature", "0");
    if let Some(language) = request.language {
        form = form.text("language", language.to_owned());
    }
    if let Some(prompt) = vocabulary_prompt(request.vocabulary) {
        form = form.text("prompt", prompt);
    }
    let http = authorize(
        client.post(endpoint(request.base_url, "audio/transcriptions")),
        ApiFlavor::OpenAiCompatible,
        request.api_key,
    );
    let response: TranscriptionResponse =
        parse_json(http.timeout(REQUEST_TIMEOUT).multipart(form).send().await?).await?;
    Ok(response.text.trim().to_owned())
}

fn vocabulary_prompt(vocabulary: &[String]) -> Option<String> {
    let mut prompt = String::new();
    for term in vocabulary {
        let separator = if prompt.is_empty() { "" } else { ", " };
        if prompt.chars().count() + separator.len() + term.chars().count() > MAX_PROMPT_CHARS {
            break;
        }
        prompt.push_str(separator);
        prompt.push_str(term);
    }
    (!prompt.is_empty()).then(|| format!("{prompt}."))
}

#[cfg(test)]
mod tests {
    use wiremock::matchers::{body_string_contains, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;
    use crate::Error;

    fn request<'a>(base_url: &'a str, vocabulary: &'a [String]) -> TranscriptionRequest<'a> {
        TranscriptionRequest {
            base_url,
            api_key: Some("test-key"),
            model: "whisper-large-v3",
            language: Some("en"),
            vocabulary,
            wav: b"RIFF....WAVE".to_vec(),
        }
    }

    #[tokio::test]
    async fn uploads_audio_as_multipart_and_reads_the_text() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/audio/transcriptions"))
            .and(header("authorization", "Bearer test-key"))
            .and(body_string_contains(
                "name=\"model\"\r\n\r\nwhisper-large-v3",
            ))
            .and(body_string_contains("name=\"language\"\r\n\r\nen"))
            .and(body_string_contains(
                "name=\"prompt\"\r\n\r\nTauri, Granola.",
            ))
            .and(body_string_contains("filename=\"dictation.wav\""))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"text": " Hello there. "})),
            )
            .expect(1)
            .mount(&server)
            .await;

        let base_url = format!("{}/v1", server.uri());
        let vocabulary = vec!["Tauri".to_owned(), "Granola".to_owned()];
        let text = transcribe(&reqwest::Client::new(), request(&base_url, &vocabulary))
            .await
            .expect("transcribes");
        assert_eq!(text, "Hello there.");
    }

    #[tokio::test]
    async fn surfaces_the_provider_error_message() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(401)
                    .set_body_json(serde_json::json!({"error": {"message": "Invalid API Key"}})),
            )
            .mount(&server)
            .await;

        let base_url = format!("{}/v1", server.uri());
        let error = transcribe(&reqwest::Client::new(), request(&base_url, &[]))
            .await
            .expect_err("fails");
        assert!(
            matches!(error, Error::Api { status: 401, ref message } if message == "Invalid API Key")
        );
    }

    #[test]
    fn vocabulary_prompt_stays_within_the_length_budget() {
        assert_eq!(vocabulary_prompt(&[]), None);
        let terms: Vec<String> = (0..500).map(|i| format!("term{i}")).collect();
        let prompt = vocabulary_prompt(&terms).expect("has terms");
        assert!(prompt.chars().count() <= MAX_PROMPT_CHARS + 1);
        assert!(prompt.starts_with("term0, term1"));
    }
}

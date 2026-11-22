use std::collections::HashSet;
use std::time::Duration;

use reqwest::multipart::{Form, Part};
use serde::Deserialize;

use crate::http::{authorize, endpoint, parse_json};
use crate::providers::ApiFlavor;
use crate::{Error, Result};

const MAX_PROMPT_CHARS: usize = 600;
const MAX_KEYTERMS: usize = 100;
const MAX_KEYTERM_CHARS: usize = 50;
const MAX_KEYTERM_WORDS: usize = 5;
const FORBIDDEN_KEYTERM_CHARS: [char; 7] = ['<', '>', '{', '}', '[', ']', '\\'];
const REQUEST_TIMEOUT: Duration = Duration::from_secs(300);

pub struct TranscriptionRequest<'a> {
    pub api: ApiFlavor,
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
    let TranscriptionRequest {
        api,
        base_url,
        api_key,
        model,
        language,
        vocabulary,
        wav,
    } = request;
    let audio = Part::bytes(wav)
        .file_name("dictation.wav")
        .mime_str("audio/wav")?;
    let (path, form) = match api {
        ApiFlavor::OpenAiCompatible => (
            "audio/transcriptions",
            openai_form(audio, model, language, vocabulary),
        ),
        ApiFlavor::ElevenLabs => (
            "speech-to-text",
            elevenlabs_form(audio, model, language, vocabulary),
        ),
        ApiFlavor::Anthropic => {
            return Err(Error::Settings(
                "Anthropic does not offer speech-to-text".to_owned(),
            ));
        }
    };
    let http = authorize(client.post(endpoint(base_url, path)), api, api_key);
    let response: TranscriptionResponse =
        parse_json(http.timeout(REQUEST_TIMEOUT).multipart(form).send().await?).await?;
    Ok(response.text.trim().to_owned())
}

fn openai_form(audio: Part, model: &str, language: Option<&str>, vocabulary: &[String]) -> Form {
    let mut form = Form::new()
        .part("file", audio)
        .text("model", model.to_owned())
        .text("response_format", "json")
        .text("temperature", "0");
    if let Some(language) = language {
        form = form.text("language", language.to_owned());
    }
    if let Some(prompt) = vocabulary_prompt(vocabulary) {
        form = form.text("prompt", prompt);
    }
    form
}

fn elevenlabs_form(
    audio: Part,
    model: &str,
    language: Option<&str>,
    vocabulary: &[String],
) -> Form {
    let mut form = Form::new()
        .part("file", audio)
        .text("model_id", model.to_owned())
        .text("tag_audio_events", "false")
        .text("timestamps_granularity", "none");
    if let Some(language) = language {
        form = form.text("language_code", language.to_owned());
    }
    for keyterm in select_keyterms(vocabulary) {
        form = form.text("keyterms", keyterm);
    }
    form
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

fn select_keyterms(vocabulary: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    vocabulary
        .iter()
        .map(|term| term.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|term| {
            !term.is_empty()
                && term.chars().count() <= MAX_KEYTERM_CHARS
                && term.split(' ').count() <= MAX_KEYTERM_WORDS
                && !term.contains(FORBIDDEN_KEYTERM_CHARS)
        })
        .filter(|term| seen.insert(term.to_lowercase()))
        .take(MAX_KEYTERMS)
        .collect()
}

#[cfg(test)]
mod tests {
    use wiremock::matchers::{body_string_contains, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;

    fn request<'a>(
        api: ApiFlavor,
        base_url: &'a str,
        model: &'a str,
        vocabulary: &'a [String],
    ) -> TranscriptionRequest<'a> {
        TranscriptionRequest {
            api,
            base_url,
            api_key: Some("test-key"),
            model,
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
        let text = transcribe(
            &reqwest::Client::new(),
            request(
                ApiFlavor::OpenAiCompatible,
                &base_url,
                "whisper-large-v3",
                &vocabulary,
            ),
        )
        .await
        .expect("transcribes");
        assert_eq!(text, "Hello there.");
    }

    #[tokio::test]
    async fn sends_elevenlabs_its_own_fields_and_one_field_per_keyterm() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/speech-to-text"))
            .and(header("xi-api-key", "test-key"))
            .and(body_string_contains("name=\"model_id\"\r\n\r\nscribe_v3"))
            .and(body_string_contains("name=\"language_code\"\r\n\r\nen"))
            .and(body_string_contains(
                "name=\"tag_audio_events\"\r\n\r\nfalse",
            ))
            .and(body_string_contains(
                "name=\"timestamps_granularity\"\r\n\r\nnone",
            ))
            .and(body_string_contains("name=\"keyterms\"\r\n\r\nTauri\r\n"))
            .and(body_string_contains("name=\"keyterms\"\r\n\r\nSiobhan\r\n"))
            .and(body_string_contains("filename=\"dictation.wav\""))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "language_code": "eng",
                "language_probability": 0.98,
                "text": "Hello Siobhan.",
                "words": []
            })))
            .expect(1)
            .mount(&server)
            .await;

        let base_url = format!("{}/v1", server.uri());
        let vocabulary = vec!["Tauri".to_owned(), "Siobhan".to_owned()];
        let text = transcribe(
            &reqwest::Client::new(),
            request(ApiFlavor::ElevenLabs, &base_url, "scribe_v3", &vocabulary),
        )
        .await
        .expect("transcribes");
        assert_eq!(text, "Hello Siobhan.");
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
        let error = transcribe(
            &reqwest::Client::new(),
            request(
                ApiFlavor::OpenAiCompatible,
                &base_url,
                "whisper-large-v3",
                &[],
            ),
        )
        .await
        .expect_err("fails");
        assert!(
            matches!(error, Error::Api { status: 401, ref message } if message == "Invalid API Key")
        );
    }

    #[tokio::test]
    async fn refuses_providers_without_speech_to_text() {
        let error = transcribe(
            &reqwest::Client::new(),
            request(
                ApiFlavor::Anthropic,
                "http://127.0.0.1:9",
                "claude-opus-5-5",
                &[],
            ),
        )
        .await
        .expect_err("fails");
        assert!(matches!(error, Error::Settings(_)));
    }

    #[test]
    fn vocabulary_prompt_stays_within_the_length_budget() {
        assert_eq!(vocabulary_prompt(&[]), None);
        let terms: Vec<String> = (0..500).map(|i| format!("term{i}")).collect();
        let prompt = vocabulary_prompt(&terms).expect("has terms");
        assert!(prompt.chars().count() <= MAX_PROMPT_CHARS + 1);
        assert!(prompt.starts_with("term0, term1"));
    }

    #[test]
    fn keyterms_keep_only_terms_elevenlabs_accepts() {
        let vocabulary: Vec<String> = [
            "  Tauri  ",
            "tauri",
            "",
            "Visual  Studio Code",
            "one two three four five six",
            "a[b]",
            "back\\slash",
            &"x".repeat(MAX_KEYTERM_CHARS + 1),
        ]
        .map(str::to_owned)
        .into();
        assert_eq!(
            select_keyterms(&vocabulary),
            vec!["Tauri", "Visual Studio Code"]
        );
    }

    #[test]
    fn keyterms_stop_at_the_limit_that_avoids_a_minimum_charge() {
        let vocabulary: Vec<String> = (0..300).map(|i| format!("term{i}")).collect();
        let keyterms = select_keyterms(&vocabulary);
        assert_eq!(keyterms.len(), MAX_KEYTERMS);
        assert_eq!(keyterms[0], "term0");
    }
}

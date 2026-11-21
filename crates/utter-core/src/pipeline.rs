use std::time::Instant;

use serde::Serialize;
use ts_rs::TS;

use crate::audio::Recording;
use crate::cleanup::{CleanupRequest, build_prompt, clean_up};
use crate::context::{TextContext, fit_to_cursor};
use crate::providers::ProviderId;
use crate::settings::Settings;
use crate::transcription::{TranscriptionRequest, transcribe};
use crate::{Error, Result};

#[derive(Clone, Debug, Default)]
pub struct ApiKeys {
    pub transcription: Option<String>,
    pub cleanup: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Dictation {
    pub raw_transcript: String,
    pub text: String,
    pub cleanup_error: Option<String>,
    pub audio_seconds: f64,
    pub transcription_ms: u32,
    pub cleanup_ms: Option<u32>,
}

pub async fn run(
    client: &reqwest::Client,
    settings: &Settings,
    keys: &ApiKeys,
    recording: &Recording,
    context: &TextContext,
) -> Result<Dictation> {
    let transcription_key = key_for(
        settings.transcription.provider,
        keys.transcription.as_deref(),
    )?;
    let wav = recording.to_wav()?;
    let started = Instant::now();
    let raw_transcript = transcribe(
        client,
        TranscriptionRequest {
            base_url: &settings.transcription.base_url,
            api_key: transcription_key,
            model: &settings.transcription.model,
            language: settings.transcription.language.as_deref(),
            vocabulary: &settings.vocabulary,
            wav,
        },
    )
    .await?;
    let transcription_ms = elapsed_ms(started);

    let mut dictation = Dictation {
        text: raw_transcript.clone(),
        raw_transcript,
        cleanup_error: None,
        audio_seconds: recording.duration().as_secs_f64(),
        transcription_ms,
        cleanup_ms: None,
    };

    if settings.cleanup.enabled && !dictation.raw_transcript.is_empty() {
        let started = Instant::now();
        match clean_up_transcript(client, settings, keys, context, &dictation.raw_transcript).await
        {
            Ok(text) => dictation.text = text,
            Err(error) => {
                log::warn!("cleanup failed, typing the raw transcript instead: {error}");
                dictation.cleanup_error = Some(error.to_string());
            }
        }
        dictation.cleanup_ms = Some(elapsed_ms(started));
    }

    dictation.text = fit_to_cursor(context.text_before_cursor.as_deref(), &dictation.text);
    Ok(dictation)
}

async fn clean_up_transcript(
    client: &reqwest::Client,
    settings: &Settings,
    keys: &ApiKeys,
    context: &TextContext,
    transcript: &str,
) -> Result<String> {
    let provider = settings.cleanup.provider;
    let api_key = key_for(provider, keys.cleanup.as_deref())?;
    let prompt = build_prompt(
        transcript,
        context,
        &settings.vocabulary,
        &settings.cleanup.instructions,
    );
    clean_up(
        client,
        CleanupRequest {
            api: provider.preset().api,
            base_url: &settings.cleanup.base_url,
            api_key,
            model: &settings.cleanup.model,
            prompt: &prompt,
        },
    )
    .await
}

fn key_for(provider: ProviderId, key: Option<&str>) -> Result<Option<&str>> {
    let key = key.filter(|key| !key.trim().is_empty());
    if key.is_none() && provider.preset().requires_api_key {
        return Err(Error::MissingApiKey(provider.preset().label.to_owned()));
    }
    Ok(key)
}

fn elapsed_ms(started: Instant) -> u32 {
    u32::try_from(started.elapsed().as_millis()).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;

    fn tone() -> Recording {
        let samples = (0..16_000).map(|i| 0.3 * (i as f32 * 0.05).sin()).collect();
        Recording::new(samples, 16_000)
    }

    async fn server_with_transcript(text: &str) -> MockServer {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/audio/transcriptions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"text": text})))
            .mount(&server)
            .await;
        server
    }

    fn settings_for(server: &MockServer, cleanup_enabled: bool) -> Settings {
        let mut settings = Settings::default();
        settings.transcription.base_url = format!("{}/v1", server.uri());
        settings.cleanup.base_url = format!("{}/v1", server.uri());
        settings.cleanup.enabled = cleanup_enabled;
        settings
    }

    fn keys() -> ApiKeys {
        ApiKeys {
            transcription: Some("key".into()),
            cleanup: Some("key".into()),
        }
    }

    #[tokio::test]
    async fn transcribes_then_cleans_up_and_fits_the_cursor() {
        let server = server_with_transcript("um hello priyanka").await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "choices": [{"message": {"content": "hello, Priyanka."}, "finish_reason": "stop"}]
            })))
            .mount(&server)
            .await;
        let context = TextContext {
            text_before_cursor: Some("Oh".into()),
            ..TextContext::default()
        };

        let dictation = run(
            &reqwest::Client::new(),
            &settings_for(&server, true),
            &keys(),
            &tone(),
            &context,
        )
        .await
        .expect("runs");
        assert_eq!(dictation.raw_transcript, "um hello priyanka");
        assert_eq!(dictation.text, " hello, Priyanka.");
        assert_eq!(dictation.cleanup_error, None);
        assert!(dictation.cleanup_ms.is_some());
        assert!((dictation.audio_seconds - 1.0).abs() < 0.01);
    }

    #[tokio::test]
    async fn falls_back_to_the_raw_transcript_when_cleanup_fails() {
        let server = server_with_transcript("hello there").await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(
                ResponseTemplate::new(503)
                    .set_body_json(json!({"error": {"message": "overloaded"}})),
            )
            .mount(&server)
            .await;

        let dictation = run(
            &reqwest::Client::new(),
            &settings_for(&server, true),
            &keys(),
            &tone(),
            &TextContext::default(),
        )
        .await
        .expect("still produces text");
        assert_eq!(dictation.text, "hello there");
        assert!(
            dictation
                .cleanup_error
                .expect("records the error")
                .contains("overloaded")
        );
    }

    #[tokio::test]
    async fn keeps_the_transcript_when_cleanup_replies_with_nothing() {
        let server = server_with_transcript("hello there").await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "choices": [{"message": {"content": "  "}, "finish_reason": "stop"}]
            })))
            .mount(&server)
            .await;

        let dictation = run(
            &reqwest::Client::new(),
            &settings_for(&server, true),
            &keys(),
            &tone(),
            &TextContext::default(),
        )
        .await
        .expect("still produces text");
        assert_eq!(dictation.text, "hello there");
        assert!(dictation.cleanup_error.is_some());
    }

    #[tokio::test]
    async fn skips_cleanup_when_disabled_or_nothing_was_said() {
        let server = server_with_transcript("").await;
        let dictation = run(
            &reqwest::Client::new(),
            &settings_for(&server, true),
            &keys(),
            &tone(),
            &TextContext::default(),
        )
        .await
        .expect("runs");
        assert_eq!(dictation.text, "");
        assert_eq!(dictation.cleanup_ms, None);

        let server = server_with_transcript("raw words").await;
        let dictation = run(
            &reqwest::Client::new(),
            &settings_for(&server, false),
            &keys(),
            &tone(),
            &TextContext::default(),
        )
        .await
        .expect("runs");
        assert_eq!(dictation.text, "raw words");
        assert_eq!(dictation.cleanup_ms, None);
    }

    #[tokio::test]
    async fn requires_a_key_for_hosted_providers_before_uploading_audio() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200))
            .expect(0)
            .mount(&server)
            .await;
        let error = run(
            &reqwest::Client::new(),
            &settings_for(&server, true),
            &ApiKeys::default(),
            &tone(),
            &TextContext::default(),
        )
        .await
        .expect_err("needs a key");
        assert!(matches!(error, Error::MissingApiKey(ref label) if label == "Groq"));
    }
}

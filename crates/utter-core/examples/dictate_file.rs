use std::env;
use std::error::Error;

use utter_core::audio::Recording;
use utter_core::context::TextContext;
use utter_core::pipeline::{self, ApiKeys};
use utter_core::providers::ProviderId;
use utter_core::settings::{Settings, TranscriptionSettings};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let path = args
        .next()
        .ok_or("usage: dictate_file <recording.wav> [text before the cursor]")?;
    let context = TextContext {
        text_before_cursor: args.next(),
        ..TextContext::default()
    };
    let mut settings = Settings::default();
    if let Ok(provider) = env::var("UTTER_TRANSCRIPTION_PROVIDER") {
        let provider: ProviderId = serde_json::from_value(serde_json::Value::String(provider))?;
        settings.transcription = TranscriptionSettings::for_provider(provider);
    }
    if let Ok(vocabulary) = env::var("UTTER_VOCABULARY") {
        settings.vocabulary = vocabulary
            .split(',')
            .map(|term| term.trim().to_owned())
            .collect();
    }
    let api_key_for = |provider: ProviderId| {
        provider
            .api_key_env_var()
            .and_then(|name| env::var(name).ok())
    };
    let keys = ApiKeys {
        transcription: api_key_for(settings.transcription.provider),
        cleanup: api_key_for(settings.cleanup.provider),
    };
    let dictation = pipeline::run(
        &reqwest::Client::new(),
        &settings,
        &keys,
        &read_wav(&path)?,
        &context,
    )
    .await?;
    println!("{}", serde_json::to_string_pretty(&dictation)?);
    Ok(())
}

fn read_wav(path: &str) -> Result<Recording, Box<dyn Error>> {
    let mut reader = hound::WavReader::open(path)?;
    let spec = reader.spec();
    let interleaved: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader.samples::<f32>().collect::<Result<_, _>>()?,
        hound::SampleFormat::Int => {
            let scale = (1_i64 << (spec.bits_per_sample - 1)) as f32;
            reader
                .samples::<i32>()
                .map(|sample| sample.map(|value| value as f32 / scale))
                .collect::<Result<_, _>>()?
        }
    };
    let channels = usize::from(spec.channels.max(1));
    let mono = interleaved
        .chunks(channels)
        .map(|frame| frame.iter().sum::<f32>() / frame.len() as f32)
        .collect();
    Ok(Recording::new(mono, spec.sample_rate))
}

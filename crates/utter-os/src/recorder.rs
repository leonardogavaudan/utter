use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample, StreamConfig};
use utter_core::audio::Recording;

const INITIAL_CAPACITY_SECONDS: usize = 30;

#[derive(Debug, thiserror::Error)]
pub enum RecorderError {
    #[error("no microphone is available")]
    NoMicrophone,
    #[error("the microphone uses an unsupported sample format ({0})")]
    UnsupportedFormat(String),
    #[error("the microphone could not be opened: {0}")]
    Device(String),
}

pub struct ActiveRecording {
    stop: mpsc::Sender<()>,
    worker: JoinHandle<Recording>,
}

impl ActiveRecording {
    pub fn start() -> Result<ActiveRecording, RecorderError> {
        let (stop, stop_requested) = mpsc::channel();
        let (ready, started) = mpsc::sync_channel(1);
        let worker = thread::Builder::new()
            .name("utter-recorder".to_owned())
            .spawn(move || record_until_stopped(&ready, &stop_requested))
            .map_err(|error| RecorderError::Device(error.to_string()))?;
        match started.recv() {
            Ok(Ok(())) => Ok(ActiveRecording { stop, worker }),
            Ok(Err(error)) => Err(error),
            Err(_) => Err(RecorderError::Device(
                "the recording thread exited".to_owned(),
            )),
        }
    }

    pub fn stop(self) -> Recording {
        let _ = self.stop.send(());
        self.worker
            .join()
            .unwrap_or_else(|_| Recording::new(Vec::new(), 0))
    }
}

fn record_until_stopped(
    ready: &mpsc::SyncSender<Result<(), RecorderError>>,
    stop_requested: &mpsc::Receiver<()>,
) -> Recording {
    let (stream, samples, config) = match open_stream() {
        Ok(opened) => opened,
        Err(error) => {
            let _ = ready.send(Err(error));
            return Recording::new(Vec::new(), 0);
        }
    };
    let _ = ready.send(Ok(()));
    let _ = stop_requested.recv();
    drop(stream);
    let mono = samples
        .lock()
        .map(|mut samples| std::mem::take(&mut *samples))
        .unwrap_or_default();
    Recording::new(mono, config.sample_rate)
}

type SharedSamples = Arc<Mutex<Vec<f32>>>;

fn open_stream() -> Result<(cpal::Stream, SharedSamples, StreamConfig), RecorderError> {
    let device = cpal::default_host()
        .default_input_device()
        .ok_or(RecorderError::NoMicrophone)?;
    let supported = device
        .default_input_config()
        .map_err(|error| RecorderError::Device(error.to_string()))?;
    let config = supported.config();
    let capacity = config.sample_rate as usize * INITIAL_CAPACITY_SECONDS;
    let samples: SharedSamples = Arc::new(Mutex::new(Vec::with_capacity(capacity)));
    let stream = match supported.sample_format() {
        SampleFormat::F32 => build_stream::<f32>(&device, config, &samples),
        SampleFormat::F64 => build_stream::<f64>(&device, config, &samples),
        SampleFormat::I16 => build_stream::<i16>(&device, config, &samples),
        SampleFormat::I32 => build_stream::<i32>(&device, config, &samples),
        SampleFormat::U16 => build_stream::<u16>(&device, config, &samples),
        SampleFormat::I8 => build_stream::<i8>(&device, config, &samples),
        SampleFormat::U8 => build_stream::<u8>(&device, config, &samples),
        other => return Err(RecorderError::UnsupportedFormat(other.to_string())),
    }?;
    stream
        .play()
        .map_err(|error| RecorderError::Device(error.to_string()))?;
    Ok((stream, samples, config))
}

fn build_stream<T>(
    device: &cpal::Device,
    config: StreamConfig,
    samples: &SharedSamples,
) -> Result<cpal::Stream, RecorderError>
where
    T: SizedSample,
    f32: FromSample<T>,
{
    let samples = Arc::clone(samples);
    let channels = usize::from(config.channels.max(1));
    device
        .build_input_stream(
            config,
            move |data: &[T], _: &cpal::InputCallbackInfo| {
                if let Ok(mut samples) = samples.lock() {
                    samples.extend(data.chunks(channels).map(|frame| {
                        frame
                            .iter()
                            .map(|sample| sample.to_sample::<f32>())
                            .sum::<f32>()
                            / frame.len() as f32
                    }));
                }
            },
            |error| log::error!("microphone stream error: {error}"),
            None,
        )
        .map_err(|error| RecorderError::Device(error.to_string()))
}

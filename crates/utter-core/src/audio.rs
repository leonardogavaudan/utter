use std::io::Cursor;
use std::time::Duration;

use rubato::audioadapter_buffers::direct::InterleavedSlice;
use rubato::{Fft, FixedSync, Resampler};

use crate::{Error, Result};

pub const UPLOAD_SAMPLE_RATE: u32 = 16_000;
pub const MIN_SPEECH_DURATION: Duration = Duration::from_millis(300);
pub const SILENCE_PEAK: f32 = 0.015;
pub const MAX_RECORDING_DURATION: Duration = Duration::from_secs(5 * 60);
const RESAMPLER_CHUNK_FRAMES: usize = 1024;

#[derive(Clone, Debug, PartialEq)]
pub struct Recording {
    samples: Vec<f32>,
    sample_rate: u32,
}

impl Recording {
    pub fn new(samples: Vec<f32>, sample_rate: u32) -> Recording {
        Recording {
            samples,
            sample_rate,
        }
    }

    pub fn samples(&self) -> &[f32] {
        &self.samples
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn duration(&self) -> Duration {
        if self.sample_rate == 0 {
            return Duration::ZERO;
        }
        Duration::from_secs_f64(self.samples.len() as f64 / f64::from(self.sample_rate))
    }

    pub fn peak(&self) -> f32 {
        self.samples
            .iter()
            .fold(0.0, |peak, sample| peak.max(sample.abs()))
    }

    pub fn has_speech(&self) -> bool {
        self.duration() >= MIN_SPEECH_DURATION && self.peak() >= SILENCE_PEAK
    }

    pub fn to_wav(&self) -> Result<Vec<u8>> {
        let samples = resample(&self.samples, self.sample_rate, UPLOAD_SAMPLE_RATE)?;
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: UPLOAD_SAMPLE_RATE,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut buffer = Cursor::new(Vec::with_capacity(44 + samples.len() * 2));
        let mut writer = hound::WavWriter::new(&mut buffer, spec).map_err(audio_error)?;
        for sample in samples {
            writer.write_sample(to_i16(sample)).map_err(audio_error)?;
        }
        writer.finalize().map_err(audio_error)?;
        Ok(buffer.into_inner())
    }
}

fn to_i16(sample: f32) -> i16 {
    (sample.clamp(-1.0, 1.0) * f32::from(i16::MAX)).round() as i16
}

fn resample(samples: &[f32], from_rate: u32, to_rate: u32) -> Result<Vec<f32>> {
    if from_rate == to_rate || samples.is_empty() {
        return Ok(samples.to_vec());
    }
    if from_rate == 0 {
        return Err(Error::Audio("the recording has no sample rate".to_owned()));
    }
    let mut resampler = Fft::<f32>::new(
        from_rate as usize,
        to_rate as usize,
        RESAMPLER_CHUNK_FRAMES,
        1,
        FixedSync::Input,
    )
    .map_err(audio_error)?;
    let input = InterleavedSlice::new(samples, 1, samples.len()).map_err(audio_error)?;
    let capacity = resampler.process_all_needed_output_len(samples.len());
    let mut output_samples = vec![0.0; capacity];
    let mut output =
        InterleavedSlice::new_mut(&mut output_samples, 1, capacity).map_err(audio_error)?;
    let (_, produced) = resampler
        .process_all_into_buffer(&input, &mut output, samples.len(), None)
        .map_err(audio_error)?;
    output_samples.truncate(produced);
    Ok(output_samples)
}

fn audio_error(error: impl std::fmt::Display) -> Error {
    Error::Audio(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(frequency: f32, sample_rate: u32, seconds: f32, amplitude: f32) -> Vec<f32> {
        let frames = (sample_rate as f32 * seconds) as usize;
        (0..frames)
            .map(|i| {
                amplitude
                    * (2.0 * std::f32::consts::PI * frequency * i as f32 / sample_rate as f32).sin()
            })
            .collect()
    }

    #[test]
    fn treats_short_or_quiet_recordings_as_silence() {
        assert!(!Recording::new(sine(440.0, 16_000, 0.1, 0.5), 16_000).has_speech());
        assert!(!Recording::new(sine(440.0, 16_000, 1.0, 0.001), 16_000).has_speech());
        assert!(Recording::new(sine(440.0, 16_000, 1.0, 0.3), 16_000).has_speech());
    }

    #[test]
    fn encodes_16_khz_mono_wav_from_any_input_rate() {
        for rate in [16_000, 44_100, 48_000] {
            let recording = Recording::new(sine(440.0, rate, 1.0, 0.5), rate);
            let wav = recording.to_wav().expect("encodes");
            let reader = hound::WavReader::new(Cursor::new(wav)).expect("valid wav");
            let spec = reader.spec();
            assert_eq!(
                (spec.channels, spec.sample_rate, spec.bits_per_sample),
                (1, 16_000, 16)
            );
            let frames = reader.len() as i64;
            assert!(
                (frames - 16_000).abs() <= 16,
                "{rate} Hz produced {frames} frames"
            );
        }
    }

    #[test]
    fn resampling_keeps_the_signal_level() {
        let samples = resample(&sine(440.0, 48_000, 1.0, 0.5), 48_000, 16_000).expect("resamples");
        let middle = &samples[2_000..14_000];
        let peak = middle.iter().fold(0.0f32, |peak, s| peak.max(s.abs()));
        assert!((peak - 0.5).abs() < 0.02, "peak was {peak}");
    }

    #[test]
    fn clamps_out_of_range_samples() {
        assert_eq!(to_i16(2.0), i16::MAX);
        assert_eq!(to_i16(-2.0), -i16::MAX);
        assert_eq!(to_i16(0.0), 0);
    }

    #[test]
    fn empty_recordings_encode_to_a_header_only_file() {
        let wav = Recording::new(Vec::new(), 48_000)
            .to_wav()
            .expect("encodes");
        assert_eq!(
            hound::WavReader::new(Cursor::new(wav))
                .expect("valid wav")
                .len(),
            0
        );
    }
}

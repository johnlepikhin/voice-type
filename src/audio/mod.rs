pub mod capture;

use std::io::Cursor;

use hound::{SampleFormat, WavSpec, WavWriter};

use crate::error::RecordingError;
use crate::provider::AudioData;
use crate::types::RmsLevel;

pub use capture::{AudioCapture, CaptureConfig, CapturedAudio};

use capture::calculate_rms;

impl CapturedAudio {
    /// Encode captured audio as WAV bytes and create [`AudioData`] for transcription.
    ///
    /// # Errors
    /// Returns `RecordingError` if samples are empty or WAV encoding fails.
    pub fn into_audio_data(self) -> Result<AudioData, RecordingError> {
        if self.samples.is_empty() {
            return Err(RecordingError::DeviceError(
                "No audio samples captured".to_owned(),
            ));
        }

        let wav_bytes = encode_wav(&self.samples, self.sample_rate)?;

        Ok(AudioData {
            wav_bytes,
            duration: self.duration,
        })
    }

    /// Calculate the overall RMS level of the captured audio.
    #[must_use]
    pub fn rms_level(&self) -> RmsLevel {
        calculate_rms(&self.samples)
    }

    /// Check if the audio is likely silence.
    #[must_use]
    pub fn is_silence(&self, threshold: RmsLevel) -> bool {
        self.rms_level().value() < threshold.value()
    }
}

/// Encode PCM samples as WAV bytes (mono, 16-bit).
fn encode_wav(samples: &[i16], sample_rate: u32) -> Result<Vec<u8>, RecordingError> {
    let spec = WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };

    let mut cursor = Cursor::new(Vec::new());
    {
        let mut writer = WavWriter::new(&mut cursor, spec)
            .map_err(|e| RecordingError::DeviceError(format!("WAV encoding error: {e}")))?;

        for &sample in samples {
            writer
                .write_sample(sample)
                .map_err(|e| RecordingError::DeviceError(format!("WAV write error: {e}")))?;
        }

        writer
            .finalize()
            .map_err(|e| RecordingError::DeviceError(format!("WAV finalize error: {e}")))?;
    }

    Ok(cursor.into_inner())
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn encode_wav_produces_valid_bytes() {
        let samples = vec![0i16; 16000]; // 1 second of silence at 16kHz
        let wav = encode_wav(&samples, 16000).unwrap();
        assert!(wav.len() > 44); // WAV header is 44 bytes
        assert_eq!(&wav[..4], b"RIFF");
    }

    #[test]
    fn captured_audio_silence_detection() {
        let audio = CapturedAudio {
            samples: vec![0i16; 1000],
            sample_rate: 16000,
            duration: Duration::from_millis(62),
        };
        assert!(audio.is_silence(RmsLevel::new(0.01)));
    }

    #[test]
    fn captured_audio_non_silence() {
        #[allow(clippy::cast_possible_truncation)]
        let samples: Vec<i16> = (0..1000).map(|i| (i * 30) as i16).collect();
        let audio = CapturedAudio {
            samples,
            sample_rate: 16000,
            duration: Duration::from_millis(62),
        };
        assert!(!audio.is_silence(RmsLevel::new(0.01)));
    }

    #[test]
    fn captured_audio_into_audio_data() {
        let audio = CapturedAudio {
            samples: vec![100i16; 16000],
            sample_rate: 16000,
            duration: Duration::from_secs(1),
        };
        let data = audio.into_audio_data().unwrap();
        assert!(!data.wav_bytes.is_empty());
        assert_eq!(data.duration, Duration::from_secs(1));
    }

    #[test]
    fn empty_samples_error() {
        let audio = CapturedAudio {
            samples: vec![],
            sample_rate: 16000,
            duration: Duration::from_secs(0),
        };
        assert!(audio.into_audio_data().is_err());
    }

    #[test]
    fn wav_roundtrip_via_hound() {
        let original: Vec<i16> = (0..1600).map(|i| ((i * 20) % 32000) as i16).collect();
        let wav_bytes = encode_wav(&original, 16000).unwrap();

        let cursor = Cursor::new(wav_bytes);
        let mut reader = hound::WavReader::new(cursor).unwrap();
        let spec = reader.spec();
        assert_eq!(spec.channels, 1);
        assert_eq!(spec.sample_rate, 16000);
        assert_eq!(spec.bits_per_sample, 16);

        let decoded: Vec<i16> = reader.samples::<i16>().map(|s| s.unwrap()).collect();
        assert_eq!(decoded, original);
    }
}

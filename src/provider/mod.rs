pub mod openai;

use std::time::Duration;

use crate::error::TranscriptionError;
use crate::types::{LanguageCode, TranscribedText};

/// Audio data ready for transcription.
#[derive(Debug, Clone)]
pub struct AudioData {
    /// Raw WAV-encoded bytes.
    pub wav_bytes: Vec<u8>,
    /// Duration of the audio.
    pub duration: Duration,
}

/// Options passed to the transcription provider.
#[derive(Debug, Clone, Default)]
pub struct TranscribeOptions {
    /// ISO-639-1 language hint for recognition accuracy.
    pub language: Option<LanguageCode>,
    /// System prompt to guide recognition style/context.
    pub prompt: Option<String>,
}

/// Result returned from a transcription provider.
#[derive(Debug, Clone)]
pub struct TranscriptionResult {
    /// The transcribed text.
    pub text: TranscribedText,
    /// How long the transcription request took.
    pub request_duration: Duration,
}

/// A speech-to-text transcription service.
///
/// Implementations MUST be `Send + Sync` to allow usage from
/// background threads. The `transcribe` method is blocking —
/// it runs on a dedicated thread, not the glib main loop.
pub trait TranscriptionProvider: Send + Sync {
    /// Transcribe audio data into text.
    ///
    /// # Errors
    /// Returns `TranscriptionError` on network, auth, or provider failures.
    fn transcribe(
        &self,
        audio: &AudioData,
        options: &TranscribeOptions,
    ) -> Result<TranscriptionResult, TranscriptionError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockProvider {
        response: String,
    }

    impl TranscriptionProvider for MockProvider {
        fn transcribe(
            &self,
            _audio: &AudioData,
            _options: &TranscribeOptions,
        ) -> Result<TranscriptionResult, TranscriptionError> {
            Ok(TranscriptionResult {
                text: TranscribedText::new(self.response.clone()),
                request_duration: Duration::from_millis(100),
            })
        }
    }

    #[test]
    fn mock_provider_works() {
        let provider = MockProvider {
            response: "hello world".to_owned(),
        };
        let audio = AudioData {
            wav_bytes: vec![0; 100],
            duration: Duration::from_secs(1),
        };
        let result = provider.transcribe(&audio, &TranscribeOptions::default());
        assert!(result.is_ok());
        assert_eq!(result.unwrap().text.as_str(), "hello world");
    }
}

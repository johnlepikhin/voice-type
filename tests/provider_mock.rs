use std::time::Duration;

use voice_type::audio::CapturedAudio;
use voice_type::error::TranscriptionError;
use voice_type::provider::{
    AudioData, TranscribeOptions, TranscriptionProvider, TranscriptionResult,
};
use voice_type::types::{LanguageCode, RmsLevel, TranscribedText};

/// Mock provider that returns a fixed response.
struct MockProvider {
    response: Result<String, TranscriptionError>,
    delay: Duration,
}

impl TranscriptionProvider for MockProvider {
    fn transcribe(
        &self,
        _audio: &AudioData,
        _options: &TranscribeOptions,
    ) -> Result<TranscriptionResult, TranscriptionError> {
        std::thread::sleep(self.delay);
        match &self.response {
            Ok(text) => Ok(TranscriptionResult {
                text: TranscribedText::new(text.clone()),
                request_duration: self.delay,
            }),
            Err(e) => Err(match e {
                TranscriptionError::AuthenticationError => TranscriptionError::AuthenticationError,
                TranscriptionError::Timeout => TranscriptionError::Timeout,
                TranscriptionError::EmptyAudio => TranscriptionError::EmptyAudio,
                TranscriptionError::NetworkError(msg) => {
                    TranscriptionError::NetworkError(msg.clone())
                }
                TranscriptionError::ProviderError { status, message } => {
                    TranscriptionError::ProviderError {
                        status: *status,
                        message: message.clone(),
                    }
                }
                _ => TranscriptionError::NetworkError("unknown error".to_owned()),
            }),
        }
    }
}

#[test]
fn mock_transcription_success() {
    let provider = MockProvider {
        response: Ok("Hello world".to_owned()),
        delay: Duration::ZERO,
    };
    let audio = AudioData {
        wav_bytes: vec![0; 100],
        duration: Duration::from_secs(1),
    };
    let result = provider
        .transcribe(&audio, &TranscribeOptions::default())
        .unwrap();
    assert_eq!(result.text.as_str(), "Hello world");
}

#[test]
fn mock_transcription_with_options() {
    let provider = MockProvider {
        response: Ok("Привет мир".to_owned()),
        delay: Duration::ZERO,
    };
    let audio = AudioData {
        wav_bytes: vec![0; 100],
        duration: Duration::from_secs(1),
    };
    let options = TranscribeOptions {
        language: Some(LanguageCode::new("ru").unwrap()),
        prompt: Some("Technical discussion".to_owned()),
    };
    let result = provider.transcribe(&audio, &options).unwrap();
    assert_eq!(result.text.as_str(), "Привет мир");
}

#[test]
fn mock_transcription_auth_error() {
    let provider = MockProvider {
        response: Err(TranscriptionError::AuthenticationError),
        delay: Duration::ZERO,
    };
    let audio = AudioData {
        wav_bytes: vec![0; 100],
        duration: Duration::from_secs(1),
    };
    let err = provider
        .transcribe(&audio, &TranscribeOptions::default())
        .unwrap_err();
    assert!(matches!(err, TranscriptionError::AuthenticationError));
    assert!(err.to_string().contains("Authentication"));
}

#[test]
fn mock_transcription_network_error() {
    let provider = MockProvider {
        response: Err(TranscriptionError::NetworkError(
            "connection refused".to_owned(),
        )),
        delay: Duration::ZERO,
    };
    let audio = AudioData {
        wav_bytes: vec![0; 100],
        duration: Duration::from_secs(1),
    };
    let err = provider
        .transcribe(&audio, &TranscribeOptions::default())
        .unwrap_err();
    assert!(matches!(err, TranscriptionError::NetworkError(_)));
    assert!(err.to_string().contains("connection refused"));
}

#[test]
fn mock_transcription_empty_audio() {
    let provider = MockProvider {
        response: Err(TranscriptionError::EmptyAudio),
        delay: Duration::ZERO,
    };
    let audio = AudioData {
        wav_bytes: vec![0; 100],
        duration: Duration::from_secs(1),
    };
    let err = provider
        .transcribe(&audio, &TranscribeOptions::default())
        .unwrap_err();
    assert!(matches!(err, TranscriptionError::EmptyAudio));
    assert!(err.to_string().contains("No speech"));
}

#[test]
fn mock_transcription_provider_error() {
    let provider = MockProvider {
        response: Err(TranscriptionError::ProviderError {
            status: 429,
            message: "Rate limit exceeded".to_owned(),
        }),
        delay: Duration::ZERO,
    };
    let audio = AudioData {
        wav_bytes: vec![0; 100],
        duration: Duration::from_secs(1),
    };
    let err = provider
        .transcribe(&audio, &TranscribeOptions::default())
        .unwrap_err();
    assert!(matches!(
        err,
        TranscriptionError::ProviderError { status: 429, .. }
    ));
    assert!(err.to_string().contains("429"));
}

#[test]
fn audio_data_from_captured_audio() {
    let captured = CapturedAudio {
        samples: vec![5000i16; 16000],
        sample_rate: 16000,
        duration: Duration::from_secs(1),
    };

    // Should not be silence
    assert!(!captured.is_silence(RmsLevel::new(0.01)));

    let audio_data = captured.into_audio_data().unwrap();
    assert!(!audio_data.wav_bytes.is_empty());
    assert_eq!(audio_data.duration, Duration::from_secs(1));

    // WAV header check
    assert_eq!(&audio_data.wav_bytes[..4], b"RIFF");
}

#[test]
fn silence_detection() {
    let silent = CapturedAudio {
        samples: vec![0i16; 16000],
        sample_rate: 16000,
        duration: Duration::from_secs(1),
    };
    assert!(silent.is_silence(RmsLevel::new(0.01)));

    let noisy: Vec<i16> = (0..16000).map(|i| ((i * 50) % 32000) as i16).collect();
    let loud = CapturedAudio {
        samples: noisy,
        sample_rate: 16000,
        duration: Duration::from_secs(1),
    };
    assert!(!loud.is_silence(RmsLevel::new(0.01)));
}

#[test]
fn full_pipeline_mock() {
    // Simulate the full capture → encode → transcribe pipeline
    let captured = CapturedAudio {
        samples: vec![500i16; 16000],
        sample_rate: 16000,
        duration: Duration::from_secs(1),
    };

    assert!(!captured.is_silence(RmsLevel::new(0.01)));

    let audio_data = captured.into_audio_data().unwrap();
    assert!(!audio_data.wav_bytes.is_empty());

    let provider = MockProvider {
        response: Ok("Test transcription result".to_owned()),
        delay: Duration::ZERO,
    };

    let options = TranscribeOptions {
        language: Some(LanguageCode::new("en").unwrap()),
        prompt: None,
    };

    let result = provider.transcribe(&audio_data, &options).unwrap();
    assert_eq!(result.text.as_str(), "Test transcription result");
}

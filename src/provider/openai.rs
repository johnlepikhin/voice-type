use std::io::Write as _;
use std::time::{Duration, Instant};

use serde::Deserialize;
use ureq::Agent;

use crate::config::Secret;
use crate::error::TranscriptionError;
use crate::types::TranscribedText;

use super::{AudioData, TranscribeOptions, TranscriptionProvider, TranscriptionResult};

/// Default `OpenAI` Whisper API endpoint.
const DEFAULT_ENDPOINT: &str = "https://api.openai.com/v1/audio/transcriptions";

/// Multipart boundary for form-data requests.
const BOUNDARY: &str = "----VoiceTypeBoundary9876543210";

/// `OpenAI` Whisper transcription provider.
///
/// Sends WAV audio to the `OpenAI` `/v1/audio/transcriptions` endpoint
/// and returns the transcribed text. Runs on a background thread
/// (blocking HTTP via ureq).
pub struct OpenAiWhisperProvider {
    agent: Agent,
    api_key: Secret,
    model: String,
    endpoint: String,
}

/// JSON response from the `OpenAI` transcription API.
#[derive(Deserialize)]
struct WhisperResponse {
    text: String,
}

impl OpenAiWhisperProvider {
    /// Create a new provider from configuration values.
    #[must_use]
    pub fn new(api_key: Secret, model: String, timeout: Duration) -> Self {
        let config = Agent::config_builder()
            .timeout_global(Some(timeout))
            .max_idle_age(crate::http::HTTP_IDLE_TIMEOUT)
            .http_status_as_error(false)
            .build();
        let agent = Agent::new_with_config(config);

        Self {
            agent,
            api_key,
            model,
            endpoint: DEFAULT_ENDPOINT.to_owned(),
        }
    }

    /// Build a multipart/form-data body for the Whisper API.
    fn build_multipart_body(
        wav_bytes: &[u8],
        model: &str,
        language: Option<&str>,
        prompt: Option<&str>,
    ) -> Vec<u8> {
        let mut body = Vec::with_capacity(wav_bytes.len() + 512);

        // File field
        write!(body, "--{BOUNDARY}\r\n").unwrap();
        write!(
            body,
            "Content-Disposition: form-data; name=\"file\"; filename=\"audio.wav\"\r\n"
        )
        .unwrap();
        write!(body, "Content-Type: audio/wav\r\n").unwrap();
        write!(body, "\r\n").unwrap();
        body.extend_from_slice(wav_bytes);
        write!(body, "\r\n").unwrap();

        // Model field
        write!(body, "--{BOUNDARY}\r\n").unwrap();
        write!(body, "Content-Disposition: form-data; name=\"model\"\r\n").unwrap();
        write!(body, "\r\n").unwrap();
        write!(body, "{model}\r\n").unwrap();

        // Optional language
        if let Some(lang) = language {
            write!(body, "--{BOUNDARY}\r\n").unwrap();
            write!(
                body,
                "Content-Disposition: form-data; name=\"language\"\r\n"
            )
            .unwrap();
            write!(body, "\r\n").unwrap();
            write!(body, "{lang}\r\n").unwrap();
        }

        // Optional prompt
        if let Some(p) = prompt {
            write!(body, "--{BOUNDARY}\r\n").unwrap();
            write!(body, "Content-Disposition: form-data; name=\"prompt\"\r\n").unwrap();
            write!(body, "\r\n").unwrap();
            write!(body, "{p}\r\n").unwrap();
        }

        write!(body, "--{BOUNDARY}--\r\n").unwrap();
        body
    }
}

impl TranscriptionProvider for OpenAiWhisperProvider {
    fn transcribe(
        &self,
        audio: &AudioData,
        options: &TranscribeOptions,
    ) -> Result<TranscriptionResult, TranscriptionError> {
        let api_key = self
            .api_key
            .unsecure()
            .map_err(|_| TranscriptionError::AuthenticationError)?;

        let body = Self::build_multipart_body(
            &audio.wav_bytes,
            &self.model,
            options
                .language
                .as_ref()
                .map(crate::types::LanguageCode::as_str),
            options.prompt.as_deref(),
        );

        let content_type = format!("multipart/form-data; boundary={BOUNDARY}");
        let auth_header = format!("Bearer {api_key}");

        let start = Instant::now();

        let result = self
            .agent
            .post(&self.endpoint)
            .header("Authorization", &auth_header)
            .header("Content-Type", &content_type)
            .send(&body);

        // Free the multipart body (~20MB for 5-min recording) before parsing response.
        drop(body);

        let request_duration = start.elapsed();

        let mut response = match result {
            Ok(resp) => resp,
            Err(e) => return Err(crate::http::TransportError::from(e).into()),
        };

        let status = response.status().as_u16();

        let body_str = response
            .body_mut()
            .read_to_string()
            .map_err(|e| TranscriptionError::NetworkError(e.to_string()))?;

        if status == 401 {
            return Err(TranscriptionError::AuthenticationError);
        }

        if !(200..300).contains(&status) {
            let message = crate::http::extract_api_error(&body_str);
            return Err(TranscriptionError::ProviderError { status, message });
        }

        let whisper_response: WhisperResponse =
            serde_json::from_str(&body_str).map_err(|e| TranscriptionError::ProviderError {
                status,
                message: format!("Failed to parse response: {e}"),
            })?;

        if whisper_response.text.trim().is_empty() {
            return Err(TranscriptionError::EmptyAudio);
        }

        Ok(TranscriptionResult {
            text: TranscribedText::new(whisper_response.text),
            request_duration,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multipart_body_structure() {
        let wav = vec![0u8; 100];
        let body = OpenAiWhisperProvider::build_multipart_body(&wav, "whisper-1", None, None);
        let body_str = String::from_utf8_lossy(&body);

        assert!(body_str.contains("name=\"file\""));
        assert!(body_str.contains("filename=\"audio.wav\""));
        assert!(body_str.contains("name=\"model\""));
        assert!(body_str.contains("whisper-1"));
        assert!(body_str.contains(&format!("--{BOUNDARY}--")));
    }

    #[test]
    fn multipart_body_with_options() {
        let wav = vec![0u8; 50];
        let body = OpenAiWhisperProvider::build_multipart_body(
            &wav,
            "whisper-1",
            Some("ru"),
            Some("test"),
        );
        let body_str = String::from_utf8_lossy(&body);

        assert!(body_str.contains("name=\"language\""));
        assert!(body_str.contains("ru"));
        assert!(body_str.contains("name=\"prompt\""));
        assert!(body_str.contains("test"));
    }
}

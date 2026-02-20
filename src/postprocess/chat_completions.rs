use std::time::Duration;

use serde::{Deserialize, Serialize};
use ureq::Agent;

use crate::error::PostProcessingError;

use super::config::PostProcessorConfig;

/// Default `OpenAI` chat completions API path.
const CHAT_COMPLETIONS_PATH: &str = "/v1/chat/completions";

/// `OpenAI`-compatible chat completions client.
///
/// Sends a system prompt + user text to the chat completions API
/// and returns the assistant's response content. Runs on a background
/// thread (blocking HTTP via ureq).
pub struct ChatCompletionsClient {
    agent: Agent,
    api_key: crate::config::Secret,
    model: String,
    endpoint: String,
    system_prompt: String,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    max_retries: u32,
}

/// Chat completion request body.
#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

/// A single message in the chat conversation.
#[derive(Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

/// Response from the chat completions API.
#[derive(Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatChoice>,
}

/// A single choice in the completion response.
#[derive(Deserialize)]
struct ChatChoice {
    message: ChatResponseMessage,
}

/// The assistant message in the response.
#[derive(Deserialize)]
struct ChatResponseMessage {
    content: Option<String>,
}

/// Error response from the `OpenAI` API.
#[derive(Deserialize)]
struct OpenAiErrorResponse {
    error: OpenAiErrorDetail,
}

/// Inner error detail.
#[derive(Deserialize)]
struct OpenAiErrorDetail {
    message: String,
}

impl ChatCompletionsClient {
    /// Create a new chat completions client from processor configuration.
    #[must_use]
    pub fn new(config: &PostProcessorConfig) -> Self {
        let agent_config = Agent::config_builder()
            .timeout_global(Some(config.timeout))
            .max_idle_age(crate::HTTP_IDLE_TIMEOUT)
            .http_status_as_error(false)
            .build();
        let agent = Agent::new_with_config(agent_config);

        Self {
            agent,
            api_key: config.api_key.clone(),
            model: config.model.clone(),
            endpoint: config.endpoint.clone(),
            system_prompt: config.system_prompt.clone(),
            temperature: config.temperature,
            max_tokens: config.max_tokens,
            max_retries: config.max_retries,
        }
    }

    /// Send text through the chat completions API with retry on transient errors.
    ///
    /// Retries up to `max_retries` times with exponential backoff for
    /// retryable errors (network, timeout, 429, 5xx).
    ///
    /// # Errors
    /// Returns `PostProcessingError` on network, auth, provider, or empty response failures.
    pub fn send(&self, user_text: &str) -> Result<String, PostProcessingError> {
        let mut last_error = match self.send_once(user_text) {
            Ok(result) => return Ok(result),
            Err(e) => e,
        };

        for attempt in 0..self.max_retries {
            if !last_error.is_retryable() {
                return Err(last_error);
            }
            let delay = retry_delay(attempt);
            tracing::warn!(
                attempt = attempt + 1,
                max = self.max_retries,
                delay_ms = delay.as_secs() * 1000 + u64::from(delay.subsec_millis()),
                error = %last_error,
                "Retrying post-processing request"
            );
            std::thread::sleep(delay);

            match self.send_once(user_text) {
                Ok(result) => return Ok(result),
                Err(e) => last_error = e,
            }
        }

        Err(last_error)
    }

    /// Perform a single HTTP request to the chat completions API.
    fn send_once(&self, user_text: &str) -> Result<String, PostProcessingError> {
        let api_key = self
            .api_key
            .unsecure()
            .map_err(|_| PostProcessingError::AuthenticationError)?;

        let request_body = ChatRequest {
            model: &self.model,
            messages: vec![
                ChatMessage {
                    role: "system",
                    content: &self.system_prompt,
                },
                ChatMessage {
                    role: "user",
                    content: user_text,
                },
            ],
            temperature: self.temperature,
            max_tokens: self.max_tokens,
        };

        let url = format!("{}{CHAT_COMPLETIONS_PATH}", self.endpoint);
        let auth_header = format!("Bearer {api_key}");

        let body_json = serde_json::to_vec(&request_body)
            .map_err(|e| PostProcessingError::NetworkError(e.to_string()))?;

        let result = self
            .agent
            .post(&url)
            .header("Authorization", &auth_header)
            .header("Content-Type", "application/json")
            .send(&body_json);

        let mut response = match result {
            Ok(resp) => resp,
            Err(e) => return Err(map_ureq_error(e)),
        };

        let status = response.status().as_u16();

        let body_str = response
            .body_mut()
            .read_to_string()
            .map_err(|e| PostProcessingError::NetworkError(e.to_string()))?;

        if status == 401 {
            return Err(PostProcessingError::AuthenticationError);
        }

        if !(200..300).contains(&status) {
            let message = serde_json::from_str::<OpenAiErrorResponse>(&body_str)
                .map(|r| r.error.message)
                .unwrap_or(body_str);
            return Err(PostProcessingError::ProviderError { status, message });
        }

        let response: ChatCompletionResponse =
            serde_json::from_str(&body_str).map_err(|e| PostProcessingError::ProviderError {
                status,
                message: format!("Failed to parse response: {e}"),
            })?;

        let content = response
            .choices
            .into_iter()
            .next()
            .and_then(|c| c.message.content)
            .unwrap_or_default();

        if content.trim().is_empty() {
            return Err(PostProcessingError::EmptyResponse);
        }

        Ok(content)
    }
}

/// Compute retry delay with exponential backoff: 500ms base, doubling, capped at 5s.
fn retry_delay(attempt: u32) -> Duration {
    let base_ms = 500u64;
    let shift = 1u64.checked_shl(attempt).unwrap_or(u64::MAX);
    let delay_ms = base_ms.saturating_mul(shift);
    Duration::from_millis(delay_ms.min(5000))
}

/// Map ureq transport-level errors to post-processing error types.
fn map_ureq_error(error: ureq::Error) -> PostProcessingError {
    match error {
        ureq::Error::Timeout(_) => PostProcessingError::Timeout,
        other => PostProcessingError::NetworkError(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_delay_exponential_backoff() {
        assert_eq!(retry_delay(0), Duration::from_millis(500));
        assert_eq!(retry_delay(1), Duration::from_millis(1000));
        assert_eq!(retry_delay(2), Duration::from_millis(2000));
        assert_eq!(retry_delay(3), Duration::from_millis(4000));
    }

    #[test]
    fn retry_delay_capped_at_5s() {
        assert_eq!(retry_delay(4), Duration::from_millis(5000));
        assert_eq!(retry_delay(10), Duration::from_millis(5000));
        // Must not panic even with extreme values
        assert_eq!(retry_delay(64), Duration::from_millis(5000));
        assert_eq!(retry_delay(u32::MAX), Duration::from_millis(5000));
    }
}

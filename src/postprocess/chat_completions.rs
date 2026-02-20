use std::time::Duration;

use serde::{Deserialize, Serialize};
use ureq::Agent;

use crate::config::Secret;
use crate::error::PostProcessingError;

/// Default `OpenAI` chat completions API path.
const CHAT_COMPLETIONS_PATH: &str = "/v1/chat/completions";

/// `OpenAI`-compatible chat completions client.
///
/// Sends a system prompt + user text to the chat completions API
/// and returns the assistant's response content. Runs on a background
/// thread (blocking HTTP via ureq).
pub struct ChatCompletionsClient {
    agent: Agent,
    api_key: Secret,
    model: String,
    endpoint: String,
    system_prompt: String,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
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
    /// Create a new chat completions client from processor config values.
    #[must_use]
    pub fn new(
        api_key: Secret,
        model: String,
        endpoint: String,
        system_prompt: String,
        timeout: Duration,
        temperature: Option<f32>,
        max_tokens: Option<u32>,
    ) -> Self {
        let config = Agent::config_builder()
            .timeout_global(Some(timeout))
            .http_status_as_error(false)
            .build();
        let agent = Agent::new_with_config(config);

        Self {
            agent,
            api_key,
            model,
            endpoint,
            system_prompt,
            temperature,
            max_tokens,
        }
    }

    /// Send text through the chat completions API and return the processed result.
    ///
    /// # Errors
    /// Returns `PostProcessingError` on network, auth, provider, or empty response failures.
    pub fn send(&self, user_text: &str) -> Result<String, PostProcessingError> {
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

/// Map ureq transport-level errors to post-processing error types.
fn map_ureq_error(error: ureq::Error) -> PostProcessingError {
    match error {
        ureq::Error::Timeout(_) => PostProcessingError::Timeout,
        other => PostProcessingError::NetworkError(other.to_string()),
    }
}

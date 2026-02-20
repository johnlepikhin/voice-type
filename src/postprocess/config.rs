use std::fmt;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use structdoc::{Documentation, StructDoc};

use crate::config::Secret;
use crate::error::ValidationError;

/// Default chat completions endpoint.
const DEFAULT_CHAT_ENDPOINT: &str = "https://api.openai.com";

/// Default post-processor timeout.
fn default_processor_timeout() -> Duration {
    Duration::from_secs(15)
}

/// Default maximum retry attempts for transient errors.
fn default_max_retries() -> u32 {
    3
}

/// Default model for chat completions.
fn default_processor_model() -> String {
    "gpt-4o-mini".to_owned()
}

/// Default endpoint for chat completions.
fn default_processor_endpoint() -> String {
    DEFAULT_CHAT_ENDPOINT.to_owned()
}

/// Human-readable processor name used in progress display and error messages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessorName(String);

impl ProcessorName {
    /// Create a new processor name.
    ///
    /// # Errors
    /// Returns error if the name is empty.
    pub fn new(name: &str) -> Result<Self, String> {
        if name.trim().is_empty() {
            return Err("Processor name cannot be empty".to_owned());
        }
        Ok(Self(name.to_owned()))
    }

    /// Get the name as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ProcessorName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl StructDoc for ProcessorName {
    fn document() -> Documentation {
        Documentation::leaf("Human-readable processor name (e.g., \"Grammar\", \"Translate\")")
    }
}

/// Configuration for a single post-processor.
#[derive(Debug, Clone, Serialize, Deserialize, StructDoc)]
pub struct PostProcessorConfig {
    /// Human-readable name for progress display and error messages.
    pub name: ProcessorName,

    /// System prompt sent to the LLM.
    pub system_prompt: String,

    /// API key for the chat completions provider.
    pub api_key: Secret,

    /// Model name (e.g., "gpt-4o-mini").
    #[serde(default = "default_processor_model")]
    pub model: String,

    /// Base endpoint URL (e.g., "<https://api.openai.com>").
    #[serde(default = "default_processor_endpoint")]
    pub endpoint: String,

    /// Request timeout for this processor.
    #[serde(default = "default_processor_timeout", with = "humantime_serde")]
    pub timeout: Duration,

    /// LLM temperature (0.0..=2.0). Omitted if `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    /// Maximum tokens in LLM response. Omitted if `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,

    /// Maximum retry attempts for transient errors (0 = no retry).
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
}

impl PostProcessorConfig {
    /// Validate this processor's config, appending errors to `errors`.
    pub fn validate_into(&self, index: usize, errors: &mut Vec<ValidationError>) {
        let prefix = format!("post_processing[{index}]");

        if self.name.as_str().trim().is_empty() {
            errors.push(ValidationError {
                field: format!("{prefix}.name"),
                message: "name cannot be empty".to_owned(),
                suggestion: Some("Provide a descriptive name like \"Grammar\"".to_owned()),
            });
        }

        if self.system_prompt.trim().is_empty() {
            errors.push(ValidationError {
                field: format!("{prefix}.system_prompt"),
                message: "system_prompt cannot be empty".to_owned(),
                suggestion: Some(
                    "Provide instructions for the LLM, e.g. \"Fix grammar and punctuation.\""
                        .to_owned(),
                ),
            });
        }

        if self.model.trim().is_empty() {
            errors.push(ValidationError {
                field: format!("{prefix}.model"),
                message: "model cannot be empty".to_owned(),
                suggestion: Some("Use \"gpt-4o-mini\" for fast post-processing".to_owned()),
            });
        }

        if let Some(temp) = self.temperature {
            if !(0.0..=2.0).contains(&temp) {
                errors.push(ValidationError {
                    field: format!("{prefix}.temperature"),
                    message: format!("temperature {temp} is out of range (0.0..=2.0)"),
                    suggestion: Some("Use 0.3 for deterministic output".to_owned()),
                });
            }
        }

        if let Some(max) = self.max_tokens {
            if max == 0 {
                errors.push(ValidationError {
                    field: format!("{prefix}.max_tokens"),
                    message: "max_tokens must be greater than 0".to_owned(),
                    suggestion: Some("Use 2048 as a reasonable default".to_owned()),
                });
            }
        }

        if self.max_retries > 10 {
            errors.push(ValidationError {
                field: format!("{prefix}.max_retries"),
                message: format!("max_retries {} is too high (max 10)", self.max_retries),
                suggestion: Some("Use 3 as a reasonable default".to_owned()),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn processor_name_valid() {
        let name = ProcessorName::new("Grammar").unwrap();
        assert_eq!(name.as_str(), "Grammar");
        assert_eq!(name.to_string(), "Grammar");
    }

    #[test]
    fn processor_name_empty_rejected() {
        assert!(ProcessorName::new("").is_err());
        assert!(ProcessorName::new("   ").is_err());
    }

    #[test]
    fn validate_empty_name() {
        let config = PostProcessorConfig {
            name: ProcessorName(String::new()),
            system_prompt: "Fix grammar.".to_owned(),
            api_key: Secret::from_string("test"),
            model: "gpt-4o-mini".to_owned(),
            endpoint: DEFAULT_CHAT_ENDPOINT.to_owned(),
            timeout: default_processor_timeout(),
            temperature: None,
            max_tokens: None,
            max_retries: 3,
        };
        let mut errors = Vec::new();
        config.validate_into(0, &mut errors);
        assert!(errors.iter().any(|e| e.field.contains("name")));
    }

    #[test]
    fn validate_empty_system_prompt() {
        let config = PostProcessorConfig {
            name: ProcessorName("Test".to_owned()),
            system_prompt: String::new(),
            api_key: Secret::from_string("test"),
            model: "gpt-4o-mini".to_owned(),
            endpoint: DEFAULT_CHAT_ENDPOINT.to_owned(),
            timeout: default_processor_timeout(),
            temperature: None,
            max_tokens: None,
            max_retries: 3,
        };
        let mut errors = Vec::new();
        config.validate_into(0, &mut errors);
        assert!(errors.iter().any(|e| e.field.contains("system_prompt")));
    }

    #[test]
    fn validate_bad_temperature() {
        let config = PostProcessorConfig {
            name: ProcessorName("Test".to_owned()),
            system_prompt: "Fix.".to_owned(),
            api_key: Secret::from_string("test"),
            model: "gpt-4o-mini".to_owned(),
            endpoint: DEFAULT_CHAT_ENDPOINT.to_owned(),
            timeout: default_processor_timeout(),
            temperature: Some(3.0),
            max_tokens: None,
            max_retries: 3,
        };
        let mut errors = Vec::new();
        config.validate_into(0, &mut errors);
        assert!(errors.iter().any(|e| e.field.contains("temperature")));
    }

    #[test]
    fn validate_excessive_max_retries() {
        let config = PostProcessorConfig {
            name: ProcessorName("Test".to_owned()),
            system_prompt: "Fix.".to_owned(),
            api_key: Secret::from_string("test"),
            model: "gpt-4o-mini".to_owned(),
            endpoint: DEFAULT_CHAT_ENDPOINT.to_owned(),
            timeout: default_processor_timeout(),
            temperature: None,
            max_tokens: None,
            max_retries: 11,
        };
        let mut errors = Vec::new();
        config.validate_into(0, &mut errors);
        assert!(errors.iter().any(|e| e.field.contains("max_retries")));
    }

    #[test]
    fn validate_zero_max_tokens() {
        let config = PostProcessorConfig {
            name: ProcessorName("Test".to_owned()),
            system_prompt: "Fix.".to_owned(),
            api_key: Secret::from_string("test"),
            model: "gpt-4o-mini".to_owned(),
            endpoint: DEFAULT_CHAT_ENDPOINT.to_owned(),
            timeout: default_processor_timeout(),
            temperature: None,
            max_tokens: Some(0),
            max_retries: 3,
        };
        let mut errors = Vec::new();
        config.validate_into(0, &mut errors);
        assert!(errors.iter().any(|e| e.field.contains("max_tokens")));
    }
}

use std::fmt::Write;
use std::path::PathBuf;

/// Errors during secret resolution.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SecretError {
    /// Environment variable not set.
    #[error("Environment variable '{0}' is not set")]
    EnvVarNotSet(String),

    /// Command execution failed.
    #[error("Secret command failed: {0}")]
    CommandFailed(String),

    /// Internal cache error.
    #[error("Secret cache error: {0}")]
    CacheError(String),
}

/// Errors during audio recording.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RecordingError {
    /// No microphone device found on the system.
    #[error("No microphone detected. Please connect a microphone and try again")]
    NoMicrophone,

    /// Microphone was disconnected during recording.
    #[error("Microphone disconnected during recording")]
    DeviceDisconnected,

    /// General device error.
    #[error("Audio device error: {0}")]
    DeviceError(String),
}

/// Errors during speech-to-text transcription.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum TranscriptionError {
    /// Network connectivity issue.
    #[error("Network error: {0}. Check your internet connection and try again")]
    NetworkError(String),

    /// Invalid or missing API key.
    #[error("Authentication failed. Check your API key in the configuration file")]
    AuthenticationError,

    /// Provider returned an error response.
    #[error("Provider error (HTTP {status}): {message}")]
    ProviderError {
        /// HTTP status code.
        status: u16,
        /// Error message from provider.
        message: String,
    },

    /// Request timed out.
    #[error("Transcription timed out. Try again or check your network connection")]
    Timeout,

    /// Audio contained no speech (silence only).
    #[error("No speech detected in the recording. Please speak louder or check your microphone")]
    EmptyAudio,

    /// Template expansion failed in the provider prompt.
    #[error("Template expansion failed: {0}")]
    TemplateExpansionError(#[from] crate::template::TemplateError),
}

impl From<crate::http::TransportError> for TranscriptionError {
    fn from(e: crate::http::TransportError) -> Self {
        match e {
            crate::http::TransportError::Timeout => Self::Timeout,
            crate::http::TransportError::Network(msg) => Self::NetworkError(msg),
        }
    }
}

/// Errors during configuration loading and validation.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ConfigError {
    /// Configuration file not found.
    #[error("Configuration file not found: {0}")]
    FileNotFound(PathBuf),

    /// YAML parse error.
    #[error("Configuration parse error at line {line}: {message}")]
    ParseError {
        /// Line number in the config file.
        line: usize,
        /// Description of the parse error.
        message: String,
    },

    /// One or more validation errors.
    #[error("{}", format_validation_errors(.0))]
    ValidationErrors(Vec<ValidationError>),

    /// Secret could not be resolved.
    #[error("Failed to resolve secret for '{field}': {reason}")]
    SecretResolutionError {
        /// Config field path.
        field: String,
        /// Underlying error description.
        reason: String,
    },

    /// I/O error reading the file.
    #[error("Failed to read configuration file: {0}")]
    IoError(#[from] std::io::Error),
}

fn format_validation_errors(errors: &[ValidationError]) -> String {
    let mut s = format!("Configuration has {} validation error(s):", errors.len());
    for e in errors {
        let _ = write!(s, "\n  - {e}");
    }
    s
}

/// Errors during text post-processing pipeline execution.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PostProcessingError {
    /// Network connectivity issue.
    #[error("Post-processing network error: {0}")]
    NetworkError(String),

    /// Invalid or missing API key.
    #[error("Post-processing authentication failed. Check the API key for this processor")]
    AuthenticationError,

    /// Provider returned an error response.
    #[error("Post-processing provider error (HTTP {status}): {message}")]
    ProviderError {
        /// HTTP status code.
        status: u16,
        /// Error message from provider.
        message: String,
    },

    /// Request timed out.
    #[error("Post-processing request timed out")]
    Timeout,

    /// Provider returned an empty response.
    #[error("Post-processing returned an empty response")]
    EmptyResponse,

    /// Template expansion failed in the system prompt.
    #[error("Post-processing template expansion failed: {0}")]
    TemplateExpansionError(#[from] crate::template::TemplateError),
}

impl From<crate::http::TransportError> for PostProcessingError {
    fn from(e: crate::http::TransportError) -> Self {
        match e {
            crate::http::TransportError::Timeout => Self::Timeout,
            crate::http::TransportError::Network(msg) => Self::NetworkError(msg),
        }
    }
}

impl PostProcessingError {
    /// Returns `true` if this error is transient and the request can be retried.
    ///
    /// Retryable: [`NetworkError`](Self::NetworkError), [`Timeout`](Self::Timeout),
    /// [`ProviderError`](Self::ProviderError) with status 429 or >= 500.
    ///
    /// Not retryable: [`AuthenticationError`](Self::AuthenticationError),
    /// [`EmptyResponse`](Self::EmptyResponse),
    /// [`TemplateExpansionError`](Self::TemplateExpansionError), other 4xx status codes.
    #[must_use]
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::NetworkError(_) | Self::Timeout => true,
            Self::ProviderError { status, .. } => *status == 429 || *status >= 500,
            Self::AuthenticationError | Self::EmptyResponse | Self::TemplateExpansionError(_) => {
                false
            }
        }
    }
}

/// A single validation error with suggestion.
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// Dotted field path (e.g., "`provider.api_key`").
    pub field: String,
    /// Description of what's wrong.
    pub message: String,
    /// Suggested fix.
    pub suggestion: Option<String>,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.field, self.message)?;
        if let Some(ref suggestion) = self.suggestion {
            write!(f, "\n    Suggestion: {suggestion}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recording_error_display() {
        let err = RecordingError::NoMicrophone;
        assert!(err.to_string().contains("microphone"));
    }

    #[test]
    fn transcription_error_display() {
        let err = TranscriptionError::ProviderError {
            status: 429,
            message: "Rate limit exceeded".to_owned(),
        };
        assert!(err.to_string().contains("429"));
        assert!(err.to_string().contains("Rate limit"));
    }

    #[test]
    fn validation_error_display() {
        let err = ValidationError {
            field: "provider.api_key".to_owned(),
            message: "missing required field".to_owned(),
            suggestion: Some("Add `api_key: !FromEnv OPENAI_API_KEY`".to_owned()),
        };
        let s = err.to_string();
        assert!(s.contains("provider.api_key"));
        assert!(s.contains("Suggestion"));
    }

    #[test]
    fn post_processing_error_display() {
        let err = PostProcessingError::ProviderError {
            status: 429,
            message: "Rate limit exceeded".to_owned(),
        };
        assert!(err.to_string().contains("429"));
        assert!(err.to_string().contains("Rate limit"));

        let err = PostProcessingError::EmptyResponse;
        assert!(err.to_string().contains("empty response"));

        let err = PostProcessingError::AuthenticationError;
        assert!(err.to_string().contains("authentication"));
    }

    #[test]
    fn retryable_network_error() {
        let err = PostProcessingError::NetworkError("connection reset".to_owned());
        assert!(err.is_retryable());
    }

    #[test]
    fn retryable_timeout() {
        assert!(PostProcessingError::Timeout.is_retryable());
    }

    #[test]
    fn retryable_provider_429() {
        let err = PostProcessingError::ProviderError {
            status: 429,
            message: "rate limit".to_owned(),
        };
        assert!(err.is_retryable());
    }

    #[test]
    fn retryable_provider_500() {
        let err = PostProcessingError::ProviderError {
            status: 500,
            message: "internal server error".to_owned(),
        };
        assert!(err.is_retryable());
    }

    #[test]
    fn retryable_provider_503() {
        let err = PostProcessingError::ProviderError {
            status: 503,
            message: "service unavailable".to_owned(),
        };
        assert!(err.is_retryable());
    }

    #[test]
    fn not_retryable_authentication() {
        assert!(!PostProcessingError::AuthenticationError.is_retryable());
    }

    #[test]
    fn not_retryable_provider_400() {
        let err = PostProcessingError::ProviderError {
            status: 400,
            message: "bad request".to_owned(),
        };
        assert!(!err.is_retryable());
    }

    #[test]
    fn not_retryable_empty_response() {
        assert!(!PostProcessingError::EmptyResponse.is_retryable());
    }

    #[test]
    fn not_retryable_template_expansion() {
        let err = PostProcessingError::TemplateExpansionError(
            crate::template::TemplateError::CommandFailed {
                command: "test".to_owned(),
                reason: "failed".to_owned(),
            },
        );
        assert!(!err.is_retryable());
    }

    #[test]
    fn config_error_validation_count() {
        let err = ConfigError::ValidationErrors(vec![
            ValidationError {
                field: "a".to_owned(),
                message: "bad".to_owned(),
                suggestion: None,
            },
            ValidationError {
                field: "b".to_owned(),
                message: "bad".to_owned(),
                suggestion: None,
            },
        ]);
        assert!(err.to_string().contains("2 validation error"));
    }
}

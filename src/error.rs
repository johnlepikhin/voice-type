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

/// Errors during hotkey listener operation.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum HotkeyError {
    /// Invalid hotkey binding string.
    #[error("Invalid hotkey binding: {0}")]
    InvalidBinding(String),

    /// Failed to start the hotkey listener.
    #[error("Failed to start hotkey listener: {0}")]
    ListenerFailed(String),
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
}

/// Errors during text insertion into the target window.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum TextInsertionError {
    /// Clipboard tool not available.
    #[error("Clipboard tool not available. Install xclip (X11) or wl-copy (Wayland)")]
    ClipboardUnavailable,

    /// Paste simulation failed.
    #[error("Failed to simulate paste. Install xdotool (X11) or wtype (Wayland)")]
    PasteSimulationFailed,

    /// Target window no longer exists.
    #[error("The target window is no longer available. Text has been copied to clipboard")]
    TargetWindowGone,

    /// Unsupported display session type.
    #[error("Unsupported session type. Set $XDG_SESSION_TYPE to 'x11' or 'wayland'")]
    UnsupportedSessionType,
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

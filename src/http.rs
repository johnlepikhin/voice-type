use std::time::Duration;

use serde::Deserialize;

/// Maximum idle age for pooled HTTP connections.
///
/// Prevents reuse of stale TCP connections that may have been closed
/// by upstream proxies (e.g., Cloudflare) before ureq's default idle timeout.
pub const HTTP_IDLE_TIMEOUT: Duration = Duration::from_secs(5);

/// Error response from `OpenAI`-compatible APIs.
#[derive(Deserialize)]
struct ApiErrorResponse {
    error: ApiErrorDetail,
}

/// Inner error detail from `OpenAI`-compatible APIs.
#[derive(Deserialize)]
struct ApiErrorDetail {
    message: String,
}

/// Extract error message from an `OpenAI`-compatible JSON error body.
///
/// Returns the parsed `error.message` field, or the raw body string if parsing fails.
#[must_use]
pub fn extract_api_error(body: &str) -> String {
    serde_json::from_str::<ApiErrorResponse>(body)
        .map_or_else(|_| body.to_owned(), |r| r.error.message)
}

/// Classified transport-level error from ureq.
///
/// Shared between transcription and post-processing modules.
/// Convert into domain-specific error types via `From` impls.
#[derive(Debug)]
#[non_exhaustive]
pub enum TransportError {
    /// Request timed out.
    Timeout,
    /// Network-level failure (DNS, connection reset, etc.).
    Network(String),
}

impl From<ureq::Error> for TransportError {
    fn from(error: ureq::Error) -> Self {
        match error {
            ureq::Error::Timeout(_) => Self::Timeout,
            other => Self::Network(other.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_api_error_valid_json() {
        let body = r#"{"error":{"message":"Rate limit exceeded","type":"rate_limit"}}"#;
        assert_eq!(extract_api_error(body), "Rate limit exceeded");
    }

    #[test]
    fn extract_api_error_invalid_json() {
        let body = "Internal Server Error";
        assert_eq!(extract_api_error(body), "Internal Server Error");
    }

    #[test]
    fn extract_api_error_empty_body() {
        assert_eq!(extract_api_error(""), "");
    }

    #[test]
    fn transport_error_from_transcription() {
        let err = TransportError::Network("connection reset".to_owned());
        let te: crate::error::TranscriptionError = err.into();
        assert!(
            matches!(te, crate::error::TranscriptionError::NetworkError(msg) if msg == "connection reset")
        );
    }

    #[test]
    fn transport_error_from_postprocessing() {
        let err = TransportError::Timeout;
        let pe: crate::error::PostProcessingError = err.into();
        assert!(matches!(pe, crate::error::PostProcessingError::Timeout));
    }
}

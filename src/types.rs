use std::fmt;

use serde::{Deserialize, Serialize};
use structdoc::{Documentation, StructDoc};

/// Audio RMS level normalized to 0.0..=1.0.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RmsLevel(f32);

impl RmsLevel {
    /// Create a new RMS level, clamping to [0.0, 1.0].
    #[must_use]
    pub fn new(value: f32) -> Self {
        Self(value.clamp(0.0, 1.0))
    }

    /// Get the raw f32 value.
    #[must_use]
    pub fn value(self) -> f32 {
        self.0
    }
}

impl fmt::Display for RmsLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.2}", self.0)
    }
}

impl Default for RmsLevel {
    fn default() -> Self {
        Self(0.0)
    }
}

/// Transcribed text from provider (immutable original).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TranscribedText(String);

impl TranscribedText {
    /// Create new transcribed text.
    #[must_use]
    pub fn new(text: String) -> Self {
        Self(text)
    }

    /// Get the text content.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TranscribedText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<String> for TranscribedText {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// User-edited text ready for insertion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfirmedText(String);

impl ConfirmedText {
    /// Create new confirmed text.
    #[must_use]
    pub fn new(text: String) -> Self {
        Self(text)
    }

    /// Get the text content.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ConfirmedText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<String> for ConfirmedText {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<TranscribedText> for ConfirmedText {
    fn from(t: TranscribedText) -> Self {
        Self(t.0)
    }
}

/// ISO-639-1 language code (e.g., "ru", "en").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanguageCode(String);

impl LanguageCode {
    /// Create a new language code. Must be a 2-letter lowercase string.
    ///
    /// # Errors
    /// Returns error if the code is not exactly 2 lowercase ASCII letters.
    pub fn new(code: &str) -> Result<Self, String> {
        if code.len() == 2 && code.chars().all(|c| c.is_ascii_lowercase()) {
            Ok(Self(code.to_owned()))
        } else {
            Err(format!(
                "Invalid ISO-639-1 language code: {code:?}. Expected 2 lowercase letters (e.g., \"en\", \"ru\")"
            ))
        }
    }

    /// Get the code as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for LanguageCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl StructDoc for LanguageCode {
    fn document() -> Documentation {
        Documentation::leaf("ISO-639-1 language code (e.g., \"en\", \"ru\")")
    }
}

/// Hotkey combination (e.g., "Shift+F8").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HotkeyBinding(String);

impl HotkeyBinding {
    /// Create a new hotkey binding from a string like "Shift+F8".
    ///
    /// # Errors
    /// Returns error if the binding is empty.
    pub fn new(binding: &str) -> Result<Self, String> {
        if binding.is_empty() {
            return Err("Hotkey binding cannot be empty".to_owned());
        }
        Ok(Self(binding.to_owned()))
    }

    /// Get the binding as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for HotkeyBinding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl StructDoc for HotkeyBinding {
    fn document() -> Documentation {
        Documentation::leaf(
            "Hotkey combination (e.g., \"Shift+F8\", \"Super+F9\", \"Ctrl+Alt+Delete\")",
        )
    }
}

impl Default for HotkeyBinding {
    fn default() -> Self {
        Self("Shift+F8".to_owned())
    }
}

/// Audio sample rate in Hz.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SampleRate(u32);

impl SampleRate {
    /// Whisper-optimal sample rate.
    pub const WHISPER_OPTIMAL: Self = Self(16_000);

    /// Create a new sample rate.
    ///
    /// # Errors
    /// Returns error if the rate is outside 8000..=48000 Hz.
    pub fn new(hz: u32) -> Result<Self, String> {
        if (8_000..=48_000).contains(&hz) {
            Ok(Self(hz))
        } else {
            Err(format!(
                "Sample rate {hz} Hz out of range. Must be 8000..=48000"
            ))
        }
    }

    /// Get the raw Hz value.
    #[must_use]
    pub fn hz(self) -> u32 {
        self.0
    }
}

impl fmt::Display for SampleRate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} Hz", self.0)
    }
}

impl Default for SampleRate {
    fn default() -> Self {
        Self::WHISPER_OPTIMAL
    }
}

impl StructDoc for SampleRate {
    fn document() -> Documentation {
        Documentation::leaf("Audio sample rate in Hz (8000..=48000)")
    }
}

impl StructDoc for RmsLevel {
    fn document() -> Documentation {
        Documentation::leaf("RMS level (0.0..=1.0)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rms_level_clamps() {
        assert!((RmsLevel::new(1.5).value() - 1.0).abs() < f32::EPSILON);
        assert!((RmsLevel::new(-0.5).value() - 0.0).abs() < f32::EPSILON);
        assert!((RmsLevel::new(0.5).value() - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn language_code_validation() {
        assert!(LanguageCode::new("en").is_ok());
        assert!(LanguageCode::new("ru").is_ok());
        assert!(LanguageCode::new("EN").is_err());
        assert!(LanguageCode::new("eng").is_err());
        assert!(LanguageCode::new("").is_err());
        assert!(LanguageCode::new("1a").is_err());
    }

    #[test]
    fn hotkey_binding_validation() {
        assert!(HotkeyBinding::new("Shift+F8").is_ok());
        assert!(HotkeyBinding::new("Ctrl+Shift+R").is_ok());
        assert!(HotkeyBinding::new("").is_err());
    }

    #[test]
    fn sample_rate_validation() {
        assert!(SampleRate::new(16_000).is_ok());
        assert!(SampleRate::new(8_000).is_ok());
        assert!(SampleRate::new(48_000).is_ok());
        assert!(SampleRate::new(0).is_err());
        assert!(SampleRate::new(100_000).is_err());
    }

    #[test]
    fn confirmed_text_from_transcribed() {
        let t = TranscribedText::new("hello".to_owned());
        let c = ConfirmedText::from(t);
        assert_eq!(c.as_str(), "hello");
    }
}

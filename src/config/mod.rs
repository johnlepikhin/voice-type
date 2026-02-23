pub mod secret;

use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use structdoc::StructDoc;

use crate::error::{ConfigError, ValidationError};
use crate::postprocess::config::PostProcessorConfig;
use crate::types::{LanguageCode, RmsLevel, SampleRate};

pub use secret::Secret;

/// Root application configuration.
#[derive(Debug, Clone, Serialize, Deserialize, StructDoc)]
pub struct AppConfig {
    /// Speech-to-text provider configuration.
    pub provider: ProviderConfig,

    /// Audio capture settings.
    #[serde(default)]
    pub audio: AudioConfig,

    /// Post-processing pipeline (zero or more processors).
    #[serde(default)]
    pub post_processing: Vec<PostProcessorConfig>,
}

/// Provider configuration (externally tagged enum).
///
/// Each variant wraps provider-specific settings. Adding a new provider
/// means adding a new variant with its own config struct.
///
/// ```yaml
/// provider:
///   openai:
///     api_key: !FromEnv OPENAI_API_KEY
///     model: whisper-1
/// ```
#[derive(Debug, Clone, StructDoc)]
#[non_exhaustive]
pub enum ProviderConfig {
    /// `OpenAI` Whisper API.
    OpenAi(OpenAiProviderConfig),
}

// Custom serde: serialize/deserialize as YAML map `{ openai: { ... } }` (externally tagged).
impl Serialize for ProviderConfig {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(1))?;
        match self {
            Self::OpenAi(config) => map.serialize_entry("openai", config)?,
        }
        map.end()
    }
}

/// Helper for map-based deserialization.
#[derive(Deserialize)]
struct ProviderConfigHelper {
    openai: Option<OpenAiProviderConfig>,
}

impl<'de> Deserialize<'de> for ProviderConfig {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let helper = ProviderConfigHelper::deserialize(deserializer)?;
        if let Some(config) = helper.openai {
            Ok(Self::OpenAi(config))
        } else {
            Err(serde::de::Error::custom(
                "no recognized provider; expected one of: openai",
            ))
        }
    }
}

/// `OpenAI` Whisper provider settings.
#[derive(Debug, Clone, Serialize, Deserialize, StructDoc)]
pub struct OpenAiProviderConfig {
    /// API key (secret).
    pub api_key: Secret,

    /// Model name (e.g., "whisper-1").
    #[serde(default = "default_model")]
    pub model: String,

    /// Language hint (ISO-639-1, e.g., "ru", "en").
    #[serde(default)]
    pub language: Option<LanguageCode>,

    /// System prompt for recognition context.
    #[serde(default)]
    pub prompt: Option<String>,

    /// Request timeout.
    #[serde(default = "default_timeout", with = "humantime_serde")]
    pub timeout: Duration,
}

impl ProviderConfig {
    /// Validate provider-specific fields, appending errors.
    fn validate_into(&self, errors: &mut Vec<ValidationError>) {
        match self {
            Self::OpenAi(c) => {
                if c.model.is_empty() {
                    errors.push(ValidationError {
                        field: "provider.openai.model".to_owned(),
                        message: "model cannot be empty".to_owned(),
                        suggestion: Some("Use \"whisper-1\"".to_owned()),
                    });
                }
            }
        }
    }
}

fn default_model() -> String {
    "whisper-1".to_owned()
}

fn default_timeout() -> Duration {
    Duration::from_secs(30)
}

/// Audio capture settings.
#[derive(Debug, Clone, Serialize, Deserialize, StructDoc)]
pub struct AudioConfig {
    /// Input device name (None = system default).
    #[serde(default)]
    pub device: Option<String>,

    /// Sample rate in Hz.
    #[serde(default = "default_sample_rate")]
    pub sample_rate: SampleRate,

    /// Silence detection threshold (RMS, 0.0..=1.0).
    #[serde(default = "default_silence_threshold")]
    pub silence_threshold: RmsLevel,

    /// Maximum recording duration.
    #[serde(default = "default_max_duration", with = "humantime_serde")]
    pub max_duration: Duration,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            device: None,
            sample_rate: default_sample_rate(),
            silence_threshold: default_silence_threshold(),
            max_duration: default_max_duration(),
        }
    }
}

fn default_sample_rate() -> SampleRate {
    SampleRate::WHISPER_OPTIMAL
}

fn default_silence_threshold() -> RmsLevel {
    RmsLevel::new(0.01)
}

fn default_max_duration() -> Duration {
    Duration::from_secs(300) // 5min
}

impl AppConfig {
    /// Load configuration from a YAML file.
    ///
    /// # Errors
    /// Returns `ConfigError` if the file is missing, unparseable, or invalid.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        if !path.exists() {
            return Err(ConfigError::FileNotFound(path.to_path_buf()));
        }
        let content = std::fs::read_to_string(path)?;
        let config: Self = serde_yaml::from_str(&content).map_err(|e| {
            let location = e.location();
            ConfigError::ParseError {
                line: location.map_or(0, |l| l.line()),
                message: e.to_string(),
            }
        })?;
        Ok(config)
    }

    /// Validate all configuration fields, collecting ALL errors.
    ///
    /// # Errors
    /// Returns `ConfigError::ValidationErrors` if any fields are invalid.
    pub fn validate(&self) -> Result<(), ConfigError> {
        let mut errors = Vec::new();

        // Validate provider
        self.provider.validate_into(&mut errors);

        // Validate post-processing
        for (i, processor) in self.post_processing.iter().enumerate() {
            processor.validate_into(i, &mut errors);
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(ConfigError::ValidationErrors(errors))
        }
    }

    /// Print `StructDoc` documentation tree for the configuration.
    #[must_use]
    pub fn docs() -> String {
        use structdoc::StructDoc;
        Self::document().to_string()
    }

    /// Generate a default configuration file content with comments.
    #[must_use]
    pub fn default_yaml() -> String {
        r#"# Voice Type configuration
# See `voice-type config docs` for full documentation.

provider:
  openai:
    # API key: use !FromEnv, !FromCommand, or !String
    # Examples:
    #   api_key: !FromEnv OPENAI_API_KEY
    #   api_key: !FromCommand "pass show openai/api-key"
    #   api_key: !String "sk-..."
    api_key: !FromEnv OPENAI_API_KEY
    model: whisper-1
    # language: ru            # ISO-639-1 hint for recognition
    # prompt: ""              # Context hint for recognition style
    timeout: 30s

audio:
  # device:                 # Input device name (omit for system default)
  sample_rate: 16000
  silence_threshold: 0.01
  max_duration: 5min

# post_processing:             # Optional: LLM post-processing pipeline
#   - name: Grammar
#     system_prompt: "Fix grammar. Current time: {{datetime}}"
#     # Templates: {{datetime}}, {{date}}, {{time}}, {{$(shell command)}}
#     api_key: !FromEnv OPENAI_API_KEY
#     model: gpt-4o-mini
#     # endpoint: https://api.openai.com
#     # timeout: 15s
#     # temperature: 0.3
#     # max_tokens: 2048
#     # max_retries: 3
"#
        .to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_config_yaml() -> &'static str {
        r"
provider:
  openai:
    api_key: !FromEnv OPENAI_API_KEY
    model: whisper-1
    timeout: 30s

audio:
  sample_rate: 16000
  silence_threshold: 0.01
  max_duration: 5min
"
    }

    #[test]
    fn parse_config() {
        let config: AppConfig = serde_yaml::from_str(sample_config_yaml()).unwrap();
        match &config.provider {
            ProviderConfig::OpenAi(c) => {
                assert_eq!(c.model, "whisper-1");
                assert_eq!(c.timeout, Duration::from_secs(30));
            }
        }
        assert_eq!(config.audio.sample_rate.hz(), 16_000);
    }

    #[test]
    fn validate_valid_config() {
        let config: AppConfig = serde_yaml::from_str(sample_config_yaml()).unwrap();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn bad_sample_rate_rejected_at_parse() {
        let yaml = r"
provider:
  openai:
    api_key: !FromEnv OPENAI_API_KEY
audio:
  sample_rate: 0
";
        let result = serde_yaml::from_str::<AppConfig>(yaml);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("Sample rate"), "unexpected error: {msg}");
    }

    #[test]
    fn default_yaml_parses() {
        let yaml = AppConfig::default_yaml();
        let config: AppConfig = serde_yaml::from_str(&yaml).unwrap();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn provider_config_external_tag() {
        let yaml = r#"
openai:
  api_key: !String "test"
"#;
        let config: ProviderConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(matches!(config, ProviderConfig::OpenAi(_)));
    }
}

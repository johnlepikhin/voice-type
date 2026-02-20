# Data Model: Core Voice Input

**Feature**: 001-core-voice-input
**Date**: 2026-02-20

## Domain Types

### RecordingStatus

State machine for audio recording lifecycle.

```rust
#[non_exhaustive]
pub enum RecordingStatus {
    Idle,
    Recording { started_at: Instant, rms_level: RmsLevel },
    Processing,
    Completed { duration: Duration },
    Failed { error: RecordingError },
}
```

**Transitions**:
- `Idle` → `Recording` (user presses record / hotkey)
- `Recording` → `Processing` (user stops recording)
- `Recording` → `Failed` (microphone disconnected, error)
- `Processing` → `Completed` (audio saved to buffer)
- `Processing` → `Failed` (encoding error)

### TranscriptionStatus

State machine for transcription lifecycle.

```rust
#[non_exhaustive]
pub enum TranscriptionStatus {
    Pending,
    Transcribing,
    Succeeded { text: TranscribedText },
    Failed { error: TranscriptionError },
}
```

**Transitions**:
- `Pending` → `Transcribing` (audio sent to provider)
- `Transcribing` → `Succeeded` (provider returns text)
- `Transcribing` → `Failed` (network error, API error, timeout)

### DaemonPhase

Runtime state of the background service.

```rust
#[non_exhaustive]
pub enum DaemonPhase {
    Idle,
    Recording { started_at: Instant },
    Transcribing,
    AwaitingConfirmation { text: TranscribedText },
}
```

**Transitions**:
- `Idle` → `Recording` (hotkey pressed)
- `Recording` → `Transcribing` (hotkey pressed again / stop)
- `Transcribing` → `AwaitingConfirmation` (transcription succeeded)
- `Transcribing` → `Idle` (transcription failed, error shown)
- `AwaitingConfirmation` → `Idle` (user confirms or cancels)

## Newtypes

```rust
/// Audio RMS level normalized to 0.0..=1.0
pub struct RmsLevel(f32);

/// Transcribed text from provider (immutable original)
pub struct TranscribedText(String);

/// User-edited text ready for insertion
pub struct ConfirmedText(String);

/// ISO-639-1 language code (e.g., "ru", "en")
pub struct LanguageCode(String);

/// Hotkey combination (e.g., "Super+V")
pub struct HotkeyBinding(String);

/// Application ID for D-Bus registration
pub struct ApplicationId(String);

/// Audio sample rate in Hz
pub struct SampleRate(u32);
```

## Value Objects

### AudioData

Captured audio buffer ready for transcription.

```rust
pub struct AudioData {
    pub samples: Vec<i16>,
    pub sample_rate: SampleRate,
    pub channels: u16,
    pub duration: Duration,
}
```

**Validation**:
- `sample_rate` must be 16000 (Whisper optimal)
- `channels` must be 1 (mono)
- `samples` must not be empty
- `duration` must be > 0 and < 25 MB equivalent

### TranscribeOptions

Options passed to the transcription provider.

```rust
pub struct TranscribeOptions {
    pub language: Option<LanguageCode>,
    pub prompt: Option<String>,
}
```

### TranscriptionResult

Result returned from a provider.

```rust
pub struct TranscriptionResult {
    pub text: TranscribedText,
    pub duration: Duration,
}
```

## Error Types

```rust
#[non_exhaustive]
pub enum RecordingError {
    NoMicrophone,
    DeviceDisconnected,
    DeviceError(String),
}

#[non_exhaustive]
pub enum TranscriptionError {
    NetworkError(String),
    AuthenticationError,
    ProviderError { status: u16, message: String },
    Timeout,
    EmptyAudio,
}

#[non_exhaustive]
pub enum TextInsertionError {
    ClipboardUnavailable,
    PasteSimulationFailed,
    TargetWindowGone,
    UnsupportedSessionType,
}

#[non_exhaustive]
pub enum ConfigError {
    FileNotFound(PathBuf),
    ParseError { line: usize, message: String },
    ValidationErrors(Vec<ValidationError>),
    SecretResolutionError { field: String, source: String },
}

pub struct ValidationError {
    pub field: String,
    pub message: String,
    pub suggestion: Option<String>,
}
```

## Configuration Model

### AppConfig (root)

```rust
#[derive(Serialize, Deserialize, StructDoc)]
pub struct AppConfig {
    /// Speech-to-text provider configuration
    pub provider: ProviderConfig,

    /// Audio capture settings
    #[serde(default)]
    pub audio: AudioConfig,

    /// Hotkey binding for daemon mode
    #[serde(default)]
    pub hotkey: HotkeyConfig,

    /// UI preferences
    #[serde(default)]
    pub ui: UiConfig,
}
```

### ProviderConfig

```rust
#[derive(Serialize, Deserialize, StructDoc)]
#[serde(tag = "type")]
#[non_exhaustive]
pub enum ProviderConfig {
    #[serde(rename = "openai")]
    OpenAi(OpenAiConfig),
}

#[derive(Serialize, Deserialize, StructDoc)]
pub struct OpenAiConfig {
    /// API key (secret)
    pub api_key: Secret,

    /// Model name
    #[serde(default = "default_model")]
    pub model: String, // "whisper-1"

    /// Language hint (ISO-639-1)
    pub language: Option<LanguageCode>,

    /// System prompt for recognition context
    pub prompt: Option<String>,

    /// Request timeout
    #[serde(default = "default_timeout")]
    #[serde(with = "humantime_serde")]
    pub timeout: Duration,
}
```

### AudioConfig

```rust
#[derive(Serialize, Deserialize, StructDoc)]
pub struct AudioConfig {
    /// Input device name (None = system default)
    pub device: Option<String>,

    /// Sample rate in Hz
    #[serde(default = "default_sample_rate")]
    pub sample_rate: u32, // 16000

    /// Silence detection threshold (RMS)
    #[serde(default = "default_silence_threshold")]
    pub silence_threshold: f32, // 0.01

    /// Maximum recording duration
    #[serde(default = "default_max_duration")]
    #[serde(with = "humantime_serde")]
    pub max_duration: Duration, // 5min
}
```

### HotkeyConfig

```rust
#[derive(Serialize, Deserialize, StructDoc)]
pub struct HotkeyConfig {
    /// Key combination for toggle recording
    #[serde(default = "default_hotkey")]
    pub binding: HotkeyBinding, // "Super+V"
}
```

### UiConfig

```rust
#[derive(Serialize, Deserialize, StructDoc)]
pub struct UiConfig {
    /// Overlay window position
    #[serde(default)]
    pub overlay_position: OverlayPosition,
}

#[derive(Serialize, Deserialize, StructDoc)]
#[non_exhaustive]
pub enum OverlayPosition {
    TopCenter,
    TopRight,
    BottomCenter,
    BottomRight,
    Center,
}
```

### Secret

```rust
#[derive(Serialize, Deserialize)]
pub enum Secret {
    /// Plaintext secret value
    String(SecUtf8String),

    /// Read from environment variable
    FromEnv(String),

    /// Read from shell command output
    FromCommand(String),
}
```

## Entity Relationships

```
AppConfig
├── ProviderConfig (tagged enum, one active)
│   └── OpenAiConfig
│       ├── Secret (api_key)
│       ├── LanguageCode? (language hint)
│       └── String? (prompt hint)
├── AudioConfig
├── HotkeyConfig
│   └── HotkeyBinding
└── UiConfig
    └── OverlayPosition

DaemonPhase (runtime state machine)
├── Recording → AudioData
├── Transcribing → TranscribeOptions + AudioData → TranscriptionResult
└── AwaitingConfirmation → TranscribedText → ConfirmedText

TranscriptionProvider (trait)
├── OpenAiWhisperProvider (impl)
└── [future providers]
```

## Default YAML Example

```yaml
provider:
  type: openai
  api_key: !FromEnv OPENAI_API_KEY
  model: whisper-1
  language: ru
  prompt: "Technical discussion about software development"
  timeout: 30s

audio:
  sample_rate: 16000
  silence_threshold: 0.01
  max_duration: 5min

hotkey:
  binding: Super+V

ui:
  overlay_position: TopCenter
```

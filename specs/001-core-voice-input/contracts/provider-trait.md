# Provider Trait Contract: TranscriptionProvider

**Feature**: 001-core-voice-input
**Date**: 2026-02-20

## Trait Definition

```rust
/// A speech-to-text transcription service.
///
/// Implementations MUST be `Send + Sync` to allow usage from
/// background threads. The `transcribe` method is blocking —
/// it runs on a dedicated thread, not the glib main loop.
pub trait TranscriptionProvider: Send + Sync {
    /// Transcribe audio data into text.
    ///
    /// # Errors
    /// Returns `TranscriptionError` on network, auth, or provider failures.
    fn transcribe(
        &self,
        audio: &AudioData,
        options: &TranscribeOptions,
    ) -> Result<TranscriptionResult, TranscriptionError>;
}
```

## Implementations

### OpenAiWhisperProvider

```rust
pub struct OpenAiWhisperProvider {
    api_key: Secret,
    model: String,
    endpoint: String,
    timeout: Duration,
}
```

**HTTP Contract**:
- Method: `POST`
- URL: `https://api.openai.com/v1/audio/transcriptions`
- Content-Type: `multipart/form-data`
- Authorization: `Bearer {api_key}`
- Form fields:
  - `file`: WAV audio data (required)
  - `model`: model name, e.g. `whisper-1` (required)
  - `language`: ISO-639-1 code (optional)
  - `prompt`: recognition hint (optional)
- Response: `{ "text": "transcribed text" }`
- Errors: HTTP 401 (auth), 413 (too large), 429 (rate limit), 500 (server)

## Extension Guide

To add a new provider:

1. Add variant to `ProviderConfig` enum in config model
2. Implement `TranscriptionProvider` trait
3. Register in provider factory (match on config variant)
4. Add config documentation via `StructDoc` derive

No changes needed in recording, UI, or text insertion code.

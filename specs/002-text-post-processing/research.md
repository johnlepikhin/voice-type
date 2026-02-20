# Research: Text Post-Processing Pipeline

## R-001: OpenAI Chat Completions API via ureq

**Decision**: Use ureq (already a dependency) to call the OpenAI Chat Completions API (`/v1/chat/completions`) with manual JSON body construction.

**Rationale**: The project already uses ureq v3 for the Whisper transcription endpoint. The chat completions API uses a simple JSON request/response (no multipart), making integration straightforward. No new HTTP dependencies needed.

**Alternatives considered**:
- **reqwest**: Would add tokio dependency, conflicting with the glib main loop architecture (Constitution IV).
- **Custom HTTP**: Unnecessary complexity for a JSON POST endpoint.

**API shape**:
```
POST /v1/chat/completions
{
  "model": "gpt-4o-mini",
  "messages": [
    {"role": "system", "content": "<system_prompt>"},
    {"role": "user", "content": "<text_to_process>"}
  ],
  "temperature": 0.3,
  "max_tokens": 2048
}
```
Response: `{ "choices": [{ "message": { "content": "..." } }] }`

## R-002: Pipeline Execution Model

**Decision**: Sequential execution on the same background thread as transcription. Each processor calls the chat completions API synchronously. Progress updates are sent to the GTK main loop via the existing `mpsc::channel` pattern.

**Rationale**: Constitution IV requires async operations to use `glib::MainContext` futures or channels. The existing transcription pattern (spawn thread → mpsc → `timeout_add_local` poll) works well and is proven. Post-processing is a natural extension of this pattern.

**Alternatives considered**:
- **Separate thread per processor**: Unnecessary since processors run sequentially. Would add complexity for no benefit.
- **glib async futures**: Would require significant refactoring of the existing transcription flow. Not worth it for a sequential pipeline.

## R-003: Progress Reporting to GTK

**Decision**: Use an enum-based message protocol over `mpsc::channel`. The background thread sends `PipelineProgress` messages that the GTK poll loop interprets to update the overlay.

**Rationale**: The existing pattern sends `Result<TranscriptionResult, TranscriptionError>` as a single shot. For multi-step progress, we need intermediate messages. An enum with variants `StepStarted { index, total, name }`, `StepCompleted { index }`, `Done { text }`, `Failed { error }` fits naturally.

**Alternatives considered**:
- **Multiple channels**: One per step — overcomplicated.
- **Shared atomic counter**: Not enough for step names; requires additional shared state.

## R-004: Configuration Structure

**Decision**: Add an optional `post_processing` list to `AppConfig`. Each entry is a struct with `name`, `system_prompt`, provider config (api_key, model, endpoint), `timeout`, and optional `temperature`/`max_tokens`.

**Rationale**: Follows the existing config pattern (YAML, serde, structdoc). The list is optional with default empty vec, preserving backward compatibility (FR-006). Each processor has independent provider config per clarification decision.

**Config example**:
```yaml
post_processing:
  - name: Grammar
    system_prompt: "Fix grammar and punctuation. Return only the corrected text."
    api_key: !FromEnv OPENAI_API_KEY
    model: gpt-4o-mini
    timeout: 15s
    temperature: 0.3
  - name: Translate
    system_prompt: "Translate the following text to English."
    api_key: !FromEnv OPENAI_API_KEY
    model: gpt-4o-mini
    timeout: 15s
```

## R-005: Error Handling Strategy

**Decision**: New `PostProcessingError` enum in `error.rs` with variants matching the existing error pattern (NetworkError, AuthenticationError, ProviderError, Timeout, EmptyResponse). The pipeline catches errors per-processor and falls back to the original text.

**Rationale**: Constitution V requires `thiserror` for library errors. The existing error types (TranscriptionError, RecordingError) follow this pattern. A new dedicated error type keeps concerns separated and allows processor-name-annotated error messages (FR-008).

**Alternatives considered**:
- **Reuse TranscriptionError**: Semantically wrong — post-processing is not transcription. Would also confuse error messages.
- **Generic anyhow in library**: Violates Constitution V.

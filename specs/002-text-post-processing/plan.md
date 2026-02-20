# Implementation Plan: Text Post-Processing Pipeline

**Branch**: `002-text-post-processing` | **Date**: 2026-02-20 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/002-text-post-processing/spec.md`

## Summary

Add a configurable text post-processing pipeline that runs after speech transcription and before text insertion. Users define zero or more LLM-powered processors in YAML config, each with a name, system prompt, and independent OpenAI provider settings. Processors execute sequentially on a background thread, with step-by-step progress displayed in the overlay. On failure, the system falls back to the original transcribed text.

## Technical Context

**Language/Version**: Rust 1.88+ (stable, edition 2021)
**Primary Dependencies**: ureq 3 (HTTP), serde/serde_yaml/serde_json (config/serialization), gtk4-rs (UI), thiserror (errors), structdoc (config docs)
**Storage**: N/A (config file only)
**Testing**: cargo test, proptest (property-based)
**Target Platform**: Linux (primary)
**Project Type**: Single project (library crate + binary crate)
**Performance Goals**: < 5s additional latency per processor for typical 1-2 sentence input (SC-001)
**Constraints**: No tokio; blocking HTTP on background thread; glib main loop for UI; library crate has zero GTK deps
**Scale/Scope**: 1-3 processors per pipeline typical; up to ~10 theoretically

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Algebraic Type Design | PASS | `ProcessorName` newtype, `PipelineProgress` enum, `PipelineResult` enum, `PostProcessingError` enum with `#[non_exhaustive]`. No raw primitives in public API. |
| II. Extensibility Through Composition | PASS | `PostProcessor` is a concrete type (not trait-based) since only one provider for MVP. The pipeline is a simple `Vec<PostProcessor>` — no need for trait polymorphism yet. When additional provider types are added, a `TextProcessor` trait can be introduced behind the existing concrete type. |
| III. Test-Driven Quality | PASS | Unit tests in module, integration tests for pipeline execution with mock HTTP, proptest for config roundtrip. |
| IV. GTK Architecture Discipline | PASS | Pipeline logic in library crate (`src/postprocess/`), zero GTK deps. GTK overlay update via mpsc channel + `timeout_add_local` polling. |
| V. Code Quality & Safety | PASS | `thiserror` for `PostProcessingError`, clippy pedantic, doc comments on all public items. `http_status_as_error(false)` pattern reused from transcription provider. |

No violations. Complexity Tracking table not needed.

## Project Structure

### Documentation (this feature)

```text
specs/002-text-post-processing/
├── plan.md              # This file
├── spec.md              # Feature specification
├── research.md          # Phase 0 research
├── data-model.md        # Phase 1 data model
├── quickstart.md        # Phase 1 quickstart
├── contracts/
│   ├── chat-completions.md   # OpenAI Chat Completions API contract
│   └── pipeline-messages.md  # Internal progress message protocol
└── tasks.md             # Phase 2 output (/speckit.tasks)
```

### Source Code (repository root)

```text
src/
├── lib.rs                    # Add: pub mod postprocess
├── postprocess/
│   ├── mod.rs                # PostProcessor, ProcessingPipeline, PipelineResult, PipelineProgress
│   ├── chat_completions.rs   # OpenAI chat completions HTTP client (ureq)
│   └── config.rs             # PostProcessorConfig, ProcessorName, validation
├── config/
│   └── mod.rs                # Add: post_processing field to AppConfig, validation
├── error.rs                  # Add: PostProcessingError enum
├── app/
│   ├── mod.rs                # Add: PostProcessing phase, pipeline integration
│   └── overlay.rs            # Add: show_processing(step, total, name) method
└── types.rs                  # Add: ProcessorName newtype (if not in postprocess::config)

tests/
├── postprocess_pipeline.rs   # Integration: mock server, pipeline execution
└── config_roundtrip.rs       # Add: post_processing config roundtrip test
```

**Structure Decision**: New `src/postprocess/` module in the library crate, following the existing pattern of `src/provider/`, `src/audio/`, `src/hotkey/`. Config types in `postprocess::config`, HTTP client in `postprocess::chat_completions`, pipeline logic in `postprocess::mod`. GTK integration in `src/app/mod.rs` (thin adapter).

## Design Decisions

### D-001: PostProcessor is a concrete struct, not a trait

For MVP with only OpenAI chat completions, a trait would be premature abstraction. A concrete `PostProcessor` struct with a `process(&self, text: &str) -> Result<String, PostProcessingError>` method is sufficient. When Anthropic or other providers are added, a `TextProcessor` trait can be extracted via refactoring.

### D-002: Pipeline runs on the transcription background thread

After transcription completes on the background thread, the pipeline runs immediately (same thread). A new `mpsc::Sender<PipelineProgress>` is passed to the pipeline, and the GTK poll loop receives progress updates. This avoids spawning additional threads and keeps the pattern consistent with the existing transcription flow.

### D-003: DaemonPhase extended with PostProcessing

The existing `DaemonPhase` enum gains a `PostProcessing` variant. The flow becomes:
`Idle → Recording → Transcribing → PostProcessing → AwaitingConfirmation`

When no processors are configured, `PostProcessing` is skipped.

### D-004: Config uses same Secret type for api_key

Post-processor `api_key` uses the existing `Secret` type (supporting `!FromEnv`, `!FromCommand`, `!String`). This means users can use different environment variables per processor or share the same one.

### D-005: http_status_as_error(false) pattern reused

Same approach as the transcription provider fix: disable ureq's auto-error, read response body, check status manually. This ensures error details from OpenAI are preserved in error messages.

### D-006: Empty post_processing list = backward compatible

`AppConfig.post_processing` defaults to `Vec::new()` via `#[serde(default)]`. Existing configs without this field continue to work unchanged.

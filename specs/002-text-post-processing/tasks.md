# Tasks: Text Post-Processing Pipeline

**Input**: Design documents from `/specs/002-text-post-processing/`
**Prerequisites**: plan.md, spec.md, data-model.md, research.md, contracts/, quickstart.md

**Tests**: Included — project constitution (Principle III) mandates test-driven quality.

**Organization**: Tasks grouped by user story for independent implementation and testing.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

---

## Phase 1: Setup

**Purpose**: Create module structure for post-processing pipeline

- [x] T001 Create postprocess module files: `src/postprocess/mod.rs`, `src/postprocess/config.rs`, `src/postprocess/chat_completions.rs`; register `pub mod postprocess` in `src/lib.rs`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Types, config, and HTTP client that ALL user stories depend on

**CRITICAL**: No user story work can begin until this phase is complete

- [x] T002 Add `PostProcessingError` enum (`NetworkError`, `AuthenticationError`, `ProviderError`, `Timeout`, `EmptyResponse`) with `#[non_exhaustive]` and `thiserror` in `src/error.rs`; add unit test for Display output
- [x] T003 [P] Add `ProcessorName` newtype (non-empty String, `Display`, `StructDoc`) and `PostProcessorConfig` struct (name, system_prompt, api_key, model, endpoint, timeout, temperature, max_tokens) with serde derives in `src/postprocess/config.rs`; add validation method that collects errors into `Vec<ValidationError>`
- [x] T004 [P] Implement `ChatCompletionsClient` struct in `src/postprocess/chat_completions.rs`: ureq Agent with `http_status_as_error(false)`, `send(system_prompt, user_text) -> Result<String, PostProcessingError>` method; add `ChatCompletionResponse`/`ChatChoice`/`ChatMessage` deserialization structs; reuse OpenAI error body extraction pattern from `src/provider/openai.rs`
- [x] T005 Add `post_processing: Vec<PostProcessorConfig>` field with `#[serde(default)]` to `AppConfig` in `src/config/mod.rs`; call `PostProcessorConfig::validate_into()` for each entry in `AppConfig::validate()`; add post_processing section to `default_yaml()` as commented-out example
- [x] T006 [P] Add config tests in `tests/config_roundtrip.rs`: YAML roundtrip with post_processing entries, validation rejects empty name, validation rejects empty system_prompt, config without post_processing parses (backward compat)

**Checkpoint**: Foundation ready — all types, config, and HTTP client available for user story implementation

---

## Phase 3: User Story 1 — Single Processor for Grammar Correction (Priority: P1) MVP

**Goal**: User configures one post-processor; transcribed text passes through it before insertion

**Independent Test**: Configure one processor, verify transcribed text is transformed by the processor before reaching the overlay confirmation view

### Implementation for User Story 1

- [x] T007 [US1] Add `PostProcessor` struct (constructed from `PostProcessorConfig`, holds `ChatCompletionsClient`) with `pub fn process(&self, text: &str) -> Result<String, PostProcessingError>` in `src/postprocess/mod.rs`; handle empty response → `PostProcessingError::EmptyResponse`
- [x] T008 [US1] Add `PipelineResult` enum (`Processed { text }`, `Skipped { text }`, `Failed { original_text, processor_name, error }`) and `ProcessingPipeline` struct (holds `Vec<PostProcessor>`) with `pub fn run(&self, text: &str) -> PipelineResult` in `src/postprocess/mod.rs`; empty pipeline → `Skipped`; single processor → `Processed` or `Failed`
- [x] T009 [US1] Add `build_pipeline(&self) -> ProcessingPipeline` method to `AppConfig` (or free function in `src/postprocess/mod.rs`) that constructs pipeline from `self.post_processing`
- [x] T010 [P] [US1] Add unit tests in `src/postprocess/mod.rs`: empty pipeline returns `Skipped`, single mock processor returns `Processed`, single failing processor returns `Failed` with original text
- [x] T011 [US1] Add `DaemonPhase::PostProcessing` variant to enum in `src/app/mod.rs`
- [x] T012 [US1] Add `show_processing(&self, step: usize, total: usize, name: &str)` method to `OverlayWindow` in `src/app/overlay.rs`: set status_label to "Step {step}/{total}: {name}...", show spinner, hide text_view/timer/level
- [x] T013 [US1] Integrate pipeline into daemon flow in `src/app/mod.rs`: after transcription succeeds, run pipeline on the background thread (extend the existing `std::thread::spawn` block); if `Processed` → show result; if `Skipped` → show result as-is; if `Failed` → show original text + log error; add `PostProcessing` phase handling in hotkey poll (ignore hotkey during post-processing)

**Checkpoint**: Single processor transforms text before insertion. No processors = unchanged behavior.

---

## Phase 4: User Story 2 — Sequential Multi-Processor Pipeline (Priority: P2)

**Goal**: Multiple processors execute in order, each receives previous output, progress shown per step

**Independent Test**: Configure 2+ processors, verify each processor's output feeds into the next, overlay shows "Step X/N" updates

### Implementation for User Story 2

- [x] T014 [US2] Add `PipelineProgress` enum (`StepStarted { index, total, name }`, `Done { text }`, `Failed { processor_name, error, original_text }`) in `src/postprocess/mod.rs`
- [x] T015 [US2] Extend `ProcessingPipeline::run()` to accept `&Sender<PipelineProgress>`, iterate processors sequentially sending `StepStarted` before each, send `Done` or `Failed` at end in `src/postprocess/mod.rs`; update US1 call site to pass sender
- [x] T016 [US2] Replace single-shot result channel with `PipelineProgress` channel in `src/app/mod.rs`: background thread sends progress messages; GTK poll loop matches on `StepStarted` → `overlay.show_processing()`, `Done` → `overlay.show_result()`, `Failed` → fallback + error
- [x] T017 [P] [US2] Add unit tests in `src/postprocess/mod.rs`: 3-processor pipeline produces `StepStarted` × 3 + `Done`; pipeline with failure at step 2 produces `StepStarted` × 2 + `Failed` with original text
- [x] T018 [P] [US2] Add integration test in `tests/postprocess_pipeline.rs`: construct pipeline from config, run with mock expectations, verify output is composition of all processors

**Checkpoint**: Multi-step pipeline works with per-step overlay progress. Single processor still works (US1 regression check).

---

## Phase 5: User Story 3 — Pipeline Error Visibility (Priority: P3)

**Goal**: Failed processor identified by name in error notification; user always gets original text on failure

**Independent Test**: Configure processor with invalid API key, verify error notification names the failing processor

### Implementation for User Story 3

- [x] T019 [US3] Enhance error display in overlay: when `PipelineProgress::Failed` is received, show error notification with format "Post-processing failed at '{processor_name}': {error}" in `src/app/mod.rs`; use `overlay.show_error()` with processor name included
- [x] T020 [US3] Ensure `DaemonPhase` transitions to `AwaitingConfirmation` (not `Idle`) on pipeline failure, showing original text in editable text view in `src/app/mod.rs`
- [x] T021 [P] [US3] Add unit tests in `src/postprocess/mod.rs`: verify `PipelineResult::Failed` contains correct `processor_name` from the failing step; verify original text is preserved through multi-step failure
- [x] T022 [P] [US3] Add integration test in `tests/postprocess_pipeline.rs`: pipeline with auth error at step 2 produces `Failed` with correct processor name and original text

**Checkpoint**: All 3 user stories complete. Error reporting is transparent and user always gets usable text.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Documentation, quality verification, final validation

- [x] T023 [P] Add `StructDoc` derives/impls for `PostProcessorConfig` and `ProcessorName` so `voice-type config docs` includes post_processing documentation in `src/postprocess/config.rs`
- [x] T024 [P] Add proptest for `PostProcessorConfig` YAML roundtrip (serialize → deserialize → equal) in `tests/config_roundtrip.rs`
- [x] T025 Run full verification: `cargo fmt -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo doc --no-deps` (all via guix shell)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — can start immediately
- **Foundational (Phase 2)**: Depends on Setup — BLOCKS all user stories
- **US1 (Phase 3)**: Depends on Foundational — MVP delivery point
- **US2 (Phase 4)**: Depends on US1 (extends pipeline API with progress channel)
- **US3 (Phase 5)**: Depends on US2 (uses PipelineProgress::Failed message)
- **Polish (Phase 6)**: Depends on all user stories complete

### User Story Dependencies

- **US1 (P1)**: Can start after Foundational (Phase 2). Independent — delivers single processor value.
- **US2 (P2)**: Depends on US1 — extends `ProcessingPipeline::run()` with progress sender parameter. Cannot start until T008 is complete.
- **US3 (P3)**: Depends on US2 — uses `PipelineProgress::Failed` message for error display. Cannot start until T015 is complete.

### Within Each User Story

- Library crate code before GTK adapter code
- Core types before methods
- Implementation before integration with daemon flow
- Tests alongside implementation (Constitution III)

### Parallel Opportunities

**Phase 2** (after T001):
```
T002 (error.rs) | T003 (config.rs) — parallel, different files
T004 (chat_completions.rs) — parallel with T003, needs T002
T005 (config/mod.rs) — needs T003
T006 (tests) — parallel with T005
```

**Phase 3** (after Phase 2):
```
T007 → T008 → T009 — sequential (build pipeline types)
T010 (tests) — parallel with T009
T011 + T012 (GTK) — parallel, different files; parallel with T007-T010
T013 (integration) — needs T008 + T011 + T012
```

**Phase 4** (after Phase 3):
```
T014 → T015 — sequential (progress enum → pipeline extension)
T016 (GTK) — needs T015
T017 + T018 (tests) — parallel with each other; parallel with T016
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (T001)
2. Complete Phase 2: Foundational (T002–T006)
3. Complete Phase 3: User Story 1 (T007–T013)
4. **STOP and VALIDATE**: Single processor transforms text, no-processor config unchanged
5. This is a shippable increment

### Incremental Delivery

1. Setup + Foundational → Foundation ready
2. Add US1 → Single processor works → **MVP**
3. Add US2 → Multi-processor pipeline with progress → Test independently
4. Add US3 → Error transparency → Test independently
5. Polish → Documentation, proptests, final verification

---

## Notes

- All `cargo` commands must use `guix shell -m manifest.scm -- cargo ...`
- Library crate (`src/postprocess/`) MUST have zero GTK dependencies
- GTK adapter code only in `src/app/mod.rs` and `src/app/overlay.rs`
- `http_status_as_error(false)` pattern reused from transcription provider
- `Secret` type reused for API keys (supports `!FromEnv`, `!FromCommand`, `!String`)
- Commit after each task or logical group per Conventional Commits

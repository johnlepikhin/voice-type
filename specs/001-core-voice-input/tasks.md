# Tasks: Core Voice Input

**Input**: Design documents from `/specs/001-core-voice-input/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/

**Tests**: Included per Constitution Principle III (Test-Driven Quality):
unit tests in `#[cfg(test)] mod tests`, `proptest` for serialization
round-trips, integration tests in `tests/`.

**Organization**: Tasks grouped by user story. US1 is MVP.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: US1, US2, US3

---

## Phase 1: Setup

**Purpose**: Cargo project initialization, dependencies, tooling

- [x] T001 Create Cargo.toml with all dependencies (gtk4, gtk4-layer-shell, cpal, hound, ureq, hotkey-listener, serde, serde_yaml, structdoc, humantime-serde, secstr, clap, async-channel, thiserror, anyhow, proptest dev-dep) and `rust-version = "1.88"` in Cargo.toml
- [x] T002 Create src/lib.rs with `#![warn(clippy::all, clippy::pedantic)]` and module declarations for config, audio, provider, hotkey, insertion, types, error in src/lib.rs
- [x] T003 [P] Create src/main.rs skeleton with `#![warn(clippy::all, clippy::pedantic)]`, anyhow Result, clap parse, and GtkApplication builder stub in src/main.rs
- [x] T004 [P] Create src/css/style.css with base overlay styling (semi-transparent background, border-radius, font sizes for recording indicator and transcription text) in src/css/style.css

**Checkpoint**: `cargo check` passes, project structure matches plan.md layout

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Domain types, error types, configuration, provider trait — used by ALL user stories

**CRITICAL**: No user story work can begin until this phase is complete

- [x] T005 [P] Implement newtypes (RmsLevel, TranscribedText, ConfirmedText, LanguageCode, HotkeyBinding, SampleRate) with Display, From, validation per data-model.md in src/types.rs
- [x] T006 [P] Implement error types (RecordingError, TranscriptionError, TextInsertionError, ConfigError, ValidationError) with thiserror derives and `#[non_exhaustive]` per data-model.md in src/error.rs
- [x] T007 [P] Implement Secret enum (String/FromEnv/FromCommand) with SecUtf8String wrapper, unsecure() method, command caching via OnceLock, StructDoc impl per research R8 — copy pattern from summitx reference in src/config/secret.rs
- [x] T008 Implement AppConfig, ProviderConfig (flat struct with ProviderKind), AudioConfig, HotkeyConfig, UiConfig, OverlayPosition with Serialize/Deserialize/StructDoc derives, serde defaults, humantime_serde for Duration fields per data-model.md in src/config/mod.rs
- [x] T009 [P] Define TranscriptionProvider trait (transcribe method returning Result<TranscriptionResult, TranscriptionError>) with Send + Sync bounds, AudioData, TranscribeOptions, TranscriptionResult types per contracts/provider-trait.md in src/provider/mod.rs
- [x] T010 Implement top-level CLI structure with clap derive: Cli struct with --config/--verbose global options, Commands enum with Record/Daemon/Stop/Status/Config variants, ConfigCommands sub-enum (Validate/Show/Init/Docs) per contracts/cli.md in src/cli.rs
- [x] T011 [P] Write proptest round-trip tests for AppConfig serde (serialize → deserialize → assert equal), Secret enum YAML tags, LanguageCode validation, HotkeyBinding parsing in tests/config_roundtrip.rs

**Checkpoint**: `cargo test` passes, all foundational types compile and round-trip correctly

---

## Phase 3: User Story 1 — One-Shot Voice Transcription (P1) MVP

**Goal**: User launches app, records voice, sees transcription in a window. Validates the full audio capture → transcription → display pipeline.

**Independent Test**: Run `voice-type record`, click Start, speak a phrase, click Stop, verify transcription text appears in window.

### Implementation for User Story 1

- [x] T012 [P] [US1] Implement cpal-based audio capture: device enumeration, input stream setup (16kHz mono i16), sample collection into Vec<i16>, RMS level calculation, start/stop control via channel per research R6 in src/audio/capture.rs
- [x] T013 [P] [US1] Implement AudioData struct with WAV encoding via hound (in-memory Vec<u8>), silence detection (RMS threshold check), validation (non-empty, duration limits) per data-model.md in src/audio/mod.rs
- [x] T014 [US1] Implement OpenAiWhisperProvider: ureq multipart POST to /v1/audio/transcriptions with WAV file, model, language, prompt fields; parse JSON response; map HTTP errors (401→AuthenticationError, 413→EmptyAudio, 429/500→ProviderError) per contracts/provider-trait.md in src/provider/openai.rs
- [x] T015 [US1] Implement RecordingWindow GTK4 widget: Start/Stop button toggling RecordingStatus state machine, elapsed time label updated via glib::timeout_add_local, RMS level indicator, transcription display area (selectable TextView), loading spinner during transcription, error display with retry button, provider factory creating OpenAiWhisperProvider from config per spec acceptance scenarios 1-5 in src/app/recording_window.rs
- [x] T016 [US1] Wire `record` CLI command: load config from --config path (default ~/.config/voice-type.yaml), construct GtkApplication (non-service mode), create RecordingWindow on activate, spawn transcription on background thread via async-channel per research R2 in src/main.rs
- [x] T017 [US1] Write integration test with mock TranscriptionProvider: verify AudioData→TranscriptionResult pipeline, test error paths (NetworkError, AuthenticationError, EmptyAudio), test silence detection in tests/provider_mock.rs

**Checkpoint**: `voice-type record` opens window, captures audio, sends to Whisper API, displays transcription. US1 acceptance scenarios 1-5 pass.

---

## Phase 4: User Story 2 — Daemon Mode with Hotkey and Text Insertion (P2)

**Goal**: Background daemon listens for global hotkey, shows overlay, transcribes, inserts text into previously active window.

**Independent Test**: Start `voice-type daemon`, open a text editor, press Super+V, speak, press Super+V again, confirm in overlay, verify text appears in editor.

**Depends on**: US1 complete (reuses audio capture + transcription pipeline)

### Implementation for User Story 2

- [x] T018 [P] [US2] Implement global hotkey listener using hotkey-listener crate: parse HotkeyBinding config into key combo, run evdev listener on background thread, send pressed/released events to glib main loop via async-channel per research R4 in src/hotkey/mod.rs
- [x] T019 [P] [US2] Implement text insertion module: detect session type via $XDG_SESSION_TYPE, X11 path (xclip + xdotool key ctrl+v), Wayland path (wl-copy + wtype -M ctrl -k v), save/restore previous clipboard content, handle TargetWindowGone fallback per research R5 in src/insertion/mod.rs
- [x] T020 [US2] Implement GtkApplication daemon mode: IS_SERVICE flag + hold(), D-Bus single-instance enforcement (connect_activate for second instance detection), shutdown cleanup, DaemonPhase state machine (Idle→Recording→Transcribing→AwaitingConfirmation) tracking active window ID before overlay per research R1 and data-model.md in src/app/mod.rs
- [x] T021 [US2] Implement overlay window: compact layout with recording indicator / spinner / editable TextView / Confirm+Cancel buttons, Escape to cancel, CSS from src/css/style.css per spec acceptance scenarios 1-6 in src/app/overlay.rs
- [x] T022 [US2] Wire `daemon` and `stop` CLI commands: daemon starts GtkApplication with hotkey listener + overlay lifecycle, stop sends D-Bus quit action to running instance per contracts/cli.md in src/main.rs
- [x] T023 [US2] Implement DaemonPhase transitions: hotkey toggles Recording/Transcribing, transcription result shows overlay with AwaitingConfirmation, Confirm triggers text insertion + close, Cancel closes overlay + returns to Idle per spec acceptance scenarios 1-7 in src/app/mod.rs

**Checkpoint**: `voice-type daemon` runs in background, hotkey triggers recording, overlay shows transcription, confirmed text inserted into target window. US2 acceptance scenarios 1-7 pass.

---

## Phase 5: User Story 3 — Configuration and CLI Management (P3)

**Goal**: Full CLI management of config and daemon. Validate config, generate defaults, show effective config, print documentation, query daemon status.

**Independent Test**: Run `voice-type config init`, edit config, run `voice-type config validate`, start daemon, run `voice-type status`, run `voice-type stop`.

**Depends on**: US2 complete (status command queries daemon)

### Implementation for User Story 3

- [x] T024 [P] [US3] Implement `config validate` command: load YAML, run validation on all fields (api_key presence, sample_rate range 8000-48000, hotkey binding format, timeout range), collect ALL ValidationErrors in single pass (SC-008), display with field path + suggestion per contracts/cli.md in src/cli.rs and src/config/mod.rs
- [x] T025 [P] [US3] Implement `config init` command: generate default voice-type.yaml with YAML comments explaining each field, write to --config path (default ~/.config/voice-type.yaml), refuse overwrite without --force, include example Secret variants in comments per contracts/cli.md and data-model.md default YAML in src/cli.rs
- [x] T026 [P] [US3] Implement `config show` command: load and re-serialize AppConfig with defaults resolved, mask Secret values as `***` in output per contracts/cli.md in src/cli.rs
- [x] T027 [P] [US3] Implement `config docs` command: call AppConfig::document() via StructDoc trait, format and print documentation tree per contracts/cli.md in src/cli.rs
- [x] T028 [US3] Implement `status` command: query running daemon via D-Bus (GtkApplication activation), display PID/uptime/provider/hotkey/state in text or --json format per contracts/cli.md in src/cli.rs
- [x] T029 [US3] Write CLI integration tests: test config init creates file, config validate catches errors, config show masks secrets, config docs outputs documentation, help text includes all commands per contracts/cli.md in tests/cli_integration.rs

**Checkpoint**: All CLI commands work per contracts/cli.md. US3 acceptance scenarios 1-6 pass.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Edge cases, error handling, quality gates

- [x] T030 Implement edge case handling: microphone disconnect during recording (detect cpal stream error, offer partial transcription), silence detection warning before sending to API, network timeout with retry option, transcription cancel without losing audio per spec Edge Cases section in src/audio/capture.rs, src/app/recording_window.rs, src/app/overlay.rs
- [x] T031 [P] Review all user-facing error messages: ensure no internal details leak (FR-013), verify invalid API key shows config file guidance, no microphone shows device help, network errors show retry per spec FR-013 in src/error.rs and src/app/
- [x] T032 [P] Run quality gates: `cargo fmt -- --check`, `cargo clippy -- -D warnings`, `cargo test`, `cargo doc --no-deps` per constitution Development Workflow section
- [x] T033 Validate quickstart.md end-to-end: follow all steps from prerequisites through first run, verify all commands and troubleshooting steps are accurate per specs/001-core-voice-input/quickstart.md

---

## Dependencies & Execution Order

### Phase Dependencies

```
Phase 1 (Setup)
  └──→ Phase 2 (Foundational) ──BLOCKS──→ all user stories
         ├──→ Phase 3 (US1: One-Shot) ──MVP checkpoint
         │      └──→ Phase 4 (US2: Daemon) ──depends on US1 pipeline
         │             └──→ Phase 5 (US3: CLI) ──status needs daemon
         └──→ Phase 5 (US3: config commands T024-T027) ──can parallel with US2
              └──→ Phase 6 (Polish) ──after all stories
```

### User Story Dependencies

- **US1 (P1)**: Depends only on Foundational. No cross-story deps. **MVP scope.**
- **US2 (P2)**: Depends on US1 (reuses audio capture + transcription pipeline)
- **US3 (P3)**: `config` subcommands (T024-T027) can run parallel with US2; `status` (T028) depends on US2 daemon

### Within Each User Story

- Types/models before services
- Services before UI components
- Core implementation before CLI wiring
- Unit tests alongside implementation (in-module `#[cfg(test)]`)
- Integration tests after implementation

### Parallel Opportunities

**Phase 2**: T005, T006, T007, T009, T011 all touch different files → parallel
**US1**: T012, T013 parallel (audio/capture.rs vs audio/mod.rs)
**US2**: T018, T019 parallel (hotkey vs insertion — different modules)
**US3**: T024, T025, T026, T027 all parallel (independent CLI subcommands)

---

## Parallel Examples

### Phase 2 (Foundational)
```
Parallel batch 1: T005 (types.rs) + T006 (error.rs) + T007 (secret.rs) + T009 (provider/mod.rs) + T011 (tests/config_roundtrip.rs)
Sequential after: T008 (config/mod.rs — uses types, secret, provider) → T010 (cli.rs — uses config)
```

### User Story 1 (P1)
```
Parallel batch 1: T012 (audio/capture.rs) + T013 (audio/mod.rs)
Sequential: T014 (provider/openai.rs) → T015 (recording_window.rs) → T016 (main.rs wiring)
Parallel: T017 (tests/provider_mock.rs — after T014)
```

### User Story 2 (P2)
```
Parallel batch 1: T018 (hotkey/mod.rs) + T019 (insertion/mod.rs)
Sequential: T020 (app/mod.rs) → T021 (overlay.rs) → T022 (main.rs wiring) → T023 (state transitions)
```

### User Story 3 (P3)
```
Parallel batch 1: T024 (validate) + T025 (init) + T026 (show) + T027 (docs)
Sequential after: T028 (status — needs daemon from US2) → T029 (CLI tests)
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (T001-T004)
2. Complete Phase 2: Foundational (T005-T011)
3. Complete Phase 3: US1 One-Shot Transcription (T012-T017)
4. **STOP AND VALIDATE**: `voice-type record` works end-to-end
5. This is a deployable MVP — user can record and transcribe voice

### Incremental Delivery

1. Setup + Foundational → `cargo check` passes
2. US1 → `voice-type record` works → **MVP**
3. US2 → `voice-type daemon` + hotkey + overlay + text insertion → **Primary workflow**
4. US3 → Full CLI management (config init/validate/show/docs, status) → **Production-ready**
5. Polish → Edge cases, error messages, quality gates → **Release candidate**

---

## Notes

- Constitution requires `#[cfg(test)] mod tests` in each module — write unit tests inline as you implement each task, not as separate tasks
- T011 and T017 are dedicated test files for cross-module concerns (proptest, mock provider)
- Secret enum implementation (T007): copy pattern from `/home/evgenii/mountain-project/repos/summitx/webapp_yaml_config/src/secret.rs`
- All public enums must have `#[non_exhaustive]`
- All public items must have `///` doc comments
- Commit after each task or logical group following Conventional Commits

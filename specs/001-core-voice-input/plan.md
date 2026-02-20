# Implementation Plan: Core Voice Input

**Branch**: `001-core-voice-input` | **Date**: 2026-02-20 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/001-core-voice-input/spec.md`

## Summary

GTK4 desktop application for Linux that captures voice input, sends it
to OpenAI Whisper API for transcription, and inserts the confirmed text
into the previously active window. Operates in two modes: one-shot
(interactive window) and daemon (background service with global hotkey).

**Technical approach**: GTK4 via `gtk4-rs` with `GtkApplication` daemon
mode (D-Bus single-instance). Audio capture via `cpal` (ALSA backend).
HTTP to Whisper API via `ureq` on background threads. Global hotkeys
via `hotkey-listener` (evdev). Text insertion via clipboard + paste
simulation (`xclip`/`xdotool` on X11, `wl-copy`/`wtype` on Wayland).
Configuration via `serde_yaml` + `structdoc` + `humantime_serde`.
Trait-based `TranscriptionProvider` for provider extensibility.

## Technical Context

**Language/Version**: Rust 1.88+ (stable, edition 2021)
**Primary Dependencies**:
- `gtk4` (gtk4-rs) — UI toolkit
- `gtk4-layer-shell` — Wayland overlay windows
- `cpal` — audio capture (ALSA)
- `ureq` — blocking HTTP client
- `hotkey-listener` — evdev-based global hotkeys
- `serde`, `serde_yaml` — config serialization
- `structdoc` — config documentation generation
- `humantime_serde` — human-readable durations
- `secstr` — secure string storage
- `clap` (derive) — CLI parsing
- `async-channel` — glib ↔ thread communication
- `thiserror` — library error types
- `anyhow` — binary error handling

**Storage**: YAML config file (`~/.config/voice-type.yaml`), no database
**Testing**: `cargo test`, `proptest` for serialization round-trips
**Target Platform**: Linux (X11 + Wayland)
**Project Type**: Single Cargo project (library + binary)
**Performance Goals**: Overlay < 500ms, transcription cycle < 10s, daemon idle < 50MB
**Constraints**: No tokio runtime, single glib main loop, evdev requires `input` group
**Scale/Scope**: Single-user desktop app, ~3K LOC estimated

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### I. Algebraic Type Design — PASS

- `RecordingStatus`, `TranscriptionStatus`, `DaemonPhase`: enum state machines
  with exhaustive variants (see data-model.md)
- Newtypes: `RmsLevel`, `TranscribedText`, `ConfirmedText`, `LanguageCode`,
  `HotkeyBinding`, `SampleRate` — no raw primitives in public API
- `#[non_exhaustive]` on all public enums
- `Secret` enum for credential variants (String/FromEnv/FromCommand)
- `ProviderConfig` as tagged enum for provider extensibility

### II. Extensibility Through Composition — PASS

- `TranscriptionProvider` trait at module boundary (see contracts/provider-trait.md)
- `ProviderConfig` tagged enum + factory pattern for provider registration
- Audio capture behind trait boundary for testability
- Text insertion strategy abstracted by session type detection

### III. Test-Driven Quality — PASS

- Unit tests in `#[cfg(test)] mod tests` per module
- Integration tests in `tests/` for CLI commands and config round-trips
- `proptest` for config serialization round-trips and audio data invariants
- GTK widget testing via programmatic signal emission (recording window states)
- Provider trait enables mock implementations for testing

### IV. GTK Architecture Discipline — PASS

- Library crate (`src/lib.rs` subtree) with zero GTK dependencies:
  domain types, config, provider trait, audio capture logic
- Binary crate (`src/main.rs`) as thin GTK adapter
- Async via `glib::MainContext` futures + `async-channel` from background threads
- No blocking on main loop — HTTP and audio on dedicated threads
- CSS from external files for theming

### V. Code Quality & Safety — PASS

- `#![warn(clippy::all, clippy::pedantic)]` in lib.rs and main.rs
- `thiserror` for all library errors, `anyhow` only in main.rs
- No `unsafe` needed (gtk4-rs handles FFI internally)
- `///` doc comments on all public items
- `cargo audit` + `cargo deny` in quality gates

### Post-Phase 1 Re-check — PASS

All design artifacts (data-model.md, contracts/) align with
constitution principles. No violations found.

## Project Structure

### Documentation (this feature)

```text
specs/001-core-voice-input/
├── plan.md              # This file
├── research.md          # Phase 0: technology research
├── data-model.md        # Phase 1: domain types and state machines
├── quickstart.md        # Phase 1: setup and usage guide
├── contracts/
│   ├── cli.md           # Phase 1: CLI interface contract
│   └── provider-trait.md # Phase 1: provider abstraction contract
└── tasks.md             # Phase 2: implementation tasks (via /speckit.tasks)
```

### Source Code (repository root)

```text
Cargo.toml
src/
├── lib.rs               # Library crate root (no GTK dependency)
├── config/
│   ├── mod.rs           # AppConfig, ProviderConfig, AudioConfig, etc.
│   └── secret.rs        # Secret enum (from summitx reference)
├── audio/
│   ├── mod.rs           # AudioData, recording logic
│   └── capture.rs       # cpal-based audio capture
├── provider/
│   ├── mod.rs           # TranscriptionProvider trait
│   └── openai.rs        # OpenAI Whisper implementation
├── hotkey/
│   └── mod.rs           # Global hotkey listener (evdev)
├── insertion/
│   └── mod.rs           # Text insertion (clipboard + paste)
├── types.rs             # Newtypes (RmsLevel, TranscribedText, etc.)
├── error.rs             # Error types (thiserror)
├── main.rs              # Binary entry point, GTK application
├── app/
│   ├── mod.rs           # GtkApplication setup, daemon mode
│   ├── recording_window.rs  # One-shot recording window
│   └── overlay.rs       # Daemon overlay window
├── cli.rs               # clap command definitions
└── css/
    └── style.css        # GTK CSS theme

tests/
├── config_roundtrip.rs  # proptest: config serde round-trips
├── cli_integration.rs   # CLI command integration tests
└── provider_mock.rs     # Mock provider integration tests
```

**Structure Decision**: Single Cargo project with library + binary split.
The `src/lib.rs` subtree contains all domain logic (config, audio,
provider, types, errors) with zero GTK dependencies. The `src/main.rs`
and `src/app/` contain GTK-specific code. This follows Constitution
Principle IV (GTK Architecture Discipline).

## Complexity Tracking

> No violations. All design choices align with constitution principles.

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| *(none)*  | —          | —                                   |

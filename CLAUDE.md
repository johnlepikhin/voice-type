# voice-type Development Guidelines

Auto-generated from all feature plans. Last updated: 2026-02-20

## Active Technologies
- N/A (documentation only — no code changes) (004-user-documentation)

- Rust 1.88+ (stable, edition 2021) (001-core-voice-input)
- GTK4 via gtk4-rs bindings
- YAML config via serde_yaml + structdoc + humantime_serde

## Project Structure

```text
src/
├── lib.rs               # Library crate (no GTK dependency)
├── config/              # AppConfig, Secret, serde
├── audio/               # cpal-based audio capture
├── provider/            # TranscriptionProvider trait + OpenAI impl
├── postprocess/         # Post-processing pipeline (LLM chat completions)
├── http.rs              # Shared OpenAI-compatible HTTP helpers
├── types.rs             # Newtypes (RmsLevel, TranscribedText, etc.)
├── error.rs             # thiserror error types
├── main.rs              # GTK application entry point
├── app/                 # GTK widgets (overlay)
├── cli.rs               # clap CLI definitions
└── css/                 # External CSS theme files
tests/                   # Integration and property-based tests
specs/                   # Feature specifications (speckit)
```

## Commands

Build environment requires Guix shell:
```bash
guix shell -m manifest.scm -- cargo fmt -- --check     # Zero diff
guix shell -m manifest.scm -- cargo clippy --all-targets -- -D warnings  # Zero warnings
guix shell -m manifest.scm -- cargo test               # All tests green
guix shell -m manifest.scm -- cargo doc --no-deps      # Docs build clean
```

## Code Style

- `#![warn(clippy::all, clippy::pedantic)]`
- `thiserror` for library errors, `anyhow` only in main.rs
- No `unsafe` outside FFI boundaries
- `///` doc comments on all public items
- Conventional Commits format

## Architecture Rules

- Library crate (`src/lib.rs`) has ZERO GTK dependencies
- GTK code is a thin adapter in `src/main.rs` + `src/app/`
- Async via `glib::MainContext` — no tokio
- HTTP on background `std::thread` with `ureq` + `mpsc` channel
- All domain enums use `#[non_exhaustive]`
- Newtypes for domain primitives (no raw String/i32 in public API)
- `ProviderConfig` is an externally tagged enum with custom serde (map-based, not YAML tags)
- `ProviderConfig::build_provider()` factory creates provider + options (no duplication)

## Recent Changes
- 004-user-documentation: Added N/A (documentation only — no code changes)
- 003-memory-optimization: Daemon memory optimization (buffer lifecycle, shared HTTP agent, lazy pipeline)
- 002-text-post-processing: Text post-processing pipeline with configurable LLM processors

<!-- MANUAL ADDITIONS START -->
<!-- MANUAL ADDITIONS END -->

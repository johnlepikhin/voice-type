# Implementation Plan: Daemon Memory Optimization

**Branch**: `003-memory-optimization` | **Date**: 2026-02-20 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/003-memory-optimization/spec.md`

## Summary

Reduce daemon idle memory from 100MB+ to under 60MB (VmRSS) and ensure memory returns to baseline after each recording cycle. Primary approach: explicit buffer lifecycle management, shared HTTP agent across post-processors, lazy pipeline initialization, and memory diagnostics logging.

## Technical Context

**Language/Version**: Rust 1.88+ (stable, edition 2021)
**Primary Dependencies**: gtk4-rs 0.9, ureq 3, cpal 0.15, evdev 0.12, serde_yaml 0.9
**Storage**: N/A (no persistent storage changes)
**Testing**: `cargo test`, property-based tests via `proptest`
**Target Platform**: Linux (Guix System, Wayland/X11)
**Project Type**: Single Rust crate (library + binary)
**Performance Goals**: VmRSS < 60MB idle, memory return within 5MB after recording cycle
**Constraints**: No new dependencies; no functionality regression; startup time regression < 10%
**Scale/Scope**: Single-user desktop daemon, 0-3 post-processors typical

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Algebraic Type Design | PASS | No new public types needed; existing newtypes preserved |
| II. Extensibility Through Composition | PASS | Shared agent injected via parameter, not global state |
| III. Test-Driven Quality | PASS | Memory diagnostics testable; buffer lifecycle verifiable in unit tests |
| IV. GTK Architecture Discipline | PASS | All changes in library crate; no GTK code modified |
| V. Code Quality & Safety | PASS | No unsafe; clippy pedantic; thiserror for errors |

**Quality Gates**: `cargo fmt`, `cargo clippy -D warnings`, `cargo test`, `cargo doc --no-deps` — all must pass.

## Project Structure

### Documentation (this feature)

```text
specs/003-memory-optimization/
├── plan.md              # This file
├── research.md          # Phase 0: decisions and profiling baseline
├── data-model.md        # Phase 1: buffer lifecycle states and ownership map
├── quickstart.md        # Phase 1: verification guide
└── tasks.md             # Phase 2 output (/speckit.tasks)
```

### Source Code (files modified)

```text
src/
├── lib.rs                          # Add memory diagnostics utility
├── audio/
│   ├── mod.rs                      # Explicit buffer drops in AudioData conversion
│   └── capture.rs                  # (no changes — buffer ownership moves correctly)
├── provider/
│   └── openai.rs                   # Explicit body drop after HTTP response; shared agent param
├── postprocess/
│   ├── mod.rs                      # Lazy pipeline init; accept shared agent
│   └── chat_completions.rs         # Accept agent by reference instead of creating own
├── app/
│   └── mod.rs                      # Explicit buffer drops in daemon flow; memory logging
└── error.rs                        # (no changes expected)
tests/
└── (existing tests — verify no regression)
```

**Structure Decision**: Existing single-crate layout preserved. Changes are internal refactors within existing modules — no new files or modules needed.

## Design Decisions

### 1. Explicit buffer drops in transcription flow

**Current**: Audio samples → WAV bytes → multipart body. All three buffers coexist in memory during transcription (~40MB peak for 5-minute recording).

**Target**: Drop each buffer immediately after its data is consumed by the next stage. Peak drops from 3× to ~1.3× audio size.

**Files**: `src/app/mod.rs` (daemon recording flow), `src/provider/openai.rs` (transcribe method)

### 2. Shared HTTP agent for post-processors

**Current**: Each `ChatCompletionsClient` creates its own `ureq::Agent` with connection pool.

**Target**: Single `Agent` created in daemon setup, passed to `ProcessingPipeline` and shared across all `ChatCompletionsClient` instances.

**Files**: `src/postprocess/chat_completions.rs`, `src/postprocess/mod.rs`, `src/app/mod.rs`

### 3. Lazy pipeline initialization

**Current**: `ProcessingPipeline::from_configs()` eagerly creates all processors + HTTP agents at daemon start.

**Target**: Pipeline stores configs; processors created on first `run()` call using `OnceLock`. Idle daemon holds only config structs (~bytes), not agents (~MB).

**Files**: `src/postprocess/mod.rs`

### 4. Memory diagnostics logging

**Current**: No memory visibility.

**Target**: `log_memory_usage(label)` function that reads `/proc/self/status` VmRSS and logs via `tracing::debug!`. Called at: daemon start, recording start, transcription complete, pipeline complete.

**Files**: `src/lib.rs` (utility function), `src/app/mod.rs` (call sites)

## Complexity Tracking

No constitution violations. No complexity justification needed.

# Research: Daemon Memory Optimization

**Date**: 2026-02-20
**Branch**: `003-memory-optimization`

## Decision 1: Buffer lifecycle strategy

**Decision**: Explicit drop with `Vec::clear()` + `Vec::shrink_to_fit()` at lifecycle boundaries.

**Rationale**: The current code creates temporary buffers (audio samples, WAV encoding, multipart body) but relies on implicit drop via scope exit. During transcription, three copies of audio data coexist simultaneously (~40MB peak for a 5-minute recording). By explicitly dropping buffers as soon as their data is consumed:

- Audio `Vec<i16>` dropped after `encode_wav()` consumes it
- WAV `Vec<u8>` dropped after `build_multipart_body()` consumes it
- Multipart body dropped after HTTP response received

**Alternatives considered**:
- Streaming upload (avoids in-memory body entirely) — rejected because ureq v3 multipart requires pre-built body, and OpenAI Whisper API needs Content-Length
- Memory-mapped file as intermediate — rejected as over-engineering for 5-minute recordings (~20MB)

## Decision 2: HTTP agent sharing

**Decision**: Share a single `ureq::Agent` across all `ChatCompletionsClient` instances via parameter injection.

**Rationale**: Currently each `ChatCompletionsClient::new()` creates its own `Agent` with connection pool. With 3 post-processors, that's 3 separate pools. Since all processors talk to the same API endpoint (or similar endpoints), sharing one agent reduces idle connection overhead from ~3-6MB to ~1-2MB and improves connection reuse.

**Alternatives considered**:
- Global `LazyLock<Agent>` — rejected because it prevents different timeout configurations per processor
- Agent per unique base URL — unnecessary complexity for 0-3 processors

## Decision 3: Lazy pipeline initialization

**Decision**: Defer `ChatCompletionsClient` creation until first pipeline execution using `OnceLock` or equivalent.

**Rationale**: `ProcessingPipeline::from_configs()` eagerly creates all processors and their HTTP agents at startup. If the user never triggers a recording, this memory is wasted. Lazy init defers ~1-2MB per processor until actually needed.

**Alternatives considered**:
- Fully lazy (recreate per request) — rejected due to connection pool warm-up cost on every recording
- Config-only init with factory pattern — functionally identical to OnceLock, more code

## Decision 4: Memory diagnostics approach

**Decision**: Log VmRSS at key lifecycle points via `tracing::debug!` reading `/proc/self/status`.

**Rationale**: Simple, zero-dependency, Linux-native. Logs at: daemon start, recording start, transcription complete, post-processing complete. Activated by standard `RUST_LOG=voice_type=debug` — no new CLI flags needed.

**Alternatives considered**:
- CLI subcommand `voice-type memory` — over-engineering for diagnostics
- External profiling only (heaptrack, valgrind) — still useful but not integrated

## Decision 5: Allocator choice

**Decision**: Stay with system allocator (glibc malloc). Revisit only if profiling shows fragmentation.

**Rationale**: jemalloc/mimalloc can improve allocation throughput and reduce fragmentation, but add ~2MB binary overhead and a new dependency. The primary memory issues are architectural (buffer copies, eager init), not allocator-related. Once architectural fixes land, profile again to decide.

**Alternatives considered**:
- jemalloc via `tikv-jemallocator` — deferred, adds binary size and build complexity
- mimalloc via `mimalloc` crate — deferred, same reasoning

## Profiling baseline

- **Binary size (release, LTO, stripped)**: 4.2MB
- **Idle measurement needed**: Start daemon, wait 30s, check `/proc/PID/status` VmRSS
- **Peak during recording**: ~40MB above idle baseline (audio + WAV + multipart body simultaneously)
- **Known memory holders at idle**: GTK4 widgets, hotkey listener thread, HTTP agents (if post-processors configured), COMMAND_CACHE

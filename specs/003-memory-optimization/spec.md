# Feature Specification: Daemon Memory Optimization

**Feature Branch**: `003-memory-optimization`
**Created**: 2026-02-20
**Status**: Draft
**Input**: User description: "Необходимо оптимизировать приложение по потребляемой памяти. В режиме демона релизная сборка потребляет сейчас более 100MB."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Low idle memory footprint (Priority: P1)

As a user running voice-type as a background daemon, I want the application to consume minimal memory while idle (waiting for hotkey), so that it doesn't compete with other applications for system resources.

**Why this priority**: The daemon spends the vast majority of its time in idle state. Reducing idle memory has the highest cumulative impact.

**Independent Test**: Start the daemon, wait for initialization to complete, measure RSS memory via `/proc/PID/status` (VmRSS). Compare against the target threshold.

**Acceptance Scenarios**:

1. **Given** the daemon is started with default configuration and one post-processor, **When** 30 seconds pass without any recording, **Then** resident memory (VmRSS) is below the target idle threshold.
2. **Given** the daemon has been idle for several minutes, **When** memory is measured, **Then** it has not grown compared to the initial idle measurement (no idle memory leak).

---

### User Story 2 - Memory released after recording cycle (Priority: P2)

As a user who records and transcribes frequently, I want the application to release temporary buffers (audio samples, WAV encoding, HTTP request body) after each recording cycle, so that memory doesn't accumulate across sessions.

**Why this priority**: Without cleanup, each recording cycle can add 5-15MB of retained buffers. Over a workday of frequent use, this would cause visible memory growth.

**Independent Test**: Perform 5 consecutive recording-transcription cycles, measure RSS after each cycle returns to idle. Memory should return close to the idle baseline each time.

**Acceptance Scenarios**:

1. **Given** the daemon is idle, **When** the user records for 30 seconds, transcription completes, and the daemon returns to idle, **Then** resident memory returns to within 5MB of the pre-recording baseline.
2. **Given** 10 consecutive recording-transcription cycles have been performed, **When** the daemon is idle after the last cycle, **Then** memory is within 10MB of the initial idle measurement.

---

### User Story 3 - Lazy resource initialization (Priority: P3)

As a user who configures multiple post-processors, I want HTTP connections and agents to be created only when first needed, so that the idle daemon doesn't pay the cost of resources it may never use.

**Why this priority**: Each HTTP agent with its connection pool adds overhead. Users with multiple post-processors or those who rarely use certain features shouldn't pay upfront memory costs.

**Independent Test**: Start the daemon with 3 post-processors configured but don't trigger any recording. Verify that HTTP agent/connection pool memory is not allocated until the first recording triggers post-processing.

**Acceptance Scenarios**:

1. **Given** the daemon starts with multiple post-processors configured, **When** no recording has been triggered, **Then** HTTP connection pools for post-processing are not yet allocated.
2. **Given** the daemon starts and the user triggers one recording, **When** transcription and post-processing complete, **Then** only the required agents are initialized.

---

### Edge Cases

- What happens when the user rapidly triggers many recordings in succession? Memory should not grow unboundedly.
- How does the system behave when a recording is cancelled mid-way? Partial audio buffers should be released.
- What happens when the network is unavailable and HTTP requests fail? Retry buffers should not accumulate.
- What if the daemon runs for 24+ hours without restart? Memory should remain stable.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST release audio sample buffers after each recording cycle completes (success or error).
- **FR-002**: System MUST release WAV encoding and HTTP request body buffers after transcription response is received or request fails.
- **FR-003**: System MUST NOT pre-allocate HTTP agents or connection pools for post-processors until the first recording triggers post-processing.
- **FR-004**: System MUST provide a way to measure current memory usage for diagnostic purposes (e.g., logging RSS at key lifecycle points).
- **FR-005**: System MUST maintain stable memory over extended operation (no monotonic growth across recording cycles).
- **FR-006**: System MUST NOT regress in functionality — all existing features (recording, transcription, post-processing, hotkey handling) continue to work identically.

### Key Entities

- **Audio buffer**: Temporary `Vec<i16>` holding recorded samples during a recording session. Exists only while recording is active.
- **WAV payload**: Encoded WAV bytes used for the transcription API request. Exists only during the transcription HTTP call.
- **HTTP agent**: Connection pool manager for outbound API calls. One for transcription provider, one per post-processor.
- **Processing pipeline**: Collection of post-processors that transform transcribed text. Holds references to HTTP agents.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Idle daemon memory (VmRSS) is below 60MB with default configuration and one post-processor.
- **SC-002**: After a recording-transcription cycle, daemon memory returns to within 5MB of the idle baseline within 10 seconds.
- **SC-003**: After 20 consecutive recording cycles, memory remains within 10MB of the initial idle baseline (no cumulative leak).
- **SC-004**: Startup time does not increase by more than 10% compared to the current version (lazy initialization must not shift cost to a visible delay during first recording).

## Assumptions

- GTK4 runtime itself contributes 30-50MB of baseline memory that cannot be reduced without removing the UI layer entirely. Optimization targets memory above this baseline.
- The application runs on a Linux desktop with at least 512MB of available RAM.
- "Memory" refers to VmRSS (Resident Set Size) as reported by the kernel, not virtual memory.
- The system allocator (glibc malloc) is used. Switching allocators (jemalloc, mimalloc) is in scope if it demonstrably helps.
- Post-processing pipeline typically has 0-3 configured processors.

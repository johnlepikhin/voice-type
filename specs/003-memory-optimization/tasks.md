# Tasks: Daemon Memory Optimization

**Input**: Design documents from `/specs/003-memory-optimization/`
**Prerequisites**: plan.md (required), spec.md (required), research.md, data-model.md

**Tests**: Included only where critical for verification (US3 lazy init).

**Organization**: Tasks grouped by user story. US1 is MVP.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

---

## Phase 1: Setup

**Purpose**: Establish baseline and diagnostic infrastructure

- [ ] T001 Measure baseline idle VmRSS of release build daemon (build with `cargo build --release`, start daemon, wait 30s, record `/proc/PID/status` VmRSS)
- [X] T002 Add `log_memory_usage(label: &str)` utility function in src/lib.rs that reads VmRSS from `/proc/self/status` and logs via `tracing::debug!`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Refactor `ChatCompletionsClient` to accept a shared `Agent` — this changes the internal API and MUST complete before user stories

**CRITICAL**: No user story work can begin until this phase is complete

- [X] T003 Refactor `ChatCompletionsClient::new()` to accept `Agent` parameter instead of creating its own in src/postprocess/chat_completions.rs — remove internal `Agent::new_with_config()`, store passed agent as field
- [X] T004 Update `PostProcessor::new()` and `ProcessingPipeline::from_configs()` to create one shared `Agent` and pass it to all `ChatCompletionsClient` instances in src/postprocess/mod.rs
- [X] T005 Update existing tests to pass `Agent` where needed — fix compilation in tests/postprocess_pipeline.rs and src/postprocess/ test modules

**Checkpoint**: `cargo test` passes, shared agent plumbing complete

---

## Phase 3: User Story 1 — Low idle memory footprint (Priority: P1) MVP

**Goal**: Idle daemon VmRSS < 60MB with default config and one post-processor

**Independent Test**: Start daemon, wait 30s, check VmRSS < 60MB via `/proc/PID/status`

### Implementation for User Story 1

- [X] T006 [US1] Implement lazy processor initialization in `ProcessingPipeline` using `OnceLock<Vec<PostProcessor>>` in src/postprocess/mod.rs — store configs at construction, create processors + shared agent on first `run()` call
- [X] T007 [US1] Add `log_memory_usage` calls at daemon start and after initialization in src/app/mod.rs
- [ ] T008 [US1] Build release binary and measure idle VmRSS — verify < 60MB target (SC-001)

**Checkpoint**: Idle memory meets target. Daemon functionality unchanged.

---

## Phase 4: User Story 2 — Memory released after recording cycle (Priority: P2)

**Goal**: After recording-transcription-postprocessing cycle, VmRSS returns within 5MB of idle baseline

**Independent Test**: Record 30s, wait for completion, measure VmRSS delta from baseline

### Implementation for User Story 2

- [X] T009 [P] [US2] Add explicit `drop(body)` after HTTP response in `transcribe()` method in src/provider/openai.rs — ensures multipart body (~20MB) is freed before parsing response
- [X] T010 [US2] Restructure daemon recording flow in src/app/mod.rs to drop `AudioData` immediately after `transcribe()` returns — use inner scope or explicit `drop()` so audio buffer + WAV bytes don't persist during post-processing
- [X] T011 [US2] Add `log_memory_usage` calls at recording start, after transcription, after post-processing, and at cycle end in src/app/mod.rs
- [ ] T012 [US2] Build release binary and verify memory returns to within 5MB of baseline after a recording cycle (SC-002)

**Checkpoint**: Memory returns to near-baseline after each cycle. All features still work.

---

## Phase 5: User Story 3 — Lazy resource initialization (Priority: P3)

**Goal**: HTTP agents and connection pools not allocated until first recording triggers processing

**Independent Test**: Start daemon with 3 post-processors, don't record, verify lower VmRSS than pre-optimization baseline

### Implementation for User Story 3

- [X] T013 [US3] Add unit test in src/postprocess/mod.rs verifying that `ProcessingPipeline::from_configs()` does NOT create `Agent` or `PostProcessor` instances — assert internal `OnceLock` is uninitialized after construction
- [ ] T014 [US3] Verify with release build: start daemon with 3 post-processors configured, measure VmRSS vs single-processor config — delta should be minimal (< 1MB) since agents not yet created

**Checkpoint**: Lazy init verified. All three user stories independently pass.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Quality gates and final verification

- [X] T015 Run `cargo fmt -- --check` — zero diff
- [X] T016 Run `cargo clippy --all-targets -- -D warnings` — zero warnings
- [X] T017 Run `cargo test` — all tests green, no regressions (FR-006)
- [X] T018 Run `cargo doc --no-deps` — docs build clean
- [ ] T019 Perform 20-cycle stability test: record 20 times consecutively, verify VmRSS within 10MB of baseline (SC-003)
- [ ] T020 Verify startup time regression < 10% vs pre-optimization build (SC-004)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — start immediately
- **Foundational (Phase 2)**: Depends on T002 from Setup — BLOCKS all user stories
- **US1 (Phase 3)**: Depends on Phase 2 completion
- **US2 (Phase 4)**: Depends on Phase 2 completion — can run in PARALLEL with US1
- **US3 (Phase 5)**: Depends on US1 (T006 lazy init must exist before T013 can test it)
- **Polish (Phase 6)**: Depends on all user stories complete

### User Story Dependencies

- **US1 (P1)**: Depends on Phase 2 (shared agent). No dependency on other stories.
- **US2 (P2)**: Depends on Phase 2 (shared agent). Independent of US1/US3.
- **US3 (P3)**: Depends on US1 T006 (lazy init implementation). Verification-only phase.

### Within Each User Story

- T009 (openai.rs buffer drops) can run in parallel with T010 (app/mod.rs flow changes)
- T006 (lazy init) must complete before T007 (memory logging) for meaningful measurements

### Parallel Opportunities

```text
After Phase 2 completes:
  ┌─── US1: T006 → T007 → T008
  │
  └─── US2: T009 ─┐
                   ├─→ T011 → T012
              T010 ┘

After US1 T006 completes:
  └─── US3: T013 → T014
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (baseline measurement + diagnostics utility)
2. Complete Phase 2: Foundational (shared agent refactor)
3. Complete Phase 3: US1 (lazy pipeline init + verification)
4. **STOP and VALIDATE**: Measure idle VmRSS < 60MB
5. This alone delivers the highest-impact optimization

### Incremental Delivery

1. Setup + Foundational → shared agent plumbing ready
2. Add US1 (lazy init) → Test idle memory → Highest impact delivered
3. Add US2 (buffer drops) → Test cycle memory → Peak memory reduced
4. Add US3 (verification) → Confirm lazy init behavior → Full confidence
5. Polish → All quality gates pass

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- US1 and US2 can be developed in parallel after Phase 2
- US3 is verification-only — depends on US1's lazy init
- Commit after each task or logical group
- All `guix shell -m manifest.scm --` prefix required for cargo commands

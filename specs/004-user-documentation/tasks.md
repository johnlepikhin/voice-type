# Tasks: User Documentation (README)

**Input**: Design documents from `/specs/004-user-documentation/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, quickstart.md

**Tests**: Not applicable — this feature produces documentation, not code.

**Organization**: Tasks are grouped by user story to enable incremental writing and review.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (independent sections, no cross-references needed yet)
- **[Story]**: Which user story this task belongs to (US1, US2, US3, US4)
- All paths are relative to repository root

## Phase 1: Setup

**Purpose**: Create file skeleton and missing project assets

- [x] T001 Create MIT LICENSE file at LICENSE (text from opensource.org, year 2026, copyright holder from Cargo.toml or git config)
- [x] T002 Create README.md at repository root with section heading skeleton matching data-model.md structure (all headings, no content yet)

**Checkpoint**: README.md exists with all section headings; LICENSE file exists

---

## Phase 2: User Story 1 - Discover project purpose (Priority: P1) 🎯 MVP

**Goal**: A visitor understands what voice-type is within 30 seconds of reading

**Independent Test**: Show header + description to someone unfamiliar; they can explain what voice-type does

**Covers**: FR-001, FR-008, FR-009, SC-001

### Implementation

- [x] T003 [US1] Write project title and one-line summary in README.md — header section. Include: project name, one-sentence description ("Voice input for Linux — speak and get text via OpenAI Whisper")
- [x] T004 [US1] Write Description section in README.md — 2-3 sentences explaining what voice-type does, who it's for (Linux developers/power users), and why (hands-free text input). Reference key differentiators: GTK4 overlay, provider-agnostic, post-processing pipeline
- [x] T005 [US1] Write Features bullet list in README.md — concise list of capabilities: speech-to-text via Whisper, GTK4 recording overlay, YAML configuration, CLI tool, configurable post-processing, secret management (!FromEnv, !FromCommand), silence detection
- [x] T006 [US1] Write License section in README.md — reference MIT license, link to LICENSE file

**Checkpoint**: README.md header through Features and License sections are complete. SC-001 verifiable.

---

## Phase 3: User Story 2 - Install and run first session (Priority: P1)

**Goal**: A new user goes from zero to first transcription using only the README

**Independent Test**: Follow instructions on a fresh Linux system; tool builds, config is created, speech is transcribed

**Covers**: FR-002, FR-003, FR-004, FR-005, SC-002, SC-003

### Implementation

- [x] T007 [P] [US2] Write Prerequisites section in README.md — list: Linux OS, Rust 1.88+, system libraries (GTK4, gtk-layer-shell, ALSA lib, pkg-config, GCC toolchain), OpenAI API key. Source: manifest.scm, Cargo.toml rust-version field
- [x] T008 [P] [US2] Write Installation section in README.md — two paths: (1) Guix: `guix shell -m manifest.scm -- cargo build --release`, (2) Manual: install system deps (apt/dnf/pacman examples for GTK4, ALSA, pkg-config), then `cargo build --release`. Include note about adding to PATH
- [x] T009 [US2] Write Quick Start section in README.md — step-by-step flow from quickstart.md: (1) `voice-type config init`, (2) set API key via `!FromEnv` with `export OPENAI_API_KEY=...`, (3) `voice-type config validate`, (4) `voice-type record`, (5) explain overlay interaction (Enter to confirm, Escape to cancel, auto-stop at max_duration). Include example default config YAML from `AppConfig::default_yaml()`
- [x] T010 [US2] Write CLI Reference section in README.md — document from src/cli.rs: global options (-c/--config, -v/--verbose), `record` command with flags (-d/--device, -l/--language, -p/--prompt), `config` subcommands (validate, show, init --force, docs). Use code blocks for command examples. Source: R-003 from research.md

**Checkpoint**: README.md covers full install-to-first-use flow. SC-002 and SC-003 verifiable.

---

## Phase 4: User Story 3 - Configure advanced features (Priority: P2)

**Goal**: An existing user can customize any setting using the README as reference

**Independent Test**: Change a config option per README instructions; verify described behavior change takes effect

**Covers**: FR-006, SC-004

### Implementation

- [x] T011 [US3] Write Configuration Reference section in README.md — three subsections: (1) Provider (openai): api_key, model, language, prompt, timeout with types/defaults/validation from R-002. (2) Audio: device, sample_rate, silence_threshold, max_duration with ranges. (3) Post-processing: name, system_prompt, api_key, model, endpoint, timeout, temperature, max_tokens, max_retries with defaults/validation. Include Secret types explanation (!String, !FromEnv, !FromCommand) from src/config/secret.rs. Include complete post-processing example YAML from quickstart.md

**Checkpoint**: All 15 config fields documented with defaults and valid ranges. SC-004 verifiable.

---

## Phase 5: User Story 4 - Troubleshoot a problem (Priority: P3)

**Goal**: A user encountering a common error finds cause and resolution in the README

**Independent Test**: For each listed error, simulate the condition and verify README guidance resolves it

**Covers**: FR-007, SC-005

### Implementation

- [x] T012 [US4] Write Troubleshooting section in README.md — document 5 errors from R-004: (1) "No microphone detected" — connect mic, check ALSA permissions. (2) "Authentication failed" — verify API key, check env var. (3) "No speech detected" — speak louder, lower silence_threshold, check mic input. (4) "Network error" — check internet connection, firewall, DNS. (5) "Configuration file not found" — run `voice-type config init`. For each: error message, likely cause, resolution steps. Add tip about `-vv` for debug logging

**Checkpoint**: 5 error scenarios documented. SC-005 verifiable.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Final quality pass across all sections

- [x] T013 Review README.md for cross-references between sections — Quick Start links to Configuration Reference, CLI Reference links to Configuration Reference, Troubleshooting links to relevant config settings
- [x] T014 Verify completeness: all CLI commands from src/cli.rs covered (SC-003), all config fields from src/config/mod.rs and src/postprocess/config.rs covered (SC-004), at least 3 troubleshooting entries (SC-005)
- [x] T015 Proofread README.md for grammar, consistent formatting, and Markdown rendering correctness

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1 (Setup)**: No dependencies — start immediately
- **Phase 2 (US1)**: Depends on T002 (skeleton exists)
- **Phase 3 (US2)**: Depends on T002; T007 and T008 can run in parallel with Phase 2
- **Phase 4 (US3)**: Depends on T002; can run in parallel with Phases 2-3
- **Phase 5 (US4)**: Depends on T002; can run in parallel with Phases 2-4
- **Phase 6 (Polish)**: Depends on all user story phases complete

### User Story Dependencies

- **US1 (P1)**: No dependencies on other stories
- **US2 (P1)**: No dependencies on other stories; Quick Start may link to Features (US1) but not blocking
- **US3 (P2)**: No dependencies; standalone reference section
- **US4 (P3)**: No dependencies; standalone troubleshooting section

### Parallel Opportunities

- T007 and T008 can run in parallel (Prerequisites and Installation are independent sections)
- All user story phases (2-5) can technically run in parallel since they write to different README sections
- Within Phase 6, T013 and T015 are sequential (review before proofread)

---

## Implementation Strategy

### MVP First (User Stories 1 + 2)

1. Complete Phase 1: Setup (T001-T002)
2. Complete Phase 2: US1 — Discovery (T003-T006)
3. Complete Phase 3: US2 — Install & First Use (T007-T010)
4. **STOP and VALIDATE**: README is useful as-is for a new user to discover, install, and use the tool
5. Proceed to remaining stories

### Incremental Delivery

1. Setup → US1 (Discovery) → Minimal useful README exists
2. + US2 (Install & Use) → Complete onboarding path (MVP!)
3. + US3 (Config Reference) → Power user reference
4. + US4 (Troubleshooting) → Self-service problem resolution
5. Polish → Production-quality README

---

## Notes

- All content must be sourced from actual codebase (src/cli.rs, src/config/, src/error.rs, manifest.scm) — no guessing
- LICENSE file creation (T001) is a prerequisite discovered during research (R-006)
- README is a single file; "parallel" writing means working on independent sections
- Commit after each phase for incremental review

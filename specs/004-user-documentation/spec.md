# Feature Specification: User Documentation (README)

**Feature Branch**: `004-user-documentation`
**Created**: 2026-02-23
**Status**: Draft
**Input**: User description: "В проекте не хватает README.md - пользовательской документации."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Discover project purpose (Priority: P1)

A potential user finds the voice-type repository (e.g., on GitHub or via a search engine). They need to quickly understand what the project does, who it is for, and whether it solves their problem — all within the first 30 seconds of reading.

**Why this priority**: Without a clear project description, no one will try to install or use the software. This is the entry point for all other documentation.

**Independent Test**: Can be tested by showing the README to someone unfamiliar with the project and confirming they can describe what voice-type does within 30 seconds.

**Acceptance Scenarios**:

1. **Given** a user visits the project page, **When** they read the README header and description, **Then** they understand that voice-type is a Linux voice input tool that captures speech and converts it to text.
2. **Given** a user is evaluating alternatives, **When** they read the README, **Then** they can identify the key differentiators (GTK4 overlay, provider-agnostic design, configurable post-processing pipeline).

---

### User Story 2 - Install and run first session (Priority: P1)

A new user wants to install voice-type on their Linux system and successfully run their first voice-to-text session. They need clear, step-by-step instructions covering prerequisites, build steps, configuration, and first launch.

**Why this priority**: Equal to P1 because discovery without actionable setup instructions leads to immediate abandonment. Users must be able to go from zero to working in a single README flow.

**Independent Test**: Can be tested by following the README instructions on a fresh Linux system with the listed prerequisites and confirming the tool builds, runs, and transcribes speech.

**Acceptance Scenarios**:

1. **Given** a user has a supported Linux system with the listed prerequisites, **When** they follow the installation steps, **Then** the application builds and runs without errors.
2. **Given** a user has installed voice-type, **When** they follow the quick-start configuration guide, **Then** they can create a valid configuration file with their API key.
3. **Given** a user has a valid configuration, **When** they run the record command, **Then** they see transcribed text output.

---

### User Story 3 - Configure advanced features (Priority: P2)

An existing user wants to customize voice-type beyond the defaults — choosing a different audio device, setting a language hint, enabling post-processing, or tuning audio sensitivity thresholds.

**Why this priority**: Advanced configuration is important for power users but not essential for the first successful session. It expands the tool's usefulness after initial adoption.

**Independent Test**: Can be tested by modifying the configuration file according to README instructions and confirming the described behavior changes take effect.

**Acceptance Scenarios**:

1. **Given** a user reads the configuration reference section, **When** they want to change a setting, **Then** they find a description of each configurable option with its default value and acceptable range.
2. **Given** a user wants to enable post-processing, **When** they follow the post-processing configuration example, **Then** they can add a processor to their pipeline and see transformed output.

---

### User Story 4 - Troubleshoot a problem (Priority: P3)

A user encounters an issue (no audio device found, API key invalid, audio too quiet). They need guidance to diagnose and resolve common problems without filing a bug report.

**Why this priority**: Troubleshooting is a reactive need — most users will not encounter problems, but those who do need a clear path to resolution.

**Independent Test**: Can be tested by simulating common error conditions and confirming the README troubleshooting section addresses each one.

**Acceptance Scenarios**:

1. **Given** a user encounters a common error, **When** they consult the troubleshooting section, **Then** they find a description of the error, its likely cause, and a resolution step.

---

### Edge Cases

- What happens if a user is on a non-Linux platform? The README should clearly state Linux-only support upfront.
- What happens if a user has an older version of GTK4? Prerequisites section should specify minimum versions.
- What happens if a user does not use Guix? Build instructions should clarify the underlying system dependencies for non-Guix systems.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The README MUST contain a project title, one-line summary, and a short description (2-3 sentences) explaining what voice-type does and who it is for.
- **FR-002**: The README MUST list all system prerequisites (operating system, libraries, build tools) with minimum versions.
- **FR-003**: The README MUST provide step-by-step installation instructions that cover building from source.
- **FR-004**: The README MUST include a quick-start guide that walks through creating a configuration file, setting an API key, and running the first transcription.
- **FR-005**: The README MUST describe all CLI commands and their flags (`record`, `config validate`, `config show`, `config init`, `config docs`).
- **FR-006**: The README MUST include a configuration reference section covering all configurable options with defaults, types, and examples.
- **FR-007**: The README MUST have a troubleshooting section addressing at least 3 common errors (no audio device, invalid API key, audio too quiet / silence detection).
- **FR-008**: The README MUST include a license section referencing the project's MIT license.
- **FR-009**: The README MUST be written in English to maximize accessibility to the global developer community.

### Key Entities

- **README.md**: The root-level documentation file rendered by GitHub and other platforms as the project landing page.
- **Configuration file**: The YAML file (`~/.config/voice-type.yaml`) that users create and edit; referenced throughout the README.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user unfamiliar with the project can describe what voice-type does after reading the README for 30 seconds.
- **SC-002**: A user with the listed prerequisites can go from cloning the repository to a successful first transcription by following only the README, without consulting other sources.
- **SC-003**: 100% of CLI commands and subcommands documented in the codebase are covered in the README.
- **SC-004**: Every configurable option in the YAML configuration has a corresponding entry in the configuration reference section.
- **SC-005**: The troubleshooting section covers at least 3 common error scenarios with causes and resolutions.

## Assumptions

- The primary audience is Linux developers and power users comfortable with terminal tools and building from source.
- English is the documentation language (project has English code, commits, and comments).
- The README is the sole entry-point documentation; there is no separate docs site or wiki at this time.
- Guix is the primary build environment, but the README should also note the underlying system dependencies for non-Guix users.
- The configuration YAML format and CLI interface are stable and unlikely to change before the README is complete.

## Scope

### In Scope

- README.md file at the project root
- Project description, installation, quick-start, CLI reference, configuration reference, troubleshooting, and license sections

### Out of Scope

- Separate documentation website or wiki
- API documentation for the library crate (covered by `cargo doc`)
- Contributing guidelines (CONTRIBUTING.md)
- Changelog (CHANGELOG.md)
- Translations of the README into other languages

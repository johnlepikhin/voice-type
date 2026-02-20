# Feature Specification: Core Voice Input

**Feature Branch**: `001-core-voice-input`
**Created**: 2026-02-20
**Status**: Draft
**Input**: User description: "GTK-приложение для голосового ввода текста в Linux. Бэкенд — ChatGPT API (расширяемость для других провайдеров). YAML-конфиг (~/.config/voice-type.yaml). Развитая командная строка (clap). Режим демона с хоткеем для записи. Окно подтверждения. Вставка текста в активное окно."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - One-Shot Voice Transcription (Priority: P1)

A user launches the application, records a voice message, and receives
a text transcription displayed on screen. This is the core value
proposition — converting speech to text — without requiring daemon
mode, hotkeys, or text insertion into other applications.

The user starts the application from the terminal (or application
launcher), sees a recording window, speaks into the microphone,
stops recording, and views the transcribed text in the window. The
user can copy the text manually or dismiss the window.

**Why this priority**: Without speech-to-text transcription working
reliably, no other feature has value. This story validates the entire
audio capture → transcription → display pipeline end-to-end.

**Independent Test**: Launch the application, record a spoken phrase,
verify that the transcription appears in the window and matches the
spoken content with reasonable accuracy.

**Acceptance Scenarios**:

1. **Given** the application is launched and a microphone is available,
   **When** the user clicks "Start Recording" and speaks a phrase,
   **Then** the window displays a visual indicator that recording is
   in progress (e.g., elapsed time, audio level).

2. **Given** recording is in progress, **When** the user clicks
   "Stop Recording", **Then** the application sends the audio to the
   configured speech-to-text provider and displays a loading indicator.

3. **Given** audio has been sent for transcription, **When** the
   provider returns the result, **Then** the transcribed text is
   displayed in the window and the user can select and copy it.

4. **Given** the application is launched, **When** no microphone is
   detected, **Then** the application displays a clear error message
   explaining that a microphone is required.

5. **Given** recording is in progress, **When** a network or provider
   error occurs during transcription, **Then** the application
   displays a user-friendly error message with the option to retry.

---

### User Story 2 - Daemon Mode with Hotkey Recording and Text Insertion (Priority: P2)

A user runs the application as a background service (daemon). While
working in any application, the user presses a global hotkey to start
voice recording. A small overlay window appears showing recording
status. The user presses the hotkey again (or clicks a button) to
stop recording. The transcribed text appears in the overlay window.
The user reviews the text, optionally edits it, and confirms. The
text is then automatically inserted into the window that was active
before the overlay appeared.

**Why this priority**: This is the primary intended workflow —
hands-free voice input into any application. It depends on P1
(transcription pipeline) being functional and adds daemon lifecycle,
global hotkey listening, and text insertion into external windows.

**Independent Test**: Start the daemon, open a text editor, press
the hotkey, speak a phrase, confirm the transcription, and verify the
text appears at the cursor position in the text editor.

**Acceptance Scenarios**:

1. **Given** the daemon is running in the background, **When** the
   user presses the configured hotkey, **Then** a compact overlay
   window appears and audio recording begins immediately.

2. **Given** recording is in progress via hotkey, **When** the user
   presses the hotkey again, **Then** recording stops and
   transcription begins (loading indicator shown).

3. **Given** the transcription result is displayed in the overlay,
   **When** the user clicks "Confirm" (or presses Enter), **Then**
   the overlay closes and the transcribed text is inserted at the
   cursor position of the previously active window.

4. **Given** the transcription result is displayed in the overlay,
   **When** the user clicks "Cancel" (or presses Escape), **Then**
   the overlay closes without inserting any text.

5. **Given** the transcription result is displayed in the overlay,
   **When** the user edits the text before confirming, **Then** the
   edited version is inserted (not the original transcription).

6. **Given** the daemon is running, **When** the user presses the
   hotkey but the previously active window no longer exists, **Then**
   the application displays a warning and offers to copy the text to
   the clipboard instead.

7. **Given** the daemon is not running, **When** the user attempts
   to start a second daemon instance, **Then** the application
   detects the existing instance and reports that the daemon is
   already running (or not running, respectively).

---

### User Story 3 - Configuration and CLI Management (Priority: P3)

A user configures the application through a YAML configuration file
and manages the daemon via command-line interface. The configuration
file defines the speech-to-text provider, API credentials, hotkey
binding, audio settings, and UI preferences. The CLI provides
commands to start/stop the daemon, check status, validate
configuration, and perform a one-shot transcription.

**Why this priority**: Configuration and CLI are essential for
day-to-day usability but depend on P1 and P2 establishing the core
functionality first. A default configuration MUST work out of the
box so that P1 and P2 can be developed and tested without this story.

**Independent Test**: Create a configuration file, run CLI commands
to validate config, start the daemon, check status, and stop the
daemon. Change the provider in the config and verify the application
uses the new provider.

**Acceptance Scenarios**:

1. **Given** no configuration file exists, **When** the user launches
   the application for the first time, **Then** a default
   configuration file is created at `~/.config/voice-type.yaml`
   with sensible defaults and comments explaining each option.

2. **Given** a configuration file exists, **When** the user modifies
   the speech-to-text provider setting, **Then** the application uses
   the new provider on next launch (or after daemon restart).

3. **Given** a configuration file with an invalid value, **When** the
   user runs the "validate config" CLI command, **Then** the
   application reports the specific error with the line number and
   a suggested correction.

4. **Given** the daemon is running, **When** the user runs the
   "status" CLI command, **Then** the application displays the daemon
   PID, uptime, configured provider, and hotkey binding.

5. **Given** the user wants a custom config location, **When** the
   user passes `--config /path/to/config.yaml` on the command line,
   **Then** the application uses the specified file instead of the
   default location.

6. **Given** the user runs the application with `--help`, **When**
   the help output is displayed, **Then** all available commands and
   options are listed with descriptions and examples.

---

### Edge Cases

- What happens when the microphone is disconnected during recording?
  The application MUST detect the loss, stop recording gracefully,
  and display an error message. If any audio was captured, the user
  MUST be offered the option to transcribe the partial recording.

- What happens when the speech-to-text provider API key is missing
  or invalid? The application MUST display a clear error at startup
  (or on first transcription attempt) directing the user to configure
  credentials in the config file.

- What happens when the recording contains only silence? The
  application MUST detect silence (no meaningful audio) and inform
  the user rather than sending empty audio for transcription.

- What happens when the hotkey conflicts with another application?
  The application MUST report the conflict at daemon startup and
  suggest alternative hotkeys.

- What happens when network connectivity is lost during transcription?
  The application MUST display a timeout/network error and offer a
  retry option. The recorded audio MUST be preserved until the user
  dismisses the window.

- What happens when the transcription takes longer than expected?
  The application MUST show a progress/waiting indicator and allow
  the user to cancel the transcription without losing the recording.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST capture audio from the system microphone
  and send it to a configured speech-to-text provider for
  transcription.
- **FR-002**: System MUST display a window showing recording status
  (recording indicator, elapsed time) while audio capture is active.
- **FR-003**: System MUST display the transcribed text in a window
  where the user can review and edit it before accepting or
  dismissing.
- **FR-004**: System MUST operate as a background daemon that listens
  for a global hotkey to trigger the recording/transcription flow.
- **FR-005**: System MUST insert confirmed transcription text at the
  cursor position of the previously active window.
- **FR-006**: System MUST fall back to copying text to the clipboard
  when text insertion into the target window is not possible.
- **FR-007**: System MUST read configuration from a YAML file located
  at `~/.config/voice-type.yaml` by default, with the path
  overridable via a command-line flag.
- **FR-008**: System MUST generate a default configuration file with
  documented options when none exists.
- **FR-009**: System MUST provide a command-line interface with
  commands for: starting the daemon, stopping the daemon, checking
  daemon status, validating configuration, and performing a one-shot
  transcription.
- **FR-010**: System MUST support multiple speech-to-text providers
  through a pluggable architecture; the initial provider MUST be
  the OpenAI Whisper API (via ChatGPT API).
- **FR-011**: System MUST prevent multiple daemon instances from
  running simultaneously, using a lock mechanism.
- **FR-012**: System MUST validate the configuration file on startup
  and report specific errors (with location and suggestion) for
  invalid values.
- **FR-013**: System MUST display user-facing error messages for all
  failure modes (no microphone, network error, invalid API key,
  provider error) without exposing internal details.

### Key Entities

- **Recording**: A captured audio segment with duration, timestamp,
  and status (recording, processing, completed, failed). Associated
  with exactly one transcription attempt.
- **Transcription**: The text result of processing a recording,
  with the original text from the provider and optionally a
  user-edited version. Has a status (pending, succeeded, failed).
- **Provider**: A speech-to-text service configuration including
  the service identifier, endpoint, credentials, and
  provider-specific options. The system supports one active
  provider at a time.
- **Configuration**: The full set of user preferences including
  active provider, hotkey binding, audio input device, UI
  preferences, and file paths. Persisted as a YAML file.
- **DaemonState**: The runtime state of the background service
  including process identity, uptime, active provider, and
  current recording status (idle, recording, transcribing).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Users can complete a full voice-to-text cycle (start
  recording → speak → stop → receive transcription) in under
  10 seconds for a typical sentence (excluding speech duration).
- **SC-002**: Transcription accuracy matches the underlying
  speech-to-text provider's published accuracy rate (no degradation
  introduced by the application itself).
- **SC-003**: The overlay window appears within 500 milliseconds of
  the user pressing the hotkey.
- **SC-004**: Text insertion into the target window completes within
  1 second of the user confirming the transcription.
- **SC-005**: The daemon consumes less than 50 MB of memory while
  idle and less than 200 MB during active recording/transcription.
- **SC-006**: 95% of users can complete their first voice input
  without consulting documentation (after installing and configuring
  the API key).
- **SC-007**: The application starts (daemon mode) in under
  2 seconds.
- **SC-008**: Configuration validation reports all errors in a single
  pass (not one-at-a-time).

### Assumptions

- The user has a working microphone connected to their Linux system.
- The user has an active internet connection for cloud-based
  speech-to-text providers.
- The user has obtained API credentials for their chosen provider.
- The desktop environment supports global hotkey registration
  (X11 or Wayland with appropriate permissions).
- The target window for text insertion supports standard text input
  methods (X11 input simulation or Wayland input protocols).
- Default hotkey: `Super+V` (configurable by the user).
- Default audio format: the provider's preferred format to minimize
  conversion overhead.
- The application targets a single user running on a local machine
  (no multi-user or remote use cases).

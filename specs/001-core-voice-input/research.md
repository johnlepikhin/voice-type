# Research: Core Voice Input

## R1: GTK4 Daemon Architecture

**Decision**: Use `GtkApplication` with `ApplicationFlags::IS_SERVICE`
flag and `app.hold()`/`release()` for daemon mode.

**Rationale**: GTK4's Application has built-in D-Bus single-instance
enforcement and reference counting for keeping the app alive without
visible windows. No PID files or lock files needed — a second
instance sends `activate` signal to the first and exits.

**Key pattern**:
```rust
let app = gtk4::Application::builder()
    .application_id("com.voicetype.VoiceType")
    .flags(gio::ApplicationFlags::IS_SERVICE)
    .build();
app.connect_activate(|app| {
    app.hold(); // stay alive without windows
});
```

**Alternatives rejected**:
- Custom daemon + GTK client: over-engineered, IPC complexity
- Systemd socket activation: unnecessary for desktop app
- Separate D-Bus service: Application already provides D-Bus

## R2: Async Strategy

**Decision**: glib `MainContext` as primary runtime. HTTP on
background `std::thread` with `ureq` (blocking). Communication via
`async-channel`.

**Rationale**: GTK4 is single-threaded, tied to glib main loop. Two
runtimes (tokio + glib) add complexity. `ureq` is a blocking HTTP
client that eliminates tokio entirely. Pattern:

```rust
glib::spawn_future_local(async move {
    let (tx, rx) = async_channel::bounded(1);
    std::thread::spawn(move || {
        let result = ureq::post(url).send_multipart(form);
        tx.send_blocking(result).unwrap();
    });
    let response = rx.recv().await.unwrap();
    // update UI
});
```

**Alternatives rejected**:
- tokio + reqwest: adds ~2MB binary, two runtimes, footguns
- soup3 (GNOME HTTP client): less ecosystem support, harder to test

## R3: Overlay Window

**Decision**: `gtk4-layer-shell` for Wayland, undecorated `Window`
for X11 fallback.

**Rationale**: GTK4 removed window type hints. On Wayland,
`gtk4-layer-shell` provides wlr-layer-shell protocol for overlay
windows (`Layer::Top`). On X11, an undecorated window works.

**Alternatives rejected**:
- GDK4 popup surfaces: designed for menus, not persistent overlays
- X11-only approach: excludes Wayland users

## R4: Global Hotkey

**Decision**: `hotkey-listener` crate (evdev-based, works on both
X11 and Wayland). Fallback to `ashpd` GlobalShortcuts portal for
sandboxed environments.

**Rationale**: `hotkey-listener` uses evdev to read `/dev/input`
directly — works on X11 and Wayland. Supports modifier+key combos,
automatic keyboard reconnection. Requires `input` group membership.

**Reference**: `sotto` project uses raw `evdev` similarly but
`hotkey-listener` wraps this with a cleaner API.

**Alternatives rejected**:
- `global-hotkey` crate: X11-only on Linux
- `rdev`: requires root on Wayland for `unstable_grab`
- ashpd GlobalShortcuts only: not all compositors support it yet

**Risk**: User must be in `input` group. Document in quickstart.

## R5: Text Insertion

**Decision**: Clipboard-based approach as primary strategy.
Copy text to clipboard (`wl-copy`/`xclip`), then simulate Ctrl+V
(`wtype`/`xdotool`).

**Rationale**: Direct typing (`wtype --`) is simpler but only works
on wlroots-based Wayland compositors. Clipboard + paste is universal.

**Implementation**:
- Wayland: `wl-copy` + `wtype -M ctrl -k v`
- X11: `xclip -selection clipboard` + `xdotool key ctrl+v`
- Detect session type via `$XDG_SESSION_TYPE`

**Alternatives rejected**:
- Direct `xdotool type`: slow for long text, encoding issues
- `reis` (libei): still experimental, limited compositor support
- `ydotool`: requires root/uinput permissions

## R6: Audio Capture

**Decision**: `cpal` crate for cross-platform audio capture.
Record to WAV format (16kHz mono i16 PCM) for direct upload to
Whisper API.

**Rationale**: `cpal` is the most mature Rust audio crate. Uses
ALSA on Linux (via PipeWire/PulseAudio ALSA plugin). Supports
device enumeration and selection.

**VAD (Voice Activity Detection)**: Use simple RMS-based approach
(as in `voicsh` project). Calculate RMS of audio buffer, compare
against configurable threshold. No external VAD dependency needed
for V1 — user manually starts/stops recording.

**Alternatives rejected**:
- `gstreamer-rs`: heavy dependency for simple recording
- PipeWire native bindings: less portable, more complex
- `earshot` crate: interesting but v0.1.0, too early

## R7: Speech-to-Text Provider

**Decision**: OpenAI Whisper API via `ureq` HTTP client with
multipart form upload. Trait-based abstraction for provider
extensibility.

**API details**:
- Endpoint: `POST https://api.openai.com/v1/audio/transcriptions`
- Format: multipart form (`file`, `model`, `language`, `prompt`)
- Supported audio: wav, mp3, m4a, webm (max 25 MB)
- `language` field: ISO-639-1 hint for recognition accuracy
- `prompt` field: system-level hint to guide recognition style
- Model: `whisper-1`

**Trait pattern** (inspired by `voicsh`):
```rust
pub trait TranscriptionProvider: Send + Sync {
    fn transcribe(&self, audio: AudioData, opts: TranscribeOptions)
        -> Result<TranscriptionResult>;
}
```

**Config additions** (per user request):
- `language`: optional language hint (e.g., "ru", "en")
- `prompt`: optional system prompt for recognition context

## R8: Configuration Stack

**Decision**: `serde` + `serde_yaml` + `structdoc` + `humantime_serde`
for config. `Secret` enum from summitx for API keys. `clap` derive
for CLI.

**Key patterns**:
- `#[derive(Serialize, Deserialize, StructDoc)]` for config structs
- `#[serde(with = "humantime_serde")]` for Duration fields (e.g., `30s`)
- `Secret` enum with YAML tags: `!String`, `!FromEnv`, `!FromCommand`
- `#[serde(default)]` for optional fields with defaults

**Secret enum** (from `webapp_yaml_config/src/secret.rs`):
```rust
pub enum Secret {
    String(SecUtf8String),       // !String "raw-key"
    FromEnv(String),             // !FromEnv OPENAI_API_KEY
    FromCommand(String),         // !FromCommand "pass show openai"
}
```

**Dependencies**:
- `serde`, `serde_yaml` — serialization
- `structdoc` — config documentation generation
- `humantime_serde` — human-readable durations
- `secstr` — secure string storage
- `clap` (derive) — CLI argument parsing

## R9: System Environment

**Observed**:
- Rust 1.88.0 (stable)
- Guix System (not Debian/Fedora)
- X11 session (`$XDG_SESSION_TYPE=x11`)
- Available: `xdotool`, `xclip`
- Not available: `wtype`, `wl-copy`
- Not in `input` group (evdev needs this)
- GTK4 not found via `pkg-config` (needs installation)

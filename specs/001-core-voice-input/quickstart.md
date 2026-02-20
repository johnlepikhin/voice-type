# Quickstart: Core Voice Input

**Feature**: 001-core-voice-input
**Date**: 2026-02-20

## Prerequisites

### System Dependencies

**Guix System** (current dev environment):
```bash
guix shell -m manifest.scm -- cargo build --release
# Or install packages: guix install gtk pkg-config alsa-lib gcc-toolchain
```

**Debian/Ubuntu**:
```bash
sudo apt install libgtk-4-dev libgtk4-layer-shell-dev \
    libasound2-dev pkg-config xdotool xclip
```

**Fedora**:
```bash
sudo dnf install gtk4-devel gtk4-layer-shell-devel \
    alsa-lib-devel xdotool xclip
```

### Input Group (for global hotkeys)

The `hotkey-listener` crate uses evdev to read keyboard events
directly from `/dev/input`. The user must be in the `input` group:

```bash
sudo usermod -aG input $USER
# Log out and back in for group change to take effect
```

Verify:
```bash
groups | grep input
```

### Rust Toolchain

Rust 1.88+ (stable). Install via [rustup](https://rustup.rs/):
```bash
rustup update stable
```

### API Key

Obtain an OpenAI API key from https://platform.openai.com/api-keys

## Build

```bash
git clone <repo-url>
cd voice-type
cargo build --release
```

The binary is at `target/release/voice-type`.

## First Run

### 1. Initialize Configuration

```bash
voice-type config init
```

Creates `~/.config/voice-type.yaml` with defaults. Edit the file
to set your API key:

```yaml
provider:
  type: openai
  api_key: !FromEnv OPENAI_API_KEY
```

Set the environment variable:
```bash
export OPENAI_API_KEY="sk-..."
```

Or use a password manager:
```yaml
provider:
  type: openai
  api_key: !FromCommand "pass show openai/api-key"
```

### 2. Validate Configuration

```bash
voice-type config validate
```

### 3. One-Shot Transcription

```bash
voice-type record
```

A window appears. Click "Start Recording", speak, click "Stop".
The transcribed text appears in the window.

### 4. Daemon Mode

```bash
voice-type daemon &
```

Press `Shift+F8` (default hotkey) to start recording. Press again
to stop. Review text in overlay, click Confirm to insert into the
active window, or press Escape to cancel.

### 5. Check Status / Stop

```bash
voice-type status
voice-type stop
```

## Configuration Reference

Generate full configuration documentation:
```bash
voice-type config docs
```

## Troubleshooting

### "No microphone detected"
- Check `arecord -l` for available devices
- Set `audio.device` in config to the correct device name

### Hotkey not working
- Verify `input` group membership: `groups | grep input`
- Check for conflicting hotkeys in your desktop environment
- Try a different binding in `hotkey.binding` config

### "GTK4 not found" during build
- Install GTK4 dev packages (see System Dependencies above)
- Verify: `pkg-config --modversion gtk4`

### Text not inserted after confirmation
- X11: Verify `xdotool` and `xclip` are installed
- Wayland: Verify `wtype` and `wl-copy` are installed
- Check `$XDG_SESSION_TYPE` is set correctly

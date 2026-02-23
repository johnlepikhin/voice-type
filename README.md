# voice-type

Voice input for Linux — speak and get text via OpenAI Whisper.

## Description

voice-type is a command-line tool for Linux that records your voice and transcribes it to text using the OpenAI Whisper API. It displays a GTK4 overlay while recording and prints the transcribed text to stdout, making it easy to integrate with scripts, editors, and other tools. Designed for developers and power users who want hands-free text input on Linux.

## Features

- **Speech-to-text** via OpenAI Whisper API (`whisper-1` model)
- **GTK4 recording overlay** — visual feedback during recording with RMS level display
- **CLI-first design** — output to stdout for easy piping and scripting
- **YAML configuration** — all settings in a single `~/.config/voice-type.yaml` file
- **Post-processing pipeline** — chain LLM processors (grammar correction, translation, etc.)
- **Secure secret management** — API keys via environment variables (`!FromEnv`), shell commands (`!FromCommand`), or inline strings
- **Silence detection** — configurable RMS threshold to avoid sending empty audio
- **Auto-stop** — recording stops automatically at a configurable maximum duration
- **Provider-agnostic architecture** — extensible for future speech-to-text providers

## Prerequisites

- **Linux** (primary target platform)
- **Rust 1.88+** (stable)
- **System libraries**:
  - GTK 4
  - gtk-layer-shell
  - ALSA lib (`libasound2`)
  - pkg-config
  - GCC toolchain
- **OpenAI API key** with access to the Whisper API

## Installation

### Using Guix (recommended)

The repository includes a `manifest.scm` with all system dependencies:

```bash
git clone https://github.com/user/voice-type.git
cd voice-type
guix shell -m manifest.scm -- cargo build --release
```

The binary will be at `target/release/voice-type`.

### Manual build

Install system dependencies first:

**Debian/Ubuntu:**

```bash
sudo apt install libgtk-4-dev libgtk4-layer-shell-dev libasound2-dev pkg-config gcc
```

**Fedora:**

```bash
sudo dnf install gtk4-devel gtk4-layer-shell-devel alsa-lib-devel pkgconfig gcc
```

**Arch Linux:**

```bash
sudo pacman -S gtk4 gtk4-layer-shell alsa-lib pkgconf gcc
```

Then build:

```bash
git clone https://github.com/user/voice-type.git
cd voice-type
cargo build --release
```

Optionally, copy the binary to your PATH:

```bash
cp target/release/voice-type ~/.local/bin/
```

## Quick Start

1. **Create a default configuration file:**

   ```bash
   voice-type config init
   ```

   This creates `~/.config/voice-type.yaml` with sensible defaults.

2. **Set your OpenAI API key:**

   ```bash
   export OPENAI_API_KEY="sk-your-key-here"
   ```

   The default config reads the key from the `OPENAI_API_KEY` environment variable. To make this persistent, add the export to your shell profile (`~/.bashrc`, `~/.zshrc`, etc.).

3. **Validate your configuration:**

   ```bash
   voice-type config validate
   ```

4. **Record and transcribe:**

   ```bash
   voice-type record
   ```

   A GTK overlay window appears showing the recording state. Speak, then:
   - Press **Enter** to stop recording and transcribe
   - Press **Escape** to cancel
   - Recording auto-stops at the configured maximum duration (default: 5 minutes)

   The transcribed text is printed to stdout.

### Default configuration

```yaml
provider:
  openai:
    api_key: !FromEnv OPENAI_API_KEY
    model: whisper-1
    timeout: 30s

audio:
  sample_rate: 16000
  silence_threshold: 0.01
  max_duration: 5min
```

## Usage

### CLI Reference

```
voice-type [OPTIONS] <COMMAND>
```

**Global options:**

| Option | Description |
|--------|-------------|
| `-c, --config <PATH>` | Path to configuration file (default: `~/.config/voice-type.yaml`) |
| `-v, --verbose` | Increase logging verbosity (repeat for more: `-v`, `-vv`, `-vvv`) |

**Commands:**

#### `record`

Record voice and print transcription to stdout.

```bash
voice-type record [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `-d, --device <NAME>` | Audio input device name (overrides config) |
| `-l, --language <CODE>` | Language hint, ISO-639-1 code (overrides config) |
| `-p, --prompt <TEXT>` | Recognition prompt (overrides config) |

Examples:

```bash
# Basic recording
voice-type record

# Record with language hint
voice-type record -l en

# Record from specific device with verbose logging
voice-type -vv record -d "HDA Intel PCH"
```

#### `config`

Configuration management subcommands.

| Subcommand | Description |
|------------|-------------|
| `config validate` | Validate the configuration file |
| `config show` | Show the current effective configuration |
| `config init` | Create a default configuration file |
| `config init --force` | Overwrite an existing configuration file |
| `config docs` | Print configuration documentation |

Examples:

```bash
# Initialize config
voice-type config init

# Show current config
voice-type config show

# Validate config with custom path
voice-type -c /path/to/config.yaml config validate

# View all configuration options
voice-type config docs
```

### Configuration Reference

The configuration file is YAML, located at `~/.config/voice-type.yaml` by default. Run `voice-type config docs` to see all options, or `voice-type config show` to view your current settings.

#### Provider (OpenAI)

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `provider.openai.api_key` | Secret (required) | — | API key for OpenAI Whisper. See [Secret Management](#secret-management) |
| `provider.openai.model` | String | `"whisper-1"` | Whisper model name |
| `provider.openai.language` | String (optional) | — | ISO-639-1 language hint (e.g., `"en"`, `"ru"`) — must be 2 lowercase letters |
| `provider.openai.prompt` | String (optional) | — | Context prompt for recognition style |
| `provider.openai.timeout` | Duration | `30s` | Request timeout (humantime format: `30s`, `1min`, etc.) |

#### Audio

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `audio.device` | String (optional) | system default | Audio input device name |
| `audio.sample_rate` | Integer | `16000` | Sample rate in Hz (valid: 8000–48000) |
| `audio.silence_threshold` | Float | `0.01` | RMS silence threshold (valid: 0.0–1.0). Lower = more sensitive |
| `audio.max_duration` | Duration | `5min` | Maximum recording duration (humantime format) |

#### Post-Processing Pipeline

An optional list of LLM processors applied sequentially to the transcribed text. Each processor sends the text to a chat completions API with the given system prompt.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | String (required) | — | Human-readable name for progress display |
| `system_prompt` | String (required) | — | System prompt sent to the LLM |
| `api_key` | Secret (required) | — | API key for the chat completions provider |
| `model` | String | `"gpt-4o-mini"` | Model name |
| `endpoint` | String | `"https://api.openai.com"` | Base endpoint URL |
| `timeout` | Duration | `15s` | Request timeout |
| `temperature` | Float (optional) | — | LLM temperature (valid: 0.0–2.0) |
| `max_tokens` | Integer (optional) | — | Maximum tokens in response (must be > 0) |
| `max_retries` | Integer | `3` | Retry attempts for transient errors (valid: 0–10) |

**Example — grammar correction:**

```yaml
post_processing:
  - name: Grammar
    system_prompt: "Fix grammar and punctuation. Return only the corrected text."
    api_key: !FromEnv OPENAI_API_KEY
    model: gpt-4o-mini
    temperature: 0.3
```

### Secret Management

API keys are specified using YAML tags:

| Tag | Description | Example |
|-----|-------------|---------|
| `!FromEnv` | Read from an environment variable | `api_key: !FromEnv OPENAI_API_KEY` |
| `!FromCommand` | Read from a shell command's stdout | `api_key: !FromCommand "pass show openai/api-key"` |
| `!String` | Inline plaintext (not recommended) | `api_key: !String "sk-..."` |

`!FromEnv` is recommended for most setups. `!FromCommand` is useful with password managers like `pass` or `1password-cli`.

## Troubleshooting

### "No microphone detected"

**Cause:** No audio input device is available to the system.

**Fix:**
- Ensure a microphone is connected
- Check that your user has permission to access audio devices (`audio` group on most distros)
- Verify ALSA sees the device: `arecord -l`
- If using a specific device, set [`audio.device`](#audio) in your config

### "Authentication failed"

**Cause:** The OpenAI API key is invalid, expired, or not set.

**Fix:**
- Verify the environment variable is set: `echo $OPENAI_API_KEY`
- Check that the key is valid at [platform.openai.com](https://platform.openai.com/api-keys)
- If using `!FromCommand`, test the command manually: run it in your shell and verify it outputs the key

### "No speech detected in the recording"

**Cause:** The recorded audio was too quiet or contained only silence.

**Fix:**
- Speak louder or move closer to the microphone
- Lower the [`audio.silence_threshold`](#audio) in your config (e.g., from `0.01` to `0.005`)
- Check that the correct microphone is being used ([`audio.device`](#audio))
- Test your mic: `arecord -d 3 test.wav && aplay test.wav`

### "Network error"

**Cause:** Cannot reach the OpenAI API. No internet connection, DNS failure, or firewall blocking.

**Fix:**
- Check your internet connection
- Verify DNS resolution: `nslookup api.openai.com`
- Check if a proxy or firewall is blocking HTTPS traffic
- Increase the timeout in your config if on a slow connection

### "Configuration file not found"

**Cause:** The config file does not exist at the expected path.

**Fix:**
- Create a default config: `voice-type config init`
- Or specify a custom path: `voice-type -c /path/to/config.yaml record`

### General debugging

Enable verbose logging to see detailed information:

```bash
# Info level
voice-type -v record

# Debug level
voice-type -vv record

# Trace level (very verbose)
voice-type -vvv record
```

You can also use the `RUST_LOG` environment variable for fine-grained control:

```bash
RUST_LOG=voice_type=debug voice-type record
```

## License

This project is licensed under the [MIT License](LICENSE).

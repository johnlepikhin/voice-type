# Quick Start: User Documentation

This document describes the quick-start flow that will appear in the README.

## Flow

1. **Install** — Build from source (Guix or manual)
2. **Initialize config** — `voice-type config init`
3. **Set API key** — Edit `~/.config/voice-type.yaml`, set `api_key: !FromEnv OPENAI_API_KEY`
4. **Export key** — `export OPENAI_API_KEY="sk-..."`
5. **Validate** — `voice-type config validate`
6. **Record** — `voice-type record`
7. **Result** — Transcribed text printed to stdout

## Expected default config (from `AppConfig::default_yaml()`)

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

## User interaction during recording

- **Overlay appears** — GTK4 overlay window shows recording state
- **Enter** — Stop recording, transcribe, print result
- **Escape** — Cancel recording (exit code 1)
- **Auto-stop** — Recording stops automatically at `max_duration`

## Post-processing (optional advanced step)

Add to config:

```yaml
post_processing:
  - name: Grammar
    system_prompt: "Fix grammar and punctuation. Return only the corrected text."
    api_key: !FromEnv OPENAI_API_KEY
    model: gpt-4o-mini
```

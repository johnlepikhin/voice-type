# Research: User Documentation (README)

## R-001: README Section Structure

**Decision**: Use a linear structure: Header → Description → Features → Prerequisites → Installation → Quick Start → CLI Reference → Configuration Reference → Troubleshooting → License.

**Rationale**: Follows the standard open-source README flow (discovery → action → reference). Users read top-down; the most common journey (discover → install → use) maps directly to section order.

**Alternatives considered**:
- Wiki-style multi-page docs → Rejected: overkill for a single-binary CLI tool; spec explicitly excludes separate docs site
- Minimal "install + usage" only → Rejected: doesn't meet FR-006 (config reference) or FR-007 (troubleshooting)

## R-002: Configuration Options Inventory

**Decision**: Document all options from `AppConfig`, `AudioConfig`, `OpenAiProviderConfig`, `PostProcessorConfig` structs with defaults and valid ranges extracted from source code.

**Findings** (from codebase analysis):

### Provider (OpenAI)
| Field | Type | Default | Validation |
|-------|------|---------|------------|
| `api_key` | Secret (!FromEnv, !FromCommand, !String) | required | N/A |
| `model` | String | `"whisper-1"` | non-empty |
| `language` | LanguageCode (optional) | None | 2 lowercase ASCII letters |
| `prompt` | String (optional) | None | N/A |
| `timeout` | Duration | `30s` | humantime format |

### Audio
| Field | Type | Default | Validation |
|-------|------|---------|------------|
| `device` | String (optional) | system default | N/A |
| `sample_rate` | SampleRate | `16000` Hz | 8000..=48000 |
| `silence_threshold` | RmsLevel | `0.01` | 0.0..=1.0 (clamped) |
| `max_duration` | Duration | `5min` | humantime format |

### Post-processing (per processor)
| Field | Type | Default | Validation |
|-------|------|---------|------------|
| `name` | ProcessorName | required | non-empty |
| `system_prompt` | String | required | non-empty |
| `api_key` | Secret | required | N/A |
| `model` | String | `"gpt-4o-mini"` | non-empty |
| `endpoint` | String | `"https://api.openai.com"` | N/A |
| `timeout` | Duration | `15s` | humantime format |
| `temperature` | f32 (optional) | None | 0.0..=2.0 |
| `max_tokens` | u32 (optional) | None | > 0 |
| `max_retries` | u32 | `3` | 0..=10 |

**Rationale**: Source code is the authoritative reference; documenting from structs ensures accuracy.

## R-003: CLI Commands Inventory

**Decision**: Document all commands from `cli.rs`.

**Findings**:

```
voice-type [OPTIONS] <COMMAND>

Options:
  -c, --config <PATH>    Config file path [default: ~/.config/voice-type.yaml]
  -v, --verbose          Increase logging verbosity (-v, -vv, -vvv)

Commands:
  record                 Record voice and print transcription to stdout
    -d, --device <NAME>  Audio input device (overrides config)
    -l, --language <CODE> Language hint, ISO-639-1 (overrides config)
    -p, --prompt <TEXT>  Recognition prompt (overrides config)

  config validate        Validate configuration file
  config show            Show current effective configuration
  config init [--force]  Create default configuration file
  config docs            Print configuration documentation
```

## R-004: Common Error Scenarios

**Decision**: Document the 5 most user-facing errors with causes and fixes.

**Findings** (from `error.rs`):

| Error | Message | Cause | Fix |
|-------|---------|-------|-----|
| No microphone | "No microphone detected..." | No audio input device available | Connect microphone, check permissions |
| Auth failure | "Authentication failed..." | Invalid/expired API key | Verify API key in config, check env var |
| No speech | "No speech detected..." | Audio too quiet or silence only | Speak louder, lower `silence_threshold`, check mic |
| Network error | "Network error: ..." | No internet or DNS failure | Check internet connection |
| Config not found | "Configuration file not found: ..." | Missing config file | Run `voice-type config init` |

## R-005: Build Dependencies

**Decision**: Document both Guix and manual dependency installation.

**Findings** (from `manifest.scm` and `Cargo.toml`):

**System dependencies**: GTK4, gtk-layer-shell, pkg-config, ALSA lib, GCC toolchain
**Rust**: 1.88+ (from `Cargo.toml` `rust-version` field)
**Guix command**: `guix shell -m manifest.scm -- cargo build --release`

## R-006: License

**Decision**: Reference MIT license (declared in `Cargo.toml`). Note: LICENSE file does not exist yet — task should include creating it.

**Rationale**: `Cargo.toml` declares `license = "MIT"` but no LICENSE file is present in the repository.

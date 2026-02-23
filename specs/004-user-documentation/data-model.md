# Data Model: User Documentation (README)

This feature produces a single Markdown file. The "data model" is the document structure.

## README.md Section Model

```
README.md
├── Header (title + badges)
├── Description (what + who + why)
├── Features (bullet list of capabilities)
├── Prerequisites
│   ├── Operating system (Linux)
│   ├── Rust version (1.88+)
│   ├── System libraries (GTK4, gtk-layer-shell, ALSA, pkg-config)
│   └── API key (OpenAI)
├── Installation
│   ├── Guix (primary)
│   └── Manual (system packages + cargo build)
├── Quick Start
│   ├── Create config (voice-type config init)
│   ├── Set API key
│   └── First recording (voice-type record)
├── CLI Reference
│   ├── Global options (-c, -v)
│   ├── record command + flags
│   └── config subcommands (validate, show, init, docs)
├── Configuration Reference
│   ├── provider.openai.*
│   ├── audio.*
│   └── post_processing[].*
├── Troubleshooting
│   ├── No microphone detected
│   ├── Authentication failed
│   ├── No speech detected
│   ├── Network errors
│   └── Config file not found
└── License (MIT)
```

## Key Entities

| Entity | Source of Truth | README Section |
|--------|----------------|----------------|
| CLI commands & flags | `src/cli.rs` (`Cli`, `Commands`, `ConfigCommands`) | CLI Reference |
| Config options | `src/config/mod.rs` (`AppConfig`, `AudioConfig`, `OpenAiProviderConfig`) | Configuration Reference |
| Post-processor options | `src/postprocess/config.rs` (`PostProcessorConfig`) | Configuration Reference |
| Secret types | `src/config/secret.rs` (`Secret` enum) | Configuration Reference |
| Newtypes & ranges | `src/types.rs` (`SampleRate`, `RmsLevel`, `LanguageCode`) | Configuration Reference |
| Error messages | `src/error.rs` (all error enums) | Troubleshooting |
| Default config YAML | `AppConfig::default_yaml()` | Quick Start |
| System deps | `manifest.scm` | Prerequisites |

## Relationships

- Quick Start references → Prerequisites (must be met first)
- Quick Start references → Configuration Reference (for details)
- CLI Reference references → Configuration Reference (config overrides)
- Troubleshooting references → Configuration Reference (settings to adjust)

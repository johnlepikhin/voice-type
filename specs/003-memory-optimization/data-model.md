# Data Model: Buffer & Resource Lifecycle

**Date**: 2026-02-20

## Buffer Lifecycle States

```text
                   ┌──────────┐
                   │ NotAlloc │  (startup / idle)
                   └────┬─────┘
                        │ recording starts
                        ▼
                   ┌──────────┐
                   │ Growing  │  audio samples accumulate
                   └────┬─────┘
                        │ recording stops
                        ▼
                   ┌──────────┐
                   │ Consumed │  encode_wav() reads samples → WAV bytes
                   └────┬─────┘
                        │ samples dropped, WAV consumed by multipart builder
                        ▼
                   ┌──────────┐
                   │ Sending  │  multipart body held during HTTP request
                   └────┬─────┘
                        │ HTTP response received / error
                        ▼
                   ┌──────────┐
                   │ Released │  all buffers dropped, back to idle
                   └──────────┘
```

## Resource Ownership Map (current → target)

| Resource | Current Owner | Current Lifetime | Target Lifetime |
|----------|--------------|------------------|-----------------|
| Audio `Vec<i16>` | `Arc<Mutex<Vec<i16>>>` in AudioCapture | Recording start → implicit scope drop | Recording start → explicit drop after WAV encode |
| WAV `Vec<u8>` | Return value of `encode_wav()` | WAV encode → implicit scope drop | WAV encode → explicit drop after multipart build |
| Multipart body `Vec<u8>` | Return value of `build_multipart_body()` | Multipart build → implicit scope drop | Multipart build → explicit drop after HTTP response |
| HTTP Agent (provider) | `OpenAiWhisperProvider` field | Daemon start → daemon exit | Same (needed for connection reuse) |
| HTTP Agent (post-proc) | Per-`ChatCompletionsClient` field | Pipeline creation → daemon exit | First recording → daemon exit (lazy) |
| COMMAND_CACHE | Global `LazyLock<HashMap>` | First secret access → process exit | Same (bounded, minimal overhead) |

## Key Entities (unchanged from spec)

- **Audio buffer**: `Vec<i16>`, 2 bytes/sample, bounded by max_duration config
- **WAV payload**: `Vec<u8>`, ~same size as audio buffer + 44-byte header
- **HTTP agent**: `ureq::Agent`, connection pool with idle timeout
- **Processing pipeline**: `Vec<PostProcessor>`, each wrapping a `ChatCompletionsClient`

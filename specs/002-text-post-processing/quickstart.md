# Quickstart: Text Post-Processing Pipeline

## Minimal Config

Add to `~/.config/voice-type/config.yaml`:

```yaml
post_processing:
  - name: Grammar
    system_prompt: "Fix grammar and punctuation. Return only the corrected text."
    api_key: !FromEnv OPENAI_API_KEY
    model: gpt-4o-mini
```

## Multi-Processor Pipeline

```yaml
post_processing:
  - name: Grammar
    system_prompt: "Fix grammar and punctuation. Return only the corrected text."
    api_key: !FromEnv OPENAI_API_KEY
    model: gpt-4o-mini
    temperature: 0.2

  - name: Formalize
    system_prompt: "Rewrite the following text in a formal business tone. Return only the rewritten text."
    api_key: !FromEnv OPENAI_API_KEY
    model: gpt-4o-mini
    temperature: 0.3
    max_tokens: 1024
```

## Validate Config

```bash
guix shell -m manifest.scm -- cargo run -- config validate
```

## Expected Behavior

1. User presses hotkey → recording starts
2. User presses hotkey again → recording stops, transcription begins
3. Transcription completes → overlay shows "Step 1/2: Grammar..."
4. Grammar processor completes → overlay shows "Step 2/2: Formalize..."
5. Formalize processor completes → overlay shows processed text for review
6. User confirms → processed text is inserted

If any processor fails, the overlay shows the original transcribed text with an error notification.

## Build & Test

```bash
guix shell -m manifest.scm -- cargo clippy --all-targets -- -D warnings
guix shell -m manifest.scm -- cargo test
```

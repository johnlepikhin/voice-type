# Contract: Pipeline Progress Messages (Internal)

## Channel Protocol

Background thread → GTK main loop via `std::sync::mpsc::Sender<PipelineProgress>`.

### Message Types

```
StepStarted { index: usize, total: usize, name: String }
  → Overlay: show_processing(index + 1, total, &name)
  → Status: "Step {index+1}/{total}: {name}..."

Done { text: String }
  → Continue to AwaitingConfirmation with processed text

Failed { processor_name: String, error: String, original_text: String }
  → Show error notification with processor name
  → Continue to AwaitingConfirmation with original_text
```

### Sequence (success, 2 processors)

```
1. StepStarted { index: 0, total: 2, name: "Grammar" }
2. StepStarted { index: 1, total: 2, name: "Translate" }
3. Done { text: "<final processed text>" }
```

### Sequence (failure at step 2)

```
1. StepStarted { index: 0, total: 2, name: "Grammar" }
2. StepStarted { index: 1, total: 2, name: "Translate" }
3. Failed { processor_name: "Translate", error: "HTTP 429: Rate limit exceeded", original_text: "<original transcription>" }
```

### Sequence (no processors configured)

No messages sent. Pipeline returns `PipelineResult::Skipped` immediately.

# Data Model: Text Post-Processing Pipeline

## Entities

### PostProcessorConfig

A single post-processor configuration entry from the YAML config file.

| Field | Type | Required | Default | Validation |
|-------|------|----------|---------|------------|
| name | String (newtype) | Yes | — | Non-empty |
| system_prompt | String | Yes | — | Non-empty |
| api_key | Secret | Yes | — | Resolvable |
| model | String | Yes | — | Non-empty |
| endpoint | String | No | OpenAI default | Valid URL |
| timeout | Duration | No | 15s | > 0 |
| temperature | f32 | No | None (provider default) | 0.0..=2.0 |
| max_tokens | u32 | No | None (provider default) | > 0 |

**Relationships**: Belongs to `AppConfig.post_processing` (ordered list, 0..N).

### ProcessorName

Newtype over String. Mandatory, non-empty. Used in overlay progress display ("Step 1/3: Grammar") and error messages.

**Validation**: Non-empty string, validated at config load time.

### PostProcessor (runtime)

Constructed from `PostProcessorConfig`. Holds a ureq Agent (with per-processor timeout), API key, model, endpoint, system prompt, and optional LLM parameters.

| Field | Type | Source |
|-------|------|--------|
| name | ProcessorName | config.name |
| agent | ureq::Agent | Constructed with config.timeout |
| api_key | Secret | config.api_key |
| model | String | config.model |
| endpoint | String | config.endpoint or default |
| system_prompt | String | config.system_prompt |
| temperature | Option\<f32\> | config.temperature |
| max_tokens | Option\<u32\> | config.max_tokens |

**Behavior**: `fn process(&self, text: &str) -> Result<String, PostProcessingError>` — sends chat completion request, returns response content.

### ProcessingPipeline (runtime)

Ordered sequence of `PostProcessor` instances. Constructed from `Vec<PostProcessorConfig>`.

| Field | Type |
|-------|------|
| processors | Vec\<PostProcessor\> |

**Behavior**: `fn run(&self, text: &str, progress: &Sender<PipelineProgress>) -> PipelineResult` — runs each processor sequentially, sending progress updates.

### PipelineProgress (message enum)

Messages sent from background thread to GTK main loop during pipeline execution.

| Variant | Fields | Purpose |
|---------|--------|---------|
| StepStarted | index: usize, total: usize, name: String | Update overlay progress |
| Done | text: String | Pipeline completed successfully |
| Failed | processor_name: String, error: String, original_text: String | Processor failed, fallback |

### PipelineResult (return type)

| Variant | Fields |
|---------|--------|
| Processed | text: String |
| Skipped | text: String |
| Failed | original_text: String, processor_name: String, error: PostProcessingError |

### PostProcessingError

| Variant | Fields | Maps from |
|---------|--------|-----------|
| NetworkError | message: String | ureq transport errors |
| AuthenticationError | — | HTTP 401 |
| ProviderError | status: u16, message: String | HTTP non-2xx with body |
| Timeout | — | ureq timeout |
| EmptyResponse | — | Empty content in response |

## State Transitions

### DaemonPhase (extended)

```
Idle → Recording → Transcribing → PostProcessing → AwaitingConfirmation
                                  ↓ (on error)
                                  AwaitingConfirmation (with original text)
```

New state: `PostProcessing` — entered after successful transcription when post-processors are configured. The overlay shows "Step X/N: Name..." during this phase.

When no post-processors configured: `Transcribing → AwaitingConfirmation` (unchanged).

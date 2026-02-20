# Feature Specification: Text Post-Processing Pipeline

**Feature Branch**: `002-text-post-processing`
**Created**: 2026-02-20
**Status**: Draft
**Input**: User description: "Необходимо спроектировать фичу пост-обработки распознанного текста. Пусть этот текст прогоняется через один или несколько (последовательно) обработчиков, которые задаются в конфиге. Для каждого обработчика необходима возможность задать системный промпт."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Single Processor for Grammar Correction (Priority: P1)

A user configures a single post-processor with a system prompt like "Fix grammar and punctuation in the following text. Return only the corrected text." After dictating speech, the transcribed text is automatically passed through this processor before insertion. The user receives grammatically correct text without manual editing.

**Why this priority**: This is the core value proposition — even a single processor delivers immediate quality improvement to raw transcription output.

**Independent Test**: Can be fully tested by configuring one processor, dictating text with intentional grammar errors, and verifying the output is corrected before insertion.

**Acceptance Scenarios**:

1. **Given** a config with one post-processor defined with a grammar-correction system prompt, **When** the user completes a voice recording and transcription succeeds, **Then** the transcribed text is sent to the processor and the corrected result is used for insertion.
2. **Given** a config with one post-processor defined, **When** the processor returns an empty response, **Then** the system falls back to the original transcribed text and warns the user.
3. **Given** a config with no post-processors defined, **When** transcription completes, **Then** the text is inserted as-is (current behavior preserved).

---

### User Story 2 - Sequential Multi-Processor Pipeline (Priority: P2)

A user configures multiple post-processors that run in sequence. For example: first processor corrects grammar, second processor translates the text, third processor formats it for a specific context (e.g., formal email). Each processor receives the output of the previous one.

**Why this priority**: Composable pipelines unlock advanced use cases (translation chains, domain-specific formatting) but require the single-processor foundation from P1.

**Independent Test**: Can be tested by configuring two or more processors, dictating text, and verifying each processor's output feeds into the next, with the final result used for insertion.

**Acceptance Scenarios**:

1. **Given** a config with three post-processors A → B → C, **When** transcription completes, **Then** the text flows through A, then B receives A's output, then C receives B's output, and C's output is inserted.
2. **Given** a pipeline of two processors where the first one fails, **When** transcription completes, **Then** the pipeline stops, the original transcribed text is used for insertion, and the user is notified of the failure.

---

### User Story 3 - Pipeline Error Visibility (Priority: P3)

When a post-processor fails (network error, authentication failure, rate limit), the user sees a clear indication of which processor failed and why, while still receiving the original transcribed text (partial pipeline results are discarded).

**Why this priority**: Error transparency is important for debugging pipelines but the system is still usable without it (P1/P2 already handle fallback behavior).

**Independent Test**: Can be tested by configuring a processor with an invalid API key, dictating text, and verifying the error message identifies the failing processor.

**Acceptance Scenarios**:

1. **Given** a pipeline where processor 2 of 3 fails with a network error, **When** the user views the result, **Then** they see the original transcribed text inserted and a notification identifying processor 2 as the failure point with the error reason.

---

### Edge Cases

- What happens when a processor returns text identical to its input? The pipeline continues normally — no special handling needed.
- What happens when a processor's system prompt is empty? Validation rejects the config at load time.
- What happens when the transcribed text itself is very long (e.g., 5 minutes of speech)? The text is sent as-is to each processor; any size limits are governed by the provider's own constraints.
- What happens when a processor responds very slowly? Each processor respects the timeout configured for it; on timeout the pipeline aborts and falls back to the original text.
- What happens when the user has post-processors configured but the transcription itself fails? Post-processing is skipped entirely — no processors are invoked.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST support defining zero or more post-processors in the configuration file, each with a mandatory name and system prompt.
- **FR-002**: System MUST execute post-processors sequentially in the order they appear in the configuration.
- **FR-003**: Each post-processor MUST receive a system prompt (from config) and the text to process (as user message).
- **FR-004**: The output of each post-processor MUST be passed as input to the next processor in the pipeline.
- **FR-005**: The final processor's output MUST be used as the text for insertion.
- **FR-006**: When no post-processors are configured, the system MUST insert the raw transcribed text (preserving current behavior).
- **FR-007**: When any processor in the pipeline fails, the system MUST fall back to the original transcribed text (before any post-processing).
- **FR-008**: When a processor fails, the system MUST notify the user with the processor's name and the error reason.
- **FR-009**: Each post-processor MUST have a configurable timeout independent of the transcription timeout.
- **FR-010**: System MUST validate post-processor configuration at config load time, rejecting entries with empty system prompts or empty names.
- **FR-012**: During post-processing, the system MUST display step-by-step progress in the overlay (e.g., "Step 1/3: Grammar..."), updating as each processor completes.
- **FR-013**: Each post-processor MUST support optional `temperature` and `max_tokens` parameters to control LLM output determinism and length.
- **FR-011**: Each post-processor MUST have its own independent provider configuration (API key, model, endpoint). The initial implementation supports only OpenAI chat completions; additional provider types may be added in future iterations.

### Key Entities

- **PostProcessor**: A single text transformation step. Has a mandatory name, system prompt, provider configuration, timeout, and optional LLM parameters (temperature, max_tokens). The name is used in progress display and error messages. Receives text input and produces text output.
- **ProcessingPipeline**: An ordered sequence of zero or more PostProcessors. Executes them sequentially, threading text through each step.
- **ProcessingResult**: The outcome of running the pipeline — either the final processed text or a fallback to the original with error details.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Users can configure and run a single post-processor that transforms transcribed text in under 5 seconds of additional latency for typical utterances (1-2 sentences).
- **SC-002**: A pipeline of 3 processors executes sequentially and produces the expected composed output.
- **SC-003**: When any processor fails, the user receives the original transcription within 1 second (after timeout), with a visible error notification.
- **SC-004**: Configuration with zero post-processors produces identical behavior to the system before this feature was added (no regression).

## Assumptions

- Post-processors will use LLM chat completion APIs (not transcription APIs). Each processor has its own provider configuration, allowing heterogeneous pipelines across different LLM providers.
- System prompts are static strings defined in config, not dynamically generated.
- Post-processing happens synchronously on the same background thread as transcription, before the text reaches the UI for insertion.
- A reasonable default timeout for each post-processor is 15 seconds.

## Clarifications

### Session 2026-02-20

- Q: On pipeline failure, should the system fall back to original transcribed text or to the last successful processor output? → A: Always fall back to original transcribed text (discard partial results).
- Q: Which chat completion providers to support in MVP? → A: Only OpenAI chat completions. Additional providers in future iterations.
- Q: Should the user see UI feedback during post-processing? → A: Show step-by-step progress in the overlay (e.g., "Step 1/3: Grammar...").
- Q: Should each processor have a user-defined name? → A: Yes, mandatory `name` field. Used in progress display and error messages.
- Q: Should processors support LLM parameters beyond system prompt? → A: Yes, optional `temperature` and `max_tokens` for output determinism and length control.

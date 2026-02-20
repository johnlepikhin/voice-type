# Contract: OpenAI Chat Completions (Post-Processing)

## Request

```
POST {endpoint}/v1/chat/completions
Authorization: Bearer {api_key}
Content-Type: application/json
```

### Request Body

```json
{
  "model": "{model}",
  "messages": [
    {
      "role": "system",
      "content": "{system_prompt}"
    },
    {
      "role": "user",
      "content": "{text_to_process}"
    }
  ],
  "temperature": 0.3,
  "max_tokens": 2048
}
```

- `temperature` and `max_tokens` are optional; omit from JSON when not configured.
- `model` is required (from processor config).
- `messages` always has exactly 2 entries: system prompt + user text.

## Response (Success — 2xx)

```json
{
  "id": "chatcmpl-...",
  "object": "chat.completion",
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "content": "{processed_text}"
      },
      "finish_reason": "stop"
    }
  ],
  "usage": {
    "prompt_tokens": 42,
    "completion_tokens": 38,
    "total_tokens": 80
  }
}
```

Extract: `choices[0].message.content` — this is the processed text passed to the next pipeline step.

If `content` is empty or `choices` is empty → `PostProcessingError::EmptyResponse`.

## Response (Error — non-2xx)

```json
{
  "error": {
    "message": "You exceeded your current quota...",
    "type": "insufficient_quota",
    "code": "insufficient_quota"
  }
}
```

Mapping:
- 401 → `PostProcessingError::AuthenticationError`
- Other non-2xx → `PostProcessingError::ProviderError { status, message }` (extract `error.message`, fall back to raw body)

## Timeout

ureq agent configured with per-processor timeout (`http_status_as_error(false)`, same pattern as transcription provider).

On timeout → `PostProcessingError::Timeout`.

## Config YAML Contract

```yaml
post_processing:                     # Optional, default: [] (empty)
  - name: Grammar                    # Required, non-empty
    system_prompt: |                 # Required, non-empty
      Fix grammar and punctuation in the following text.
      Return only the corrected text.
    api_key: !FromEnv OPENAI_API_KEY # Required, Secret type
    model: gpt-4o-mini               # Required, non-empty
    # endpoint: https://api.openai.com  # Optional, default: https://api.openai.com
    # timeout: 15s                   # Optional, default: 15s
    # temperature: 0.3               # Optional, 0.0..=2.0
    # max_tokens: 2048               # Optional, > 0
```

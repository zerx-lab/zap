# Migration Guide: v0.3.x → v0.4.0-alpha.x

genai 0.4.0 is a big release, with important new features, a 

This guide highlights the key API changes between the 0.3.x and 0.4.0-alpha.x lines of `genai`, and provides concrete steps to adapt existing applications.

## New Features Worth Opting Into

- **MessageContent**: Provide a simpler and multi part structure for message content, simplifying the API and make it more flexible to support some protocol features. 
- **Embedding Support**: `EmbedOptionsSet` merges client/request-level options, making multi-tenant setups easier.
- **Structured tool streaming**: capture tool calls incrementally with `ChatOptions::with_capture_tool_calls(true)`.
- **Raw body capture**: `ChatOptions::with_capture_raw_body(true)` and `EmbedOptions::with_capture_raw_body(true)` expose provider responses for debugging.
- **Ability to have headers**: `ChatOptions::default().with_extra_headers(custom_headers)`
- **Transparent gpt-5-codex with Responses API**: `gpt-5-codex` API is only supported via the new OpenAI Responses API for now (2025-09-28), and since 0.4.x version, a new adapter `OpenAIResp` now support the subset needed to make those model supported transparently via the chat apis. 


## Quick Checklist

- Update your dependency to `genai = "0.4.0-alpha.18"` (or newer in the `0.4.0-alpha.x` range).
- Replace usages of `ChatResponse::content_text_*` helpers with the new `first_text`, `texts`, or `into_texts` accessors.
- Adjust any pattern-matching on `MessageContent` variants; it is now a struct wrapping `ContentPart` values.
- Update tool-call handling (`ChatResponse::into_tool_calls` now always returns a `Vec<ToolCall>`).
- Replace image payload helpers with the new `Binary`/`ContentPart::Binary` types.
- Adopt the updated streaming event model if you rely on tool-call streaming (`ChatStreamEvent::ToolCallChunk`).
- Review new `ChatOptions` knobs (e.g., `capture_tool_calls`, `verbosity`, `extra_headers`) and remove references to deprecated builders.
- (Optional) Integrate the new embedding improvements and header utilities (`EmbedOptionsSet`, `Headers`, `WebConfig`) if you customize HTTP behaviour.

## Breaking API Changes

### `ChatResponse` content is always present

- **0.3.x**: `ChatResponse::content` was an `Option<MessageContent>` and helper methods were prefixed with `content_...`.
- **0.4.0**: `ChatResponse::content` is always a `MessageContent` (which may be empty). Helpers have been renamed for clarity.

| 0.3.x API                                             | 0.4.0 replacement                             |
|-------------------------------------------------------|-----------------------------------------------|
| `chat_res.content_text_as_str()`                      | `chat_res.first_text()`                       |
| `chat_res.content_text_into_string()`                 | `chat_res.into_first_text()`                  |
| `chat_res.content.into_texts()`                       | `chat_res.into_texts()`                       |
| `chat_res.tool_calls() -> Option<Vec<&ToolCall>>`     | `chat_res.tool_calls() -> Vec<&ToolCall>`     |
| `chat_res.into_tool_calls() -> Option<Vec<ToolCall>>` | `chat_res.into_tool_calls() -> Vec<ToolCall>` |

When a provider returns no assistant text, `chat_res.content.is_text_empty()` now indicates emptiness.

### `MessageContent` is now a collection of `ContentPart`

- **0.3.x**: `MessageContent` was an enum (`Text`, `Parts`, `ToolCalls`, `ToolResponses`).
- **0.4.0**: `MessageContent` is a struct that wraps a `Vec<ContentPart>`.
  - Use builders such as `MessageContent::from_text("...")`, `MessageContent::from_parts(parts)`, or `MessageContent::from_tool_calls(vec)`.
  - Iterate with `content.parts()`, convert with `content.into_parts()`.

Pattern matching on enum variants will no longer compile. Prefer the new query helpers:

```rust
let texts = chat_res.content.texts();
let has_tool_calls = chat_res.content.contains_tool_call();
```

### Binary & image payloads use the new `Binary` type

- **0.3.x**: `ContentPart::Image { content_type, source }` with `ImageSource`.
- **0.4.0**: `ContentPart::Binary(Binary)` generalises non-text payloads (images, PDFs, etc.).

```rust
use genai::chat::{Binary, ContentPart, MessageContent};

let image = Binary::new(
    "image/png",
    Binary::from_base64("...base64 data..."),
    Some("diagram.png"),
);

let content = MessageContent::from_parts(vec![
    ContentPart::from("Please analyse this diagram."),
    ContentPart::from(image),
]);
```

`Binary` offers helpers (`is_image`, `is_pdf`, `into_url`) to improve provider compatibility.

### Streaming events now surface tool-call chunks

`ChatStreamEvent` gained a `ToolCallChunk` variant that emits incremental tool-call information as providers stream reasoning. If you iterate over stream events, add handling for the new variant:

```rust
while let Some(event) = stream.next().await {
    match event? {
        ChatStreamEvent::ToolCallChunk(chunk) => { /* inspect chunk.tool_call */ }
        _ => {}
    }
}
```

### Expanded `ChatOptions`

`ChatOptions` is still builder-oriented but adds new toggles:

- `with_capture_tool_calls(bool)`
- `with_capture_raw_body(bool)`
- `with_verbosity(Verbosity)` (new enum)
- `with_seed(u64)`
- `with_extra_headers(Headers)`

Deprecated helpers from 0.3.x:

- `with_json_mode(bool)` remains but only as a compatibility shim. Prefer `with_response_format(ChatResponseFormat::JsonMode)`.

If you previously relied on provider-specific headers, migrate to the new `Headers` helper:

```rust
use genai::{chat::ChatOptions, Headers};

let options = ChatOptions::default()
    .with_extra_headers(Headers::from([
        ("x-custom-header", "value"),
    ]));
```

### Reasoning effort and verbosity parsing

- `ReasoningEffort` now includes a `Minimal` variant and improved parsing logic.
- A new `Verbosity` enum (Low, Medium, High) controls provider-specific reasoning verbosity.

If you parse strings into these enums, update any validation tables to include the new keywords.

### Tool-call helper conversions

`ToolResponse` and `ToolCall` now integrate more deeply:

- `ToolResponse` implements `Into<ChatMessage>` and `Into<MessageContent>` for convenience.
- `ToolCall`/`ToolResponse` are also directly convertible into `ContentPart`.

This affects code that previously constructed `ChatMessage::assistant(MessageContent::from_tool_calls(...))`; the direct `ToolCall` to `ChatMessage` conversions may be simpler, but existing approaches continue to work with the updated builder functions.

### Resolver and header utilities

- New `Headers` struct standardises single-value headers and merging behaviour.
- `ClientBuilder::with_web_config` accepts a `WebConfig` to control timeouts and proxy settings.
- `AuthData::RequestOverride` now stores headers as `Headers` instead of `Vec<(String, String)>`. If you constructed overrides manually, update the type.

```rust
use genai::resolver::AuthData;
use genai::Headers;

let auth = AuthData::RequestOverride {
    url: "https://custom.example/v1/chat".into(),
    headers: Headers::from([
        ("Authorization", "Bearer ..."),
    ]),
};
```

## Migration Recipes

### Retrieving assistant text

```rust
// 0.3.x
let maybe_text = chat_res.content_text_as_str();

// 0.4.0
let maybe_text = chat_res.first_text();
```

To collect all segments:

```rust
let joined = chat_res.content.joined_texts();
let all = chat_res.texts(); // Vec<&str>
```

### Handling missing responses

```rust
// 0.3.x
if let Some(content) = chat_res.content {
    if let Some(text) = content.text_as_str() { ... }
}

// 0.4.0
if let Some(text) = chat_res.first_text() { ... }
// or, if you need to ensure real output:
if !chat_res.content.is_text_empty() { ... }
```

### Tool-call aggregation

```rust
// 0.3.x
if let Some(tool_calls) = chat_res.tool_calls() {
    for call in tool_calls { ... }
}

// 0.4.0
for call in chat_res.tool_calls() {
    // Always a Vec; empty when none emitted.
}
```

### Message construction with attachments

```rust
// 0.3.x
use genai::chat::{ContentPart, MessageContent};

let content = MessageContent::from_parts(vec![
    ContentPart::Text("Describe the image".into()),
    ContentPart::Image {
        content_type: "image/png".into(),
        source: ImageSource::from_base64("..."),
    },
]);

// 0.4.0
use genai::chat::{Binary, ContentPart, MessageContent};

let binary = Binary::new(
    "image/png",
    Binary::from_base64("..."),
    None,
);

let content = MessageContent::from_parts(vec![
    ContentPart::from("Describe the image"),
    ContentPart::from(binary),
]);
```

### Streaming loop upgrade

```rust
// add handling for streamed tool calls (new in 0.4.0)
match event? {
    ChatStreamEvent::ToolCallChunk(chunk) => handle_tool_chunk(chunk.tool_call),
    ChatStreamEvent::Chunk(chunk) => handle_text(chunk.content),
    ChatStreamEvent::ReasoningChunk(chunk) => handle_reasoning(chunk.content),
    ChatStreamEvent::End(end) => handle_end(end),
    ChatStreamEvent::Start => {}
}
```


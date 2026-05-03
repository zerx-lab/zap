## chat

### Goal

The `chat` module provides the core primitives for constructing chat requests, defining messages (including multi-part content like text, binary, and tool data), and handling synchronous and asynchronous (streaming) chat responses across all supported AI providers. It standardizes the data structures necessary for modern LLM interactions.

### Public Module API

The module exports the following key data structures:

- **Request/Message Structure:**
  - `ChatRequest`: The primary structure for initiating a chat completion call, containing the history (`messages`), an optional system prompt (`system`), and tool definitions (`tools`).
  - `ChatMessage`: Represents a single interaction turn, comprising a `ChatRole`, `MessageContent`, and optional `MessageOptions`.
  - `ChatRole`: Enum defining message roles (`System`, `User`, `Assistant`, `Tool`).
  - `MessageContent`: A unified container for multi-part content, wrapping a list of `ContentPart`s.
  - `ContentPart`: Enum defining content types: `Text`, `Binary`, `ToolCall`, `ToolResponse`.
  - `Binary`, `BinarySource`: Structures defining binary payloads (e.g., images), sourced via base64 or URL.
  - `MessageOptions`, `CacheControl`: Per-message configuration hints (e.g., for cache behavior).

- **Configuration:**
  - `ChatOptions`: General request configuration, including sampling parameters (`temperature`, `max_tokens`, `top_p`, `seed`), streaming capture flags, and format control.
  - `ReasoningEffort`, `Verbosity`: Provider-specific hints for reasoning intensity or output verbosity.
  - `ChatResponseFormat`, `JsonSpec`: Defines desired structured output formats (e.g., JSON mode).

- **Responses:**
  - `ChatResponse`: The result of a non-streaming request, including final content, usage, and model identifiers.
  - `ChatStreamResponse`: The result wrapper for streaming requests, containing the `ChatStream` and model identity.

- **Streaming:**
  - `ChatStream`: A `futures::Stream` implementation yielding `ChatStreamEvent`s.
  - `ChatStreamEvent`: Enum defining streaming events: `Start`, `Chunk` (content), `ReasoningChunk`, `ToolCallChunk`, and `End`.
  - `StreamEnd`: Terminal event data including optional captured usage, content, and reasoning content.

- **Tooling:**
  - `Tool`: Metadata and schema defining a function the model can call.
  - `ToolCall`: The model's invocation request for a specific tool.
  - `ToolResponse`: The output returned from executing a tool, matched by call ID.

- **Metadata:**
  - `Usage`, `PromptTokensDetails`, `CompletionTokensDetails`: Normalized token usage statistics.

- **Utilities:**
  - `printer` module: Contains `print_chat_stream` for console output utilities.

### Module Parts

The functionality is divided into specialized files/sub-modules:

- `chat_message.rs`: Defines the `ChatMessage` fundamental structure and associated types (`ChatRole`, `MessageOptions`).
- `chat_options.rs`: Manages request configuration (`ChatOptions`) and provides parsing logic for provider-specific hints like `ReasoningEffort` and `Verbosity`.
- `chat_req_response_format.rs`: Handles configuration for structured output (`ChatResponseFormat`, `JsonSpec`).
- `chat_request.rs`: Defines the top-level `ChatRequest` and methods for managing the request history and properties.
- `chat_response.rs`: Defines synchronous chat response structures (`ChatResponse`).
- `chat_stream.rs`: Implements the public `ChatStream` and its events, mapping from the internal adapter stream.
- `content_part.rs`: Defines `ContentPart`, `Binary`, and `BinarySource` for handling multi-modal inputs/outputs.
- `message_content.rs`: Defines `MessageContent`, focusing on collection management and convenient accessors for content parts (e.g., joining all text).
- `tool/mod.rs` (and associated files): Defines the tooling primitives (`Tool`, `ToolCall`, `ToolResponse`).
- `usage.rs`: Defines the normalized token counting structures (`Usage`).
- `printer.rs`: Provides utility functions for rendering stream events to standard output.

### Key Design Considerations

- **Unified Content Model:** The use of `MessageContent` composed of `ContentPart` allows any message role (user, assistant, tool) to handle complex, multi-part data seamlessly, including text, binary payloads, and tooling actions.
- **Decoupled Streaming:** The public `ChatStream` is an abstraction layer over an internal stream (`InterStream`), ensuring a consistent external interface regardless of adapter implementation details (like internal handling of usage reporting or reasoning chunks).
- **Normalized Usage Metrics:** The `Usage` structure provides an OpenAI-compatible interface while allowing for provider-specific breakdowns (e.g., caching or reasoning tokens) via detailed sub-structures.
- **Hierarchical Options:** `ChatOptions` can be applied globally at the client level or specifically per request. The internal resolution logic ensures request-specific options take precedence over client defaults.

### Implemented API Notes

- `ChatRequest` also supports stateful session helpers:
  - `with_previous_response_id(...)`
  - `with_store(...)`
  - `From<Vec<ChatMessage>>`

- `ChatMessage` includes:
  - `ChatMessage::new(role, content)` as the generic constructor
  - role-aware constructors like `system(...)`, `user(...)`, `assistant(...)`, and `tool(...)`
  - `with_reasoning_content(...)`
  - `assistant_tool_calls_with_thoughts(...)`

- `MessageOptions` includes:
  - `MessageOptions::new()`
  - `with_cache_control(...)`

- `MessageContent` includes additive collection and convenience helpers such as:
  - `iter()` and `iter_mut()`
  - `from_tool_responses(...)`
  - singular `From<ToolCall>`
  - first-item helpers for text, reasoning content, and thought signatures
  - `contains_binary()`

- `ChatOptions` includes both:
  - `with_stop_sequences(...)` for replacement-style bulk assignment
  - `with_stop_sequence(...)` for appending a single stop sequence

- `ReasoningEffort` currently includes:
  - `None`
  - `Low`
  - `Medium`
  - `High`
  - `XHigh`
  - `Max`
  - `Budget(u32)`
  - legacy `Minimal`

## adapter

### Goal

The `adapter` module is responsible for abstracting the communication with various Generative AI providers (e.g., OpenAI, Gemini, Anthropic, Groq, DeepSeek). It translates generic GenAI requests (like `ChatRequest` and `EmbedRequest`) into provider-specific HTTP request data and converts provider-specific web responses back into generic GenAI response structures. It acts as the translation and dispatch layer between the client logic and the underlying web communication.

### Public Module API

The primary public API exposed by the `adapter` module is:

- `AdapterKind`: An enum identifying the AI provider or protocol type (e.g., `OpenAI`, `Gemini`, `Anthropic`, `Cohere`). This type is used by the client and resolver layers to determine which adapter implementation should handle a specific model request.

### Module Parts

- `adapter_kind.rs`: Defines the `AdapterKind` enum. It includes implementation details for serialization, environment variable name resolution, and a default static mapping logic (`from_model`) to associate model names with a specific `AdapterKind`.

- `adapter_types.rs`: Defines the `Adapter` trait, which sets the contract for all concrete adapter implementations. It also defines common types like `ServiceType` (Chat, ChatStream, Embed) and `WebRequestData` (the normalized structure holding URL, headers, and payload before web execution).

- `dispatcher.rs`: Contains the `AdapterDispatcher` struct, which acts as the central routing mechanism. It dispatches calls from the client layer to the correct concrete adapter implementation based on the resolved `AdapterKind`.

- `inter_stream.rs`: Defines internal types (`InterStreamEvent`, `InterStreamEnd`) used by streaming adapters to standardize the output format from diverse provider streaming protocols. This intermediary layer handles complex stream features like capturing usage, reasoning content, and tool calls before conversion to public `ChatStreamResponse` events.

- `adapters/`: This submodule contains the concrete implementation of the `Adapter` trait for each provider (e.g., `openai`, `gemini`, `anthropic`, `zai`). These submodules handle the specific request/response translation logic for their respective protocols.

### Key Design Considerations

- **Stateless and Static Dispatch:** Adapters are designed to be stateless, with all methods in the `Adapter` trait being associated functions (static). Requests are routed efficiently using static dispatch through the `AdapterDispatcher`, minimizing runtime overhead and simplifying dependency management.

- **Request/Response Normalization:** The adapter layer ensures that incoming requests and outgoing responses conform to generic GenAI types, hiding provider-specific implementation details from the rest of the library.

- **Dynamic Resolution:** While `AdapterKind::from_model` provides a default mapping from model names (based on common prefixes or keywords), the system allows this to be overridden by custom `ServiceTargetResolver` configurations, enabling flexible routing (e.g., mapping a custom model name to an `OpenAI` adapter with a custom endpoint).

- **Stream Intermediation:** The introduction of `InterStreamEvent` is crucial for handling the variance in streaming protocols across providers. it ensures that complex data transmitted at the end of a stream (like final usage statistics or aggregated tool calls) can be correctly collected and normalized, regardless of the provider's specific event format.

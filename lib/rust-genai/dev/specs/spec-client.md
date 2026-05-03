## client

### Goal

The `client` module provides the core entry point (`Client`) for interacting with various Generative AI providers. It encapsulates configuration (`ClientConfig`, `WebConfig`), a builder pattern (`ClientBuilder`), request execution (`exec_chat`, `exec_embed`), and service resolution logic (e.g., determining endpoints and authentication).

### Public Module API

The `client` module exposes the following public types:

- **`Client`**: The main interface for executing AI requests (chat, embedding, streaming, model listing).
  - `Client::builder()`: Starts the configuration process.
  - `Client::default()`: Creates a client with default configuration.
  - Core execution methods: `exec_chat`, `exec_chat_stream`, `exec_embed`, `embed`, `embed_batch`.
  - Resolution/Discovery methods: `all_model_names`, `resolve_service_target`.

- **`ClientBuilder`**: Provides a fluent interface for constructing a `Client`. Used to set `ClientConfig`, default `ChatOptions`, `EmbedOptions`, and custom resolvers (`AuthResolver`, `ServiceTargetResolver`, `ModelMapper`).

- **`ClientConfig`**: Holds the resolved and default configurations used by the `Client`, including resolver functions and default options.

- **`Headers`**: A simple map wrapper (`HashMap<String, String>`) for managing HTTP headers in requests.

- **`ServiceTarget`**: A struct containing the final resolved components needed to execute a request: `Endpoint`, `AuthData`, and `ModelIden`.

- **`WebConfig`**: Configuration options specifically for building the underlying `reqwest::Client` (e.g., timeouts, proxies, default headers).

### Module Parts

The module is composed of several files that implement the layered client architecture:

- `builder.rs`: Implements `ClientBuilder`, handling the creation and configuration flow. It initializes or updates the nested `ClientConfig` and optionally an internal `WebClient`.

- `client_types.rs`: Defines the main `Client` struct and `ClientInner` (which holds `WebClient` and `ClientConfig` behind an `Arc`).

- `config.rs`: Defines `ClientConfig` and the core `resolve_service_target` logic, which orchestrates calls to `ModelMapper`, `AuthResolver`, and `ServiceTargetResolver` before falling back to adapter defaults.

- `client_impl.rs`: Contains the main implementation of the public API methods on `Client`, such as `exec_chat` and `exec_embed`. These methods perform service resolution and delegate to `AdapterDispatcher` for request creation and response parsing.

- `headers.rs`: Implements the `Headers` utility for managing key-value HTTP header maps.

- `service_target.rs`: Defines the `ServiceTarget` structure for resolved endpoints, authentication, and model identifiers.

- `web_config.rs`: Defines `WebConfig` and its logic for applying settings to a `reqwest::ClientBuilder`.

### Key Design Considerations

- **Client Immutability and Sharing**: The `Client` holds its internal state (`ClientInner` with `WebClient` and `ClientConfig`) wrapped in an `Arc`. This design ensures that the client is thread-safe and cheaply cloneable, aligning with common client patterns in asynchronous Rust applications.

- **Config Layering and Resolution**: The client architecture employs a sophisticated resolution process managed by `ClientConfig::resolve_service_target`.
  - It first applies a `ModelMapper` to potentially translate the input model identifier.
  - It then consults the `AuthResolver` for authentication data. If the resolver is absent or returns `None`, it defaults to the adapter's standard authentication mechanism (e.g., API key headers).
  - It determines the adapter's default endpoint.
  - Finally, it applies the optional `ServiceTargetResolver`, allowing users to override the endpoint, auth, or model for complex scenarios (e.g., custom proxies or routing).

- **WebClient Abstraction**: The core HTTP client logic is delegated to the `WebClient` (from the `webc` module), which handles low-level request execution and streaming setup. This separation keeps the `client` module focused on business logic and AI provider orchestration.

- **Builder Pattern for Configuration**: `ClientBuilder` enforces configuration before client creation, simplifying object construction and ensuring necessary dependencies are set up correctly.

- **Headers Simplification**: The `Headers` struct abstracts HTTP header management, ensuring that subsequent merges or overrides result in a single, final header value, which is typical for API key authorization overrides.

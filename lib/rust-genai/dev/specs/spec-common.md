## common

### Goal

The `common` module provides fundamental data structures used throughout the `genai` library, primarily focusing on identifying models and adapters in a clear and efficient manner.

### Public Module API

The module exposes two main types: `ModelName` and `ModelIden`.

- `ModelName`: Represents a generative AI model identifier (e.g., `"gpt-4o"`, `"claude-3-opus"`).
  - It wraps an `Arc<str>` for efficient cloning and sharing across threads.
  - Implements `From<String>`, `From<&String>`, `From<&str>`, and `Deref<Target = str>`.
  - Supports equality comparison (`PartialEq`) with various string types (`&str`, `String`).

- `ModelIden`: Uniquely identifies a model by coupling an `AdapterKind` with a `ModelName`.
  - Fields:
    - `adapter_kind: AdapterKind`
    - `model_name: ModelName`
  - Constructor: `fn new(adapter_kind: AdapterKind, model_name: impl Into<ModelName>) -> Self`
  - Utility methods for creating new identifiers based on name changes:
    - `fn from_name<T>(&self, new_name: T) -> ModelIden`
    - `fn from_optional_name(&self, new_name: Option<String>) -> ModelIden`

### Module Parts

The `common` module consists of:

- `model_name.rs`: Defines the `ModelName` type and related string manipulation utilities, including parsing optional namespaces (e.g., `namespace::model_name`).
- `model_iden.rs`: Defines the `ModelIden` type, which associates a `ModelName` with an `AdapterKind`.

### Key Design Considerations

- **Efficiency of ModelName:** `ModelName` uses `Arc<str>` to ensure that cloning the model identifier is cheap, which is crucial as model identifiers are frequently passed around in request and response structures.
- **Deref Implementation:** Implementing `Deref<Target = str>` for `ModelName` allows it to be used naturally as a string reference.
- **ModelIden Immutability:** `ModelIden` is designed to be immutable and fully identifiable, combining the model string identity (`ModelName`) with the service provider identity (`AdapterKind`).

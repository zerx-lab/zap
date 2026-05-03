//! ZAI API Documentation
//! API Documentation:     <https://api.z.ai>
//! Model Names:           GLM series models
//! Pricing:               <https://api.z.ai/pricing>
//!
//! ## Dual Endpoint Support
//!
//! ZAI supports two different API endpoints using the ServiceTargetResolver pattern:
//!
//! ### Regular API (Credit-based) (default for those models or with `zai::` namespace)
//! - Endpoint: `<https://api.z.ai/api/paas/v4/>`
//! - Models: `glm-4.6`, `glm-4.5`, etc.
//! - Usage: Standard API calls billed per token
//!
//! ### Coding Plan (Subscription-based only with the `zai-coding::` namepace)  
//! - Endpoint: `<https://api.z.ai/api/coding/paas/v4/>`
//! - Models: `zai-coding::glm-4.6`, `zai-coding::glm-4.5`, etc.
//! - Usage: Fixed monthly subscription for coding tasks
//!
//! ## For example
//!
//! ```rust
//! use genai::resolver::{Endpoint, ServiceTargetResolver};
//! use genai::{Client, AdapterKind, ModelIden};
//!
//! let client = Client::builder().with_service_target_resolver(target_resolver).build();
//!
//! // Use regular API
//! let response = client.exec_chat("glm-4.6", chat_request, None).await?;
//! // Same, regular API
//! let response = client.exec_chat("zai::glm-4.6", chat_request, None).await?;
//!
//! // Use coding plan
//! let response = client.exec_chat("zai-coding::glm-4.6", chat_request, None).await?;
//! ```
//!
//! See `examples/c07-zai-dual-endpoints.rs` for a complete working example.

// region:    --- Modules

mod adapter_impl;

pub use adapter_impl::*;

// endregion: --- Modules

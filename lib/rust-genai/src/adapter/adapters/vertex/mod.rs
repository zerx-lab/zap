//! Google Vertex AI (Model Garden) adapter.
//! Supports multiple publishers: Google (Gemini) and Anthropic (Claude).
//!
//! API Documentation:
//!   - Gemini on Vertex: https://cloud.google.com/vertex-ai/generative-ai/docs/model-reference/inference
//!   - Claude on Vertex: https://cloud.google.com/vertex-ai/generative-ai/docs/partner-models/use-claude
//!
//! Usage: namespace model names with `vertex::`, e.g. `vertex::gemini-2.5-flash` or `vertex::claude-sonnet-4-6`

// region:    --- Modules

mod adapter_impl;

pub use adapter_impl::*;

// endregion: --- Modules

//! Tooling primitives for chat function-calling.
//!
//! This module exposes the core traits and data structures to define tools,
//! represent tool invocations emitted by models, and carry tool execution results.
//! The concrete types are implemented in submodules and re-exported here for
//! ergonomic importing.

// region:    --- Modules

mod tool_base;
mod tool_call;
mod tool_response;
mod tool_types;
mod web_search_config;

pub use tool_base::*;
pub use tool_call::*;
pub use tool_response::*;
pub use tool_types::*;
pub use web_search_config::*;

// endregion: --- Modules

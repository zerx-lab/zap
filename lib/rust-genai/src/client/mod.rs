//! Client module.
//!
//! Re-exports the public client API: builder, client types, configuration,
//! headers, service targets, and web configuration utilities.

// region:    --- Modules

mod builder;
mod client_impl;
mod client_types;
mod config;
mod headers;
mod model_spec;
mod service_target;
mod web_config;

pub use builder::*;
pub use client_types::*;
pub use config::*;
pub use headers::*;
pub use model_spec::*;
pub use service_target::*;
pub use web_config::*;

// endregion: --- Modules

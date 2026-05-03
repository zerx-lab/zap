//! Resolvers are hooks that library users can set to customize aspects of the library's default behavior.
//! A good example is the AuthResolver, which provides the authentication data (e.g., api_key).
//!
//! Eventually, the library will have more resolvers.

// region:    --- Modules

mod auth_data;
mod auth_resolver;
mod endpoint;
mod error;
mod model_mapper;
mod service_target_resolver;

pub use auth_data::*;
pub use auth_resolver::*;
pub use endpoint::*;
pub use error::{Error, Result};
pub use model_mapper::*;
pub use service_target_resolver::*;

// endregion: --- Modules

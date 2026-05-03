//! The GenAI web client construct that uses reqwest. Only `webc::Error` is exposed as the public interface.

// region:    --- Modules

mod error;
mod event_source_stream;
mod web_client;
mod web_stream;

pub(crate) use error::Result;
pub(crate) use event_source_stream::*;
pub(crate) use web_client::*;
pub(crate) use web_stream::*;

// Only public for external use
pub use error::Error;

// endregion: --- Modules

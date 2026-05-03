//! API DOC:     <https://docs.cohere.com/reference/chat>
//! MODEL NAMES: <https://docs.cohere.com/docs/models>
//! PRICING:     <https://cohere.com/pricing>

// region:    --- Modules

mod adapter_impl;
mod embed;
mod streamer;

pub use adapter_impl::*;
pub use streamer::*;

// endregion: --- Modules

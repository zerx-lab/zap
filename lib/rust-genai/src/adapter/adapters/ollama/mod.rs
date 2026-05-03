//! Ollama native adapter implementation.
//! OPENAI API DOC: <https://platform.openai.com/docs/api-reference/chat>
//! OLLAMA API DOC: <https://github.com/ollama/ollama/blob/main/docs/api.md>
//!  OLLAMA Models: <https://ollama.com/library>

// region:    --- Modules

mod adapter_impl;
mod adapter_shared;
mod streamer;

pub use adapter_impl::*;
#[allow(unused_imports)]
pub use adapter_shared::*;
pub use streamer::*;

// endregion: --- Modules

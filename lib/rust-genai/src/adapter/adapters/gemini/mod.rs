//! API Documentation:     <https://ai.google.dev/api/rest/v1beta/models/generateContent>
//! Model Names:           <https://ai.google.dev/gemini-api/docs/models/gemini>
//! Pricing:               <https://ai.google.dev/pricing>

// region:    --- Modules

mod adapter_impl;
mod embed;
mod openapi_schema;
mod streamer;

pub use adapter_impl::*;
pub use streamer::*;

// endregion: --- Modules

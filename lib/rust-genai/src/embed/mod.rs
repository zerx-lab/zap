//! The genai embed module contains all of the constructs necessary
//! to make embedding requests with the `genai::Client`.

// region:    --- Modules

mod embed_options;
mod embed_request;
mod embed_response;

// -- Flatten
pub use embed_options::*;
pub use embed_request::*;
pub use embed_response::*;

// endregion: --- Modules

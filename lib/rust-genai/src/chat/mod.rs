//! Chat primitives and utilities for building requests and streaming responses.
//! Intended for use with `genai::Client`

// region:    --- Modules

mod binary;
mod chat_message;
mod chat_options;
mod chat_req_response_format;
mod chat_request;
mod chat_response;
mod chat_stream;
mod content_part;
mod message_content;
mod tool;
mod usage;

// -- Flatten
pub use binary::*;
pub use chat_message::*;
pub use chat_options::*;
pub use chat_req_response_format::*;
pub use chat_request::*;
pub use chat_response::*;
pub use chat_stream::*;
pub use content_part::*;
pub use message_content::*;
pub use tool::*;
pub use usage::*;

#[doc = "Printing helpers for chat requests and streaming output."]
pub mod printer;

// endregion: --- Modules

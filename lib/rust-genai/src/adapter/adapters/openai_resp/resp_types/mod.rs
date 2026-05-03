//! Private OpenAI Responses related types to use in the openai_resp chat adapter
//!
//!
//! ## Notes:
//!
//! This is just a subset implementation of the OpenAI Responses API to match to the chat API.
//!
//! At some point, genai, will have a full OpenAI Responses support, and those types
//! and the related parsing logic might move to the `src/resp/...` module (and we will have client.exec_responses(..))

// region:    --- Modules

mod resp_output_helper;
mod resp_response;
mod resp_usage;

pub use resp_response::*;
pub use resp_usage::*;

// endregion: --- Modules

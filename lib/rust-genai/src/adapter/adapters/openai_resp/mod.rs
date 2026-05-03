//! API Documentation:     <https://platform.openai.com/docs/api-reference/responses/create>
//! Model Names:           <https://platform.openai.com/docs/models>
//! Pricing:               <https://platform.openai.com/docs/pricing/> (user: <https://openai.com/api/pricing/>)

// region:    --- Modules

mod adapter_impl;
mod resp_types;
mod streamer;

pub use adapter_impl::*;
pub use streamer::*;

// endregion: --- Modules

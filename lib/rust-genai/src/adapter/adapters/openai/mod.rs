//! API Documentation:     <https://platform.openai.com/docs/api-reference/chat>
//! Model Names:           <https://platform.openai.com/docs/models>
//! Pricing:               <https://platform.openai.com/docs/pricing/> (user: <https://openai.com/api/pricing/>)

// region:    --- Modules

mod adapter_impl;
mod adapter_shared;
mod embed;
mod streamer;

pub use adapter_impl::*;
pub use adapter_shared::*;
pub use streamer::*;

// endregion: --- Modules

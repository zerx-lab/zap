//! `genai` library - A client library for any AI provider.
//! See [examples/c00-readme.rs](./examples/c00-readme.rs)

// region:    --- Modules

mod support;

mod client;
mod common;
mod error;

// -- Flatten
pub use client::*;
pub use common::*;
pub use error::{BoxError, Error, Result};

// -- Public Modules
pub mod adapter;
pub mod chat;
pub mod embed;
pub mod resolver;
pub mod webc;

// endregion: --- Modules

//! This support module is for common constructs and utilities for all the adapter implementations.
//! It should be private to the `crate::adapter::adapters` module.

use crate::ModelIden;
use crate::chat::{ChatOptionsSet, Usage};
use crate::resolver::AuthData;
use crate::{Error, Result};

pub fn get_api_key(auth: AuthData, model: &ModelIden) -> Result<String> {
	auth.single_key_value().map_err(|resolver_error| Error::Resolver {
		model_iden: model.clone(),
		resolver_error,
	})
}

// region:    --- StreamerChatOptions

#[derive(Debug)]
pub struct StreamerOptions {
	pub capture_usage: bool,
	pub capture_reasoning_content: bool,
	pub capture_content: bool,
	pub capture_tool_calls: bool,
	pub model_iden: ModelIden,
}

impl StreamerOptions {
	pub fn new(model_iden: ModelIden, options_set: ChatOptionsSet<'_, '_>) -> Self {
		Self {
			capture_usage: options_set.capture_usage().unwrap_or(false),
			capture_content: options_set.capture_content().unwrap_or(false),
			capture_reasoning_content: options_set.capture_reasoning_content().unwrap_or(false),
			capture_tool_calls: options_set.capture_tool_calls().unwrap_or(false),
			model_iden,
		}
	}
}

// endregion: --- StreamerChatOptions

// region:    --- Streamer Captured Data

#[derive(Debug, Default)]
pub struct StreamerCapturedData {
	pub usage: Option<Usage>,
	pub stop_reason: Option<String>,
	pub content: Option<String>,
	pub reasoning_content: Option<String>,
	pub tool_calls: Option<Vec<crate::chat::ToolCall>>,
	pub thought_signatures: Option<Vec<String>>,
}

// endregion: --- Streamer Captured Data

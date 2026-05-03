//! This module contains all the types related to an Embed Request.

use serde::{Deserialize, Serialize};

// region:    --- EmbedRequest

/// The Embed request for performing embedding operations with `Client::embed` or `Client::embed_batch`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedRequest {
	/// The input text(s) to embed.
	pub input: EmbedInput,
}

/// Constructors
impl EmbedRequest {
	/// Create a new EmbedRequest with a single text input.
	pub fn new(input: impl Into<String>) -> Self {
		Self {
			input: EmbedInput::Single(input.into()),
		}
	}

	/// Create a new EmbedRequest with multiple text inputs for batch processing.
	pub fn new_batch(inputs: Vec<String>) -> Self {
		Self {
			input: EmbedInput::Batch(inputs),
		}
	}

	/// Create an EmbedRequest from a single string.
	pub fn from_text(text: impl Into<String>) -> Self {
		Self::new(text)
	}

	/// Create an EmbedRequest from multiple strings.
	pub fn from_texts(texts: Vec<String>) -> Self {
		Self::new_batch(texts)
	}
}

/// Getters
impl EmbedRequest {
	/// Get the input as a single string if it's a single input, None otherwise.
	pub fn single_input(&self) -> Option<&str> {
		match &self.input {
			EmbedInput::Single(text) => Some(text),
			EmbedInput::Batch(_) => None,
		}
	}

	/// Get the input as a vector of strings.
	/// For single input, returns a vector with one element.
	pub fn inputs(&self) -> Vec<&str> {
		match &self.input {
			EmbedInput::Single(text) => vec![text],
			EmbedInput::Batch(texts) => texts.iter().map(|s| s.as_str()).collect(),
		}
	}

	/// Check if this is a batch request.
	pub fn is_batch(&self) -> bool {
		matches!(self.input, EmbedInput::Batch(_))
	}

	/// Get the number of inputs.
	pub fn input_count(&self) -> usize {
		match &self.input {
			EmbedInput::Single(_) => 1,
			EmbedInput::Batch(texts) => texts.len(),
		}
	}
}

// endregion: --- EmbedRequest

// region:    --- EmbedInput

/// The input for an embedding request, supporting both single and batch operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmbedInput {
	/// A single text input.
	Single(String),
	/// Multiple text inputs for batch processing.
	Batch(Vec<String>),
}

impl From<String> for EmbedInput {
	fn from(text: String) -> Self {
		EmbedInput::Single(text)
	}
}

impl From<&str> for EmbedInput {
	fn from(text: &str) -> Self {
		EmbedInput::Single(text.to_string())
	}
}

impl From<Vec<String>> for EmbedInput {
	fn from(texts: Vec<String>) -> Self {
		EmbedInput::Batch(texts)
	}
}

impl From<Vec<&str>> for EmbedInput {
	fn from(texts: Vec<&str>) -> Self {
		EmbedInput::Batch(texts.into_iter().map(|s| s.to_string()).collect())
	}
}

// endregion: --- EmbedInput

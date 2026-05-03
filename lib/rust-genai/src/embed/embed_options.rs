//! EmbedOptions allows customization of an embed request.
//! - It can be provided at the `client::embed(..)` level as an argument,
//! - or set in the client config `client_config.with_embed_options(..)` to be used as the default for all requests

use crate::Headers;
use serde::{Deserialize, Serialize};

// region:    --- EmbedOptions

/// Options for customizing embedding requests.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EmbedOptions {
	/// Custom headers to include in the request.
	pub headers: Option<Headers>,

	/// Whether to capture the raw response body for provider-specific features.
	pub capture_raw_body: Option<bool>,

	/// Whether to capture usage information (token counts, etc.).
	pub capture_usage: Option<bool>,

	/// The desired dimensionality of the embedding vectors (if supported by the provider).
	/// Note: Not all providers support custom dimensions.
	pub dimensions: Option<usize>,

	/// The encoding format for the embeddings (if supported by the provider).
	/// Common values: "float", "base64", "binary", etc.
	pub encoding_format: Option<String>,

	/// A unique identifier representing your end-user (for OpenAI and similar providers).
	pub user: Option<String>,

	/// The type/purpose of the embedding request.
	/// - Cohere: "search_document", "search_query", "classification", "clustering"
	/// - Gemini: "SEMANTIC_SIMILARITY", "RETRIEVAL_QUERY", "RETRIEVAL_DOCUMENT", "CLASSIFICATION"
	///
	/// Default: "search_document" (Cohere), "SEMANTIC_SIMILARITY" (Gemini)
	pub embedding_type: Option<String>,

	/// How to handle inputs longer than the maximum token length (supported by Cohere).
	/// Common values: "NONE", "START", "END"
	/// Default: "END"
	pub truncate: Option<String>,
}

/// Constructors
impl EmbedOptions {
	/// Create a new EmbedOptions with default values.
	pub fn new() -> Self {
		Self::default()
	}
}

/// Chainable Setters
impl EmbedOptions {
	/// Set custom headers for the request.
	pub fn with_headers(mut self, headers: Headers) -> Self {
		self.headers = Some(headers);
		self
	}

	/// Enable or disable capturing the raw response body.
	pub fn with_capture_raw_body(mut self, capture: bool) -> Self {
		self.capture_raw_body = Some(capture);
		self
	}

	/// Enable or disable capturing usage information.
	pub fn with_capture_usage(mut self, capture: bool) -> Self {
		self.capture_usage = Some(capture);
		self
	}

	/// Set the desired dimensionality of the embedding vectors.
	pub fn with_dimensions(mut self, dimensions: usize) -> Self {
		self.dimensions = Some(dimensions);
		self
	}

	/// Set the encoding format for the embeddings.
	pub fn with_encoding_format(mut self, format: impl Into<String>) -> Self {
		self.encoding_format = Some(format.into());
		self
	}

	/// Set the user identifier.
	pub fn with_user(mut self, user: impl Into<String>) -> Self {
		self.user = Some(user.into());
		self
	}

	/// Set the type/purpose of the embedding request.
	pub fn with_embedding_type(mut self, embedding_type: impl Into<String>) -> Self {
		self.embedding_type = Some(embedding_type.into());
		self
	}

	/// Set the truncation method for inputs longer than the maximum token length.
	pub fn with_truncate(mut self, truncate: impl Into<String>) -> Self {
		self.truncate = Some(truncate.into());
		self
	}
}

/// Getters
impl EmbedOptions {
	/// Get the headers.
	pub fn headers(&self) -> Option<&Headers> {
		self.headers.as_ref()
	}

	/// Get whether to capture raw body.
	pub fn capture_raw_body(&self) -> bool {
		self.capture_raw_body.unwrap_or(false)
	}

	/// Get whether to capture usage.
	pub fn capture_usage(&self) -> bool {
		self.capture_usage.unwrap_or(true)
	}

	/// Get the desired dimensions.
	pub fn dimensions(&self) -> Option<usize> {
		self.dimensions
	}

	/// Get the encoding format.
	pub fn encoding_format(&self) -> Option<&str> {
		self.encoding_format.as_deref()
	}

	/// Get the user identifier.
	pub fn user(&self) -> Option<&str> {
		self.user.as_deref()
	}

	/// Get the type/purpose of the embedding request.
	pub fn embedding_type(&self) -> Option<&str> {
		self.embedding_type.as_deref()
	}

	/// Get the truncation method.
	pub fn truncate(&self) -> Option<&str> {
		self.truncate.as_deref()
	}
}

// endregion: --- EmbedOptions

// region:    --- EmbedOptionsSet

/// A set of EmbedOptions that can be layered (client-level defaults + request-level overrides).
#[derive(Debug, Clone, Default)]
pub struct EmbedOptionsSet<'client, 'request> {
	client_options: Option<&'client EmbedOptions>,
	request_options: Option<&'request EmbedOptions>,
}

impl<'client, 'request> EmbedOptionsSet<'client, 'request> {
	/// Create a new EmbedOptionsSet.
	pub fn new() -> Self {
		Self::default()
	}

	/// Set the client-level options.
	pub fn with_client_options(mut self, options: Option<&'client EmbedOptions>) -> Self {
		self.client_options = options;
		self
	}

	/// Set the request-level options.
	pub fn with_request_options(mut self, options: Option<&'request EmbedOptions>) -> Self {
		self.request_options = options;
		self
	}

	/// Get the effective headers (request overrides client).
	pub fn headers(&self) -> Option<&Headers> {
		self.request_options
			.and_then(|o| o.headers())
			.or_else(|| self.client_options.and_then(|o| o.headers()))
	}

	/// Get the effective capture_raw_body setting.
	pub fn capture_raw_body(&self) -> bool {
		self.request_options
			.map(|o| o.capture_raw_body())
			.or_else(|| self.client_options.map(|o| o.capture_raw_body()))
			.unwrap_or(false)
	}

	/// Get the effective capture_usage setting.
	pub fn capture_usage(&self) -> bool {
		self.request_options
			.map(|o| o.capture_usage())
			.or_else(|| self.client_options.map(|o| o.capture_usage()))
			.unwrap_or(true)
	}

	/// Get the effective dimensions setting.
	pub fn dimensions(&self) -> Option<usize> {
		self.request_options
			.and_then(|o| o.dimensions())
			.or_else(|| self.client_options.and_then(|o| o.dimensions()))
	}

	/// Get the effective encoding_format setting.
	pub fn encoding_format(&self) -> Option<&str> {
		self.request_options
			.and_then(|o| o.encoding_format())
			.or_else(|| self.client_options.and_then(|o| o.encoding_format()))
	}

	/// Get the effective user setting.
	pub fn user(&self) -> Option<&str> {
		self.request_options
			.and_then(|o| o.user())
			.or_else(|| self.client_options.and_then(|o| o.user()))
	}

	/// Get the effective embedding type setting.
	pub fn embedding_type(&self) -> Option<&str> {
		self.request_options
			.and_then(|o| o.embedding_type())
			.or_else(|| self.client_options.and_then(|o| o.embedding_type()))
	}

	/// Get the effective truncate setting.
	pub fn truncate(&self) -> Option<&str> {
		self.request_options
			.and_then(|o| o.truncate())
			.or_else(|| self.client_options.and_then(|o| o.truncate()))
	}
}

// endregion: --- EmbedOptionsSet

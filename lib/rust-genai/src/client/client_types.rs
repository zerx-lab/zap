use crate::webc::WebClient;
use crate::{ClientBuilder, ClientConfig};
use std::sync::Arc;

/// Client for sending AI requests to supported providers.
///
/// Construct with:
/// - [`ClientBuilder::default()`] followed by `.build()`, or
///
/// - [`Client::builder()`], which is equivalent to `ClientBuilder::default()`.
#[derive(Debug, Clone)]
pub struct Client {
	pub(super) inner: Arc<ClientInner>,
}

// region:    --- Client Constructors

impl Default for Client {
	/// Creates a [`Client`] with default configuration.
	///
	/// Equivalent to `Client::builder().build()`.
	fn default() -> Self {
		Client::builder().build()
	}
}

impl Client {
	/// Returns a builder for configuring and constructing a [`Client`].
	///
	/// Equivalent to calling [`ClientBuilder::default()`].
	pub fn builder() -> ClientBuilder {
		ClientBuilder::default()
	}
}

// endregion: --- Client Constructors

// region:    --- Client Getters

impl Client {
	pub(crate) fn web_client(&self) -> &WebClient {
		&self.inner.web_client
	}

	pub(crate) fn config(&self) -> &ClientConfig {
		&self.inner.config
	}
}

// endregion: --- Client Getters

// region:    --- ClientInner

#[derive(Debug)]
pub(super) struct ClientInner {
	pub(super) web_client: WebClient,

	pub(super) config: ClientConfig,
}

// endregion: --- ClientInner

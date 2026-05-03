use serde::{Deserialize, Serialize};

/// Configuration for the built-in web search tool.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct WebSearchConfig {
	/// Maximum number of web fetches allowed per request.
	/// If not set, the model decides how many fetches to perform.
	pub max_uses: Option<u32>,

	/// Only URLs from these domains will be fetched.
	/// Cannot be used together with `blocked_domains`.
	pub allowed_domains: Option<Vec<String>>,

	/// URLs from these domains will not be fetched.
	/// Cannot be used together with `allowed_domains`.
	pub blocked_domains: Option<Vec<String>>,
}

impl WebSearchConfig {
	/// Set maximum number of web fetches allowed per request.
	/// Takes the actual value (not an Option) and returns the updated config.
	pub fn with_max_uses(mut self, max_uses: u32) -> Self {
		self.max_uses = Some(max_uses);
		self
	}

	/// Set the allowed domains list. Takes an iterator of items convertible into String.
	pub fn with_allowed_domains<I, S>(mut self, allowed_domains: I) -> Self
	where
		I: IntoIterator<Item = S>,
		S: Into<String>,
	{
		self.allowed_domains = Some(allowed_domains.into_iter().map(Into::into).collect());
		self
	}

	/// Set the blocked domains list. Takes an iterator of items convertible into String.
	pub fn with_blocked_domains<I, S>(mut self, blocked_domains: I) -> Self
	where
		I: IntoIterator<Item = S>,
		S: Into<String>,
	{
		self.blocked_domains = Some(blocked_domains.into_iter().map(Into::into).collect());
		self
	}
}

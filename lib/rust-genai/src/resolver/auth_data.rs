use crate::Headers;
use crate::resolver::{Error, Result};
use std::collections::HashMap;

/// `AuthData` specifies either how or the key itself for an authentication resolver call.
#[derive(Clone)]
pub enum AuthData {
	/// Specify the environment name to get the key value from.
	FromEnv(String),

	/// The key value itself.
	Key(String),

	/// Override headers and request url for unorthodox authentication schemes
	RequestOverride {
		url: String,
		headers: Headers,
	},

	/// The key names/values when a credential has multiple pieces of credential information.
	/// This will be adapter-specific.
	/// NOTE: Not used yet.
	MultiKeys(HashMap<String, String>),

	None,
}

/// Constructors
impl AuthData {
	/// Create a new `AuthData` from an environment variable name.
	pub fn from_env(env_name: impl Into<String>) -> Self {
		AuthData::FromEnv(env_name.into())
	}

	/// Create a new `AuthData` from a single value.
	pub fn from_single(value: impl Into<String>) -> Self {
		AuthData::Key(value.into())
	}

	/// Create a new `AuthData` from multiple values.
	pub fn from_multi(data: HashMap<String, String>) -> Self {
		AuthData::MultiKeys(data)
	}
}

/// Getters
impl AuthData {
	/// Get the single value from the `AuthData`.
	pub fn single_key_value(&self) -> Result<String> {
		match self {
			// Overrides don't use an api key
			AuthData::RequestOverride { .. } => Ok(String::new()),
			AuthData::FromEnv(env_name) => {
				// Get value from the environment name.
				let value = std::env::var(env_name).map_err(|_| Error::ApiKeyEnvNotFound {
					env_name: env_name.to_string(),
				})?;
				Ok(value)
			}
			AuthData::Key(value) => Ok(value.to_string()),
			_ => Err(Error::ResolverAuthDataNotSingleValue),
		}
	}
}

// region:    --- AuthData Std Impls

// Implement Debug to redact sensitive information.
impl std::fmt::Debug for AuthData {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			// NOTE: Here we also redact for `FromEnv` in case the developer confuses this with a key.
			AuthData::FromEnv(_env_name) => write!(f, "AuthData::FromEnv(REDACTED)"),
			AuthData::Key(_) => write!(f, "AuthData::Single(REDACTED)"),
			AuthData::MultiKeys(_) => write!(f, "AuthData::Multi(REDACTED)"),
			AuthData::RequestOverride { .. } => {
				write!(f, "AuthData::RequestOverride {{ url: REDACTED, headers: REDACTED }}")
			}
			AuthData::None => write!(f, "None"),
		}
	}
}

// endregion: --- AuthData Std Impls

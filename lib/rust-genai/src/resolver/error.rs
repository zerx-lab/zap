use derive_more::From;

/// Resolver Result type alias.
pub type Result<T> = core::result::Result<T, Error>;

/// Resolver error type.
#[derive(Debug, From)]
pub enum Error {
	/// The API key environment variable was not found.
	ApiKeyEnvNotFound {
		/// The name of the environment variable.
		env_name: String,
	},

	/// The `AuthData` is not a single value.
	ResolverAuthDataNotSingleValue,

	/// Custom error message.
	#[from]
	Custom(String),
}

// region:    --- Error Boilerplate

impl core::fmt::Display for Error {
	fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::result::Result<(), core::fmt::Error> {
		write!(fmt, "{self:?}")
	}
}

impl std::error::Error for Error {}

// endregion: --- Error Boilerplate

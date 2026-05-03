use crate::{ModelIden, ModelName, ServiceTarget};

/// Specifies how to identify and resolve a model for API calls.
///
/// `ModelSpec` provides three levels of control over model resolution:
///
/// - [`ModelSpec::Name`]: Just a model name string. The adapter kind is inferred
///   from the name, and auth/endpoint are resolved via the client's configured resolvers.
///
/// - [`ModelSpec::Iden`]: An explicit [`ModelIden`] with adapter kind specified.
///   Skips adapter inference but still resolves auth/endpoint via config.
///
/// - [`ModelSpec::Target`]: A complete [`ServiceTarget`] with endpoint, auth, and model.
///   Used directly, only runs the service target resolver.
///
/// # Examples
///
/// ```rust
/// use genai::adapter::AdapterKind;
/// use genai::resolver::{AuthData, Endpoint};
/// use genai::{ModelIden, ModelSpec, ServiceTarget};
///
/// // Using a string name (full inference)
/// let spec: ModelSpec = "gpt-4".into();
///
/// // Using an explicit ModelIden (skip adapter inference)
/// let spec: ModelSpec = ModelIden::new(AdapterKind::OpenAI, "gpt-4").into();
///
/// // Using a complete ServiceTarget (bypass all resolution)
/// let target = ServiceTarget {
///     endpoint: Endpoint::from_static("https://custom.api/v1/"),
///     auth: AuthData::from_env("CUSTOM_API_KEY"),
///     model: ModelIden::new(AdapterKind::OpenAI, "custom-model"),
/// };
/// let spec: ModelSpec = target.into();
/// ```
#[derive(Debug, Clone)]
pub enum ModelSpec {
	/// Model name - without or without model namespace
	Name(ModelName),

	/// Explicit [`ModelIden`] - skips adapter inference, still resolves auth/endpoint.
	Iden(ModelIden),

	/// Complete [`ServiceTarget`] - used directly, bypasses model mapping and auth resolution
	Target(ServiceTarget),
}

// region:    --- Constructors

impl ModelSpec {
	/// Creates a `ModelSpec::Name` from a string.
	pub fn from_name(name: impl Into<ModelName>) -> Self {
		ModelSpec::Name(name.into())
	}

	/// Creates a `ModelSpec::Name` from a static str.
	pub fn from_static_name(name: &'static str) -> Self {
		let name = ModelName::from_static(name);
		ModelSpec::Name(name)
	}

	/// Creates a `ModelSpec::Iden` from a ModelIden
	pub fn from_iden(model_iden: impl Into<ModelIden>) -> Self {
		let model_iden = model_iden.into();
		Self::Iden(model_iden)
	}

	/// Creates a `ModelSpec::Target` from a complete service target.
	pub fn from_target(target: ServiceTarget) -> Self {
		ModelSpec::Target(target)
	}
}

// endregion: --- Constructors

// region:    --- From Implementations

impl From<&str> for ModelSpec {
	fn from(name: &str) -> Self {
		ModelSpec::Name(name.into())
	}
}

impl From<&&str> for ModelSpec {
	fn from(name: &&str) -> Self {
		ModelSpec::Name((*name).into())
	}
}

impl From<String> for ModelSpec {
	fn from(name: String) -> Self {
		ModelSpec::Name(name.into())
	}
}

impl From<&String> for ModelSpec {
	fn from(name: &String) -> Self {
		ModelSpec::Name(name.into())
	}
}

impl From<ModelName> for ModelSpec {
	fn from(model: ModelName) -> Self {
		ModelSpec::Name(model)
	}
}

impl From<&ModelName> for ModelSpec {
	fn from(model: &ModelName) -> Self {
		ModelSpec::Name(model.clone())
	}
}

impl From<ModelIden> for ModelSpec {
	fn from(model: ModelIden) -> Self {
		ModelSpec::Iden(model)
	}
}

impl From<&ModelIden> for ModelSpec {
	fn from(model: &ModelIden) -> Self {
		ModelSpec::Iden(model.clone())
	}
}

impl From<ServiceTarget> for ModelSpec {
	fn from(target: ServiceTarget) -> Self {
		ModelSpec::Target(target)
	}
}

// endregion: --- From Implementations

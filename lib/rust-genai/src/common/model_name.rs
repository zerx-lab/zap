use std::ops::Deref;
use std::sync::Arc;

use serde::{Deserialize, Deserializer, Serialize};

/// Store a model name with or without namespace
/// e.g. `gemini-3-flash-preview` or `gemini::gemini-3-flash-preview`
#[derive(Clone, Debug, Serialize, Hash, Eq, PartialEq)]
pub struct ModelName(Inner);

#[derive(Clone, Debug, Serialize, Hash, Eq, PartialEq)]
enum Inner {
	Static(&'static str),
	Shared(Arc<str>),
}

impl<'de> Deserialize<'de> for ModelName {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		let s: &str = <&str>::deserialize(deserializer)?;
		Ok(ModelName(Inner::Shared(Arc::<str>::from(s))))
	}
}

/// Constructor
impl ModelName {
	pub fn new(name: impl Into<Arc<str>>) -> Self {
		Self(Inner::Shared(name.into()))
	}

	pub fn from_static(name: &'static str) -> Self {
		Self(Inner::Static(name))
	}
}

impl ModelName {
	pub fn as_str(&self) -> &str {
		match self.0 {
			Inner::Static(s) => s,
			Inner::Shared(ref s) => s,
		}
	}

	pub fn namespace_is(&self, namespace: &str) -> bool {
		self.namespace() == Some(namespace)
	}

	pub fn namespace(&self) -> Option<&str> {
		self.namespace_and_name().0
	}

	/// Returns `(namespace, name)`
	pub fn namespace_and_name(&self) -> (Option<&str>, &str) {
		Self::split_as_namespace_and_name(self.as_str())
	}

	/// e.g.:
	/// `openai::gpt4.1` → (Some("openai"), "gpt4.1")
	/// `gpt4.1`         → (None, "gpt4.1")
	pub(crate) fn split_as_namespace_and_name(model: &str) -> (Option<&str>, &str) {
		if let Some(ns_idx) = model.find("::") {
			let ns: &str = &model[..ns_idx];
			let name: &str = &model[(ns_idx + 2)..];
			(Some(ns), name)
		} else {
			(None, model)
		}
	}
}

impl std::fmt::Display for ModelName {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.as_str())
	}
}

// region:    --- Froms

impl From<ModelName> for String {
	fn from(model_name: ModelName) -> Self {
		model_name.as_str().to_string()
	}
}

// NOTE: Below we avoid the `T: Into<String>` blanket implementation because
//       it would prevent us from having the `From<ModelName> for String` implementation since `ModelName`
//       also implements `T: Into<String>` from its deref to `&str`

impl From<String> for ModelName {
	fn from(s: String) -> Self {
		Self(Inner::Shared(Arc::from(s)))
	}
}

impl From<&String> for ModelName {
	fn from(s: &String) -> Self {
		Self(Inner::Shared(Arc::from(s.as_str())))
	}
}

impl From<&str> for ModelName {
	fn from(s: &str) -> Self {
		Self(Inner::Shared(Arc::from(s)))
	}
}

/// Deref as `&str`
impl Deref for ModelName {
	type Target = str;

	fn deref(&self) -> &Self::Target {
		self.as_str()
	}
}

// endregion: --- Froms

// region:    --- EQ

// PartialEq implementations for various string types
impl PartialEq<str> for ModelName {
	fn eq(&self, other: &str) -> bool {
		self.as_str() == other
	}
}

impl PartialEq<&str> for ModelName {
	fn eq(&self, other: &&str) -> bool {
		self.as_str() == *other
	}
}

impl PartialEq<String> for ModelName {
	fn eq(&self, other: &String) -> bool {
		self.as_str() == other
	}
}

// Symmetric implementations (allow "string" == model_name)
impl PartialEq<ModelName> for str {
	fn eq(&self, other: &ModelName) -> bool {
		self == other.as_str()
	}
}

impl PartialEq<ModelName> for &str {
	fn eq(&self, other: &ModelName) -> bool {
		*self == other.as_str()
	}
}

impl PartialEq<ModelName> for String {
	fn eq(&self, other: &ModelName) -> bool {
		self == other.as_str()
	}
}

// endregion: --- EQ

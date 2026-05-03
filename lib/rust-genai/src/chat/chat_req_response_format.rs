use derive_more::From;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use value_ext::JsonValueExt;

/// Preferred response format for `ChatRequest` (structured output).
/// Only applied when the provider supports it.
///
/// > Note: Most providers currently ignore unsupported formats without error; this may change later.
#[derive(Debug, Clone, From, Serialize, Deserialize)]
pub enum ChatResponseFormat {
	/// Request JSON-mode output (primarily for OpenAI).
	/// Note: Include an instruction like "Reply in JSON format." in your system/prompt to improve reliability.
	JsonMode,

	/// Request structured output.
	#[from]
	JsonSpec(JsonSpec),
}

/// JSON specification used to enforce structured output.
#[derive(Debug, Clone, From, Serialize, Deserialize)]
pub struct JsonSpec {
	/// Specification name (primarily used by OpenAI).
	/// IMPORTANT: For OpenAI, only `-` and `_` are allowed; no spaces or other special characters.
	pub name: String,

	/// Human-readable description (used by some adapters in the future).
	/// NOTE: Currently ignored by the OpenAI adapter.
	pub description: Option<String>,

	/// Simplified JSON schema forwarded to the provider.
	pub schema: Value,
}

/// Constructors
impl JsonSpec {
	/// Create a `JsonSpec` with a name and schema.
	pub fn new(name: impl Into<String>, schema: impl Into<Value>) -> Self {
		Self {
			name: name.into(),
			description: None,
			schema: schema.into(),
		}
	}
}

/// Setters
impl JsonSpec {
	/// Set an optional description (builder-style).
	pub fn with_description(mut self, description: impl Into<String>) -> Self {
		self.description = Some(description.into());
		self
	}
}

/// Helpers
impl JsonSpec {
	/// Returns a clone of the schema with `"additionalProperties": false` injected into every
	/// object node. Required by several providers (Anthropic, OpenAI) whose structured-output
	/// APIs reject schemas that omit this constraint.
	pub fn schema_with_additional_properties_false(&self) -> Value {
		let mut schema = self.schema.clone();
		schema.x_walk(|parent_map, name| {
			if name == "type" {
				let typ = parent_map.get("type").and_then(|v| v.as_str()).unwrap_or("");
				if typ == "object" {
					parent_map.insert("additionalProperties".to_string(), false.into());
				}
			}
			true
		});
		schema
	}
}

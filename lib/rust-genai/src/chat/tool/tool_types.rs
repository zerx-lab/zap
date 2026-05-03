use derive_more::{Display, From};
use serde::de::{self, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

use crate::chat::WebSearchConfig;

/// Normalized tool identifiers.
///
/// Custom (user-defined) tools are the common case and serialize as bare strings:
///   `ToolName::Custom("get_weather")` → `"get_weather"`
///
/// Built-in tools require explicit qualification:
///   `ToolName::WebSearch` → `{"WebSearch": null}`
#[derive(Debug, Clone, PartialEq, Eq, Display, From)]
pub enum ToolName {
	/// Built-in web search tool (e.g., Google Search for Gemini).
	#[display("WebSearch")]
	WebSearch,

	/// User-defined custom tool.
	#[from(String, &String, &str)]
	#[display("{_0}")]
	Custom(String),
}

impl ToolName {
	pub fn as_str(&self) -> &str {
		match self {
			Self::WebSearch => "WebSearch",
			Self::Custom(name) => name.as_str(),
		}
	}
}

impl AsRef<str> for ToolName {
	fn as_ref(&self) -> &str {
		self.as_str()
	}
}

// region:    --- Intos

impl From<ToolName> for String {
	fn from(name: ToolName) -> Self {
		match name {
			ToolName::Custom(n) => n,
			other => other.as_str().to_string(),
		}
	}
}

impl From<&ToolName> for String {
	fn from(name: &ToolName) -> Self {
		name.as_str().to_string()
	}
}
// endregion: --- Intos

// region:    --- JSON Serialize / Deserialize

impl Serialize for ToolName {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		match self {
			// Custom tools are the default — just a bare string.
			Self::Custom(name) => serializer.serialize_str(name),
			// Built-in tools are qualified as {"VariantName": null}.
			Self::WebSearch => {
				let mut map = serializer.serialize_map(Some(1))?;
				map.serialize_entry("WebSearch", &())?;
				map.end()
			}
		}
	}
}

impl<'de> Deserialize<'de> for ToolName {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		struct ToolNameVisitor;

		impl<'de> Visitor<'de> for ToolNameVisitor {
			type Value = ToolName;

			fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
				f.write_str("a string (custom tool) or {\"WebSearch\": null} (built-in)")
			}

			// Bare string → Custom tool (the common case).
			fn visit_str<E: de::Error>(self, value: &str) -> Result<ToolName, E> {
				Ok(ToolName::Custom(value.to_string()))
			}

			// Object → built-in tool, keyed by variant name.
			fn visit_map<A: de::MapAccess<'de>>(self, mut map: A) -> Result<ToolName, A::Error> {
				let key: String = map.next_key()?.ok_or_else(|| de::Error::custom("empty object"))?;
				match key.as_str() {
					"WebSearch" => {
						// Consume the null value.
						let _: serde::de::IgnoredAny = map.next_value()?;
						Ok(ToolName::WebSearch)
					}
					other => Err(de::Error::unknown_variant(other, &["WebSearch"])),
				}
			}
		}

		deserializer.deserialize_any(ToolNameVisitor)
	}
}

// endregion: --- JSON Serialize / Deserialize

/// Configuration variant for tools.
///
/// Custom configurations are the common case and serialize as the raw value:
///   `ToolConfig::Custom(json)` → the json value directly
///
/// Built-in tool configs require explicit qualification:
///   `ToolConfig::WebSearch(config)` → `{"WebSearch": {...}}`
#[derive(Debug, Clone, From, PartialEq, Eq)]
pub enum ToolConfig {
	/// Configuration for web search.
	#[from]
	WebSearch(WebSearchConfig),

	/// Arbitrary JSON configuration for custom tools.
	#[from]
	Custom(serde_json::Value),
}

// region:    --- JSON Serialize / Deserialize

impl Serialize for ToolConfig {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		match self {
			// Custom configs serialize as the raw value (the common case).
			Self::Custom(conf) => conf.serialize(serializer),
			// Built-in configs are qualified as {"VariantName": config}.
			Self::WebSearch(conf) => {
				let mut map = serializer.serialize_map(Some(1))?;
				map.serialize_entry("WebSearch", conf)?;
				map.end()
			}
		}
	}
}

impl<'de> Deserialize<'de> for ToolConfig {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		// Deserialize into a generic Value, then inspect.
		let value = serde_json::Value::deserialize(deserializer)?;

		// If it's an object with exactly one key "WebSearch", treat as built-in.
		if let serde_json::Value::Object(ref map) = value
			&& map.len() == 1
			&& let Some(inner) = map.get("WebSearch")
		{
			let config: WebSearchConfig = serde_json::from_value(inner.clone()).map_err(de::Error::custom)?;
			return Ok(ToolConfig::WebSearch(config));
		}

		// Everything else is a custom config.
		Ok(ToolConfig::Custom(value))
	}
}

// endregion: --- JSON Serialize / Deserialize

#[cfg(test)]
mod tests {
	use super::*;

	// -- ToolName tests --

	#[test]
	fn tool_name_custom_roundtrip() {
		let original = ToolName::Custom("get_weather".into());
		let json = serde_json::to_string(&original).unwrap();
		// Custom tools are bare strings (the default, unqualified format).
		assert_eq!(json, r#""get_weather""#);
		let round: ToolName = serde_json::from_str(&json).unwrap();
		assert_eq!(round, original);
	}

	#[test]
	fn tool_name_websearch_roundtrip() {
		let original = ToolName::WebSearch;
		let json = serde_json::to_string(&original).unwrap();
		// Built-in tools are qualified as objects.
		assert_eq!(json, r#"{"WebSearch":null}"#);
		let round: ToolName = serde_json::from_str(&json).unwrap();
		assert_eq!(round, original);
	}

	/// A custom tool named "WebSearch" is NOT confused with the built-in.
	#[test]
	fn tool_name_custom_named_websearch_is_disambiguated() {
		let custom = ToolName::Custom("WebSearch".into());
		let builtin = ToolName::WebSearch;

		let custom_json = serde_json::to_string(&custom).unwrap();
		let builtin_json = serde_json::to_string(&builtin).unwrap();

		// They serialize differently.
		assert_ne!(custom_json, builtin_json);
		assert_eq!(custom_json, r#""WebSearch""#); // bare string
		assert_eq!(builtin_json, r#"{"WebSearch":null}"#); // qualified object

		// And round-trip to the correct variants.
		let custom_round: ToolName = serde_json::from_str(&custom_json).unwrap();
		let builtin_round: ToolName = serde_json::from_str(&builtin_json).unwrap();
		assert_eq!(custom_round, custom);
		assert_eq!(builtin_round, builtin);
	}

	// -- ToolConfig tests --

	#[test]
	fn tool_config_custom_roundtrip() {
		let original = ToolConfig::Custom(serde_json::json!({"key": "value"}));
		let json = serde_json::to_string(&original).unwrap();
		// Custom configs are raw values (the default, unqualified format).
		assert_eq!(json, r#"{"key":"value"}"#);
		let round: ToolConfig = serde_json::from_str(&json).unwrap();
		assert_eq!(round, original);
	}

	#[test]
	fn tool_config_websearch_roundtrip() {
		let original = ToolConfig::WebSearch(WebSearchConfig {
			max_uses: Some(5),
			allowed_domains: None,
			blocked_domains: None,
		});
		let json = serde_json::to_string(&original).unwrap();
		// Built-in configs are qualified as {"WebSearch": config}.
		assert!(json.contains(r#""WebSearch""#));
		let round: ToolConfig = serde_json::from_str(&json).unwrap();
		assert_eq!(round, original);
	}

	/// An array config (e.g. functionDeclarations) round-trips as Custom.
	#[test]
	fn tool_config_array_roundtrip() {
		let original = ToolConfig::Custom(serde_json::json!([
			{"name": "analyze_file", "description": "Analyze a file"}
		]));
		let json = serde_json::to_string(&original).unwrap();
		let round: ToolConfig = serde_json::from_str(&json).unwrap();
		assert_eq!(round, original);
	}
}

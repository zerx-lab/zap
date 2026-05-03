//! Convert JSON Schema to OpenAPI 3.0.3 Schema Object subset.
//!
//! Gemini uses the OpenAPI 3.0.3 Schema Object subset and rejects standard
//! JSON Schema features such as `$ref`/`$defs`, composite keywords
//! (`allOf`/`anyOf`/`oneOf`), array-type nullable (`"type": ["string", "null"]`),
//! and `additionalProperties`.
//!
//! [`to_openapi_schema`] rewrites a JSON Schema in-place so that it conforms
//! to the OpenAPI-compatible subset while preserving structural semantics.

use serde_json::{Map, Value};
use std::collections::HashSet;

/// Convert a JSON Schema value in-place to the OpenAPI 3.0.3 Schema Object subset.
///
/// The following transformations are applied (order matters):
///
/// 1. Resolve all `$ref` pointers by inlining from `$defs`/`definitions`.
/// 2. Remove the now-redundant `$defs`/`definitions` dictionaries.
/// 3. Flatten single-element `allOf`/`anyOf`/`oneOf` and nullable
///    `anyOf`/`oneOf` patterns (the schemars `Option<T>` idiom).
/// 4. Normalize `"type": ["T", "null"]` → `"type": "T"`.
/// 5. Remove `additionalProperties` everywhere.
/// 6. Recurse into `properties`, `items`, `prefixItems`, and remaining
///    composite entries.
pub(super) fn to_openapi_schema(schema: &mut Value) {
	if let Value::Object(map) = schema {
		// Step 1 + 2: resolve $ref and remove $defs
		let defs = extract_defs(map);
		if !defs.is_empty() {
			let mut visited = HashSet::new();
			resolve_refs(map, &defs, &mut visited);
		}

		// Steps 3–6
		simplify_object(map);
	}
}

// -- Private helpers

/// Extract and remove `$defs` / `definitions` from the top-level map.
fn extract_defs(map: &mut Map<String, Value>) -> Map<String, Value> {
	let mut defs = Map::new();
	for key in &["$defs", "definitions"] {
		if let Some(Value::Object(d)) = map.remove(*key) {
			defs.extend(d);
		}
	}
	defs
}

/// Recursively resolve `$ref` pointers by inlining from `defs`.
fn resolve_refs(map: &mut Map<String, Value>, defs: &Map<String, Value>, visited: &mut HashSet<String>) {
	// If this object is a $ref, inline the definition.
	if let Some(Value::String(ref_path)) = map.get("$ref") {
		// Expected form: "#/$defs/TypeName" or "#/definitions/TypeName"
		let name = ref_path.rsplit('/').next().unwrap_or("").to_string();
		if !name.is_empty()
			&& !visited.contains(&name)
			&& let Some(def) = defs.get(&name)
		{
			visited.insert(name.clone());
			let mut resolved = def.clone();
			// Recursively resolve refs within the inlined definition.
			if let Value::Object(ref mut inner) = resolved {
				resolve_refs(inner, defs, visited);
			}
			visited.remove(&name);

			// Replace this map's contents with the resolved definition.
			map.remove("$ref");
			if let Value::Object(inner) = resolved {
				for (k, v) in inner {
					map.entry(&k).or_insert(v);
				}
			}
			return;
		}
	}

	// Recurse into all nested objects and arrays.
	for value in map.values_mut() {
		match value {
			Value::Object(child) => resolve_refs(child, defs, visited),
			Value::Array(arr) => {
				for item in arr.iter_mut() {
					if let Value::Object(child) = item {
						resolve_refs(child, defs, visited);
					}
				}
			}
			_ => {}
		}
	}
}

/// Simplify an object map (steps 3–6).
fn simplify_object(map: &mut Map<String, Value>) {
	// Step 3: flatten composites.
	flatten_composites(map);

	// Step 4: normalize nullable type arrays.
	normalize_nullable_type(map);

	// Step 5: remove additionalProperties.
	map.remove("additionalProperties");

	// Step 6: recurse into sub-schemas.
	recurse_into_children(map);
}

/// Flatten `allOf`, `anyOf`, `oneOf` composite keywords.
fn flatten_composites(map: &mut Map<String, Value>) {
	for keyword in &["allOf", "anyOf", "oneOf"] {
		let Some(Value::Array(variants)) = map.remove(*keyword) else {
			continue;
		};

		// Collect non-null variants.
		let non_null: Vec<Value> = variants.into_iter().filter(|v| !is_null_schema(v)).collect();

		if non_null.len() == 1 {
			// Single effective variant → merge into parent.
			if let Value::Object(inner) = non_null.into_iter().next().unwrap() {
				for (k, v) in inner {
					map.entry(&k).or_insert(v);
				}
			}
		} else if !non_null.is_empty() {
			// Multiple non-null variants — keep the composite but re-insert.
			map.insert(keyword.to_string(), Value::Array(non_null));
		}

		// If we successfully flattened, don't process additional composite keywords
		// on this same map (unlikely to have multiples, but be safe).
		if !map.contains_key(*keyword) {
			// We consumed it; check if any more remain.
			continue;
		}
	}
}

/// Returns `true` if `v` represents a null-only schema: `{"type": "null"}`.
fn is_null_schema(v: &Value) -> bool {
	match v {
		Value::Object(m) => m.get("type").and_then(Value::as_str) == Some("null"),
		_ => false,
	}
}

/// Normalize `"type": ["string", "null"]` → `"type": "string"`.
fn normalize_nullable_type(map: &mut Map<String, Value>) {
	if let Some(Value::Array(types)) = map.get("type") {
		let non_null: Vec<&Value> = types.iter().filter(|t| t.as_str() != Some("null")).collect();
		if non_null.len() == 1 {
			let single = non_null[0].clone();
			map.insert("type".to_string(), single);
		}
	}
}

/// Recurse into `properties`, `items`, `prefixItems`, and remaining composite entries.
fn recurse_into_children(map: &mut Map<String, Value>) {
	// properties
	if let Some(Value::Object(props)) = map.get_mut("properties") {
		for prop_schema in props.values_mut() {
			if let Value::Object(inner) = prop_schema {
				simplify_object(inner);
			}
		}
	}

	// items (single schema or array of schemas)
	if let Some(items) = map.get_mut("items") {
		match items {
			Value::Object(inner) => simplify_object(inner),
			Value::Array(arr) => {
				for item in arr.iter_mut() {
					if let Value::Object(inner) = item {
						simplify_object(inner);
					}
				}
			}
			_ => {}
		}
	}

	// prefixItems
	if let Some(Value::Array(arr)) = map.get_mut("prefixItems") {
		for item in arr.iter_mut() {
			if let Value::Object(inner) = item {
				simplify_object(inner);
			}
		}
	}

	// Remaining composite entries (if multi-variant composites survived flattening)
	for keyword in &["allOf", "anyOf", "oneOf"] {
		if let Some(Value::Array(arr)) = map.get_mut(*keyword) {
			for item in arr.iter_mut() {
				if let Value::Object(inner) = item {
					simplify_object(inner);
				}
			}
		}
	}
}

// region:    --- Tests

#[cfg(test)]
mod tests {
	use super::*;
	use serde_json::json;

	#[test]
	fn test_passthrough_clean_schema() {
		let mut schema = json!({
			"type": "object",
			"properties": {
				"name": { "type": "string" }
			},
			"required": ["name"]
		});
		let expected = json!({
			"type": "object",
			"properties": {
				"name": { "type": "string" }
			},
			"required": ["name"]
		});
		to_openapi_schema(&mut schema);
		assert_eq!(schema, expected);
	}

	#[test]
	fn test_removes_additional_properties() {
		let mut schema = json!({
			"type": "object",
			"properties": {
				"inner": {
					"type": "object",
					"additionalProperties": false,
					"properties": {
						"x": { "type": "integer", "additionalProperties": false }
					}
				}
			},
			"additionalProperties": false
		});
		to_openapi_schema(&mut schema);

		assert!(schema.get("additionalProperties").is_none());
		let inner = &schema["properties"]["inner"];
		assert!(inner.get("additionalProperties").is_none());
		let x = &inner["properties"]["x"];
		assert!(x.get("additionalProperties").is_none());
	}

	#[test]
	fn test_nullable_array_type() {
		let mut schema = json!({
			"type": "object",
			"properties": {
				"name": { "type": ["string", "null"] }
			}
		});
		to_openapi_schema(&mut schema);

		assert_eq!(schema["properties"]["name"]["type"], "string");
	}

	#[test]
	fn test_nullable_array_type_null_first() {
		let mut schema = json!({
			"type": "object",
			"properties": {
				"count": { "type": ["null", "integer"] }
			}
		});
		to_openapi_schema(&mut schema);

		assert_eq!(schema["properties"]["count"]["type"], "integer");
	}

	#[test]
	fn test_single_allof_flattening() {
		let mut schema = json!({
			"type": "object",
			"properties": {
				"config": {
					"allOf": [
						{ "type": "object", "properties": { "key": { "type": "string" } } }
					]
				}
			}
		});
		to_openapi_schema(&mut schema);

		assert_eq!(schema["properties"]["config"]["type"], "object");
		assert_eq!(schema["properties"]["config"]["properties"]["key"]["type"], "string");
		assert!(schema["properties"]["config"].get("allOf").is_none());
	}

	#[test]
	fn test_nullable_anyof_schemars_pattern() {
		// schemars generates this for Option<T>
		let mut schema = json!({
			"type": "object",
			"properties": {
				"label": {
					"anyOf": [
						{ "type": "string" },
						{ "type": "null" }
					]
				}
			}
		});
		to_openapi_schema(&mut schema);

		assert_eq!(schema["properties"]["label"]["type"], "string");
		assert!(schema["properties"]["label"].get("anyOf").is_none());
	}

	#[test]
	fn test_nullable_oneof() {
		let mut schema = json!({
			"type": "object",
			"properties": {
				"value": {
					"oneOf": [
						{ "type": "integer" },
						{ "type": "null" }
					]
				}
			}
		});
		to_openapi_schema(&mut schema);

		assert_eq!(schema["properties"]["value"]["type"], "integer");
		assert!(schema["properties"]["value"].get("oneOf").is_none());
	}

	#[test]
	fn test_ref_resolution_and_defs_removal() {
		let mut schema = json!({
			"type": "object",
			"properties": {
				"address": { "$ref": "#/$defs/Address" }
			},
			"$defs": {
				"Address": {
					"type": "object",
					"properties": {
						"street": { "type": "string" },
						"city": { "type": "string" }
					},
					"additionalProperties": false
				}
			}
		});
		to_openapi_schema(&mut schema);

		// $defs should be removed
		assert!(schema.get("$defs").is_none());
		// $ref should be resolved
		assert!(schema["properties"]["address"].get("$ref").is_none());
		assert_eq!(schema["properties"]["address"]["type"], "object");
		assert_eq!(
			schema["properties"]["address"]["properties"]["street"]["type"],
			"string"
		);
		// additionalProperties removed from resolved def
		assert!(schema["properties"]["address"].get("additionalProperties").is_none());
	}

	#[test]
	fn test_ref_with_definitions_key() {
		let mut schema = json!({
			"type": "object",
			"properties": {
				"item": { "$ref": "#/definitions/Item" }
			},
			"definitions": {
				"Item": {
					"type": "object",
					"properties": {
						"id": { "type": "integer" }
					}
				}
			}
		});
		to_openapi_schema(&mut schema);

		assert!(schema.get("definitions").is_none());
		assert_eq!(schema["properties"]["item"]["type"], "object");
	}

	#[test]
	fn test_cycle_detection() {
		// Self-referencing $ref should not infinite-loop.
		let mut schema = json!({
			"type": "object",
			"properties": {
				"node": { "$ref": "#/$defs/Node" }
			},
			"$defs": {
				"Node": {
					"type": "object",
					"properties": {
						"child": { "$ref": "#/$defs/Node" }
					}
				}
			}
		});
		to_openapi_schema(&mut schema);

		// Should complete without stack overflow.
		// The cyclic ref will remain as $ref (unresolved) due to cycle detection.
		assert!(schema.get("$defs").is_none());
		assert_eq!(schema["properties"]["node"]["type"], "object");
	}

	#[test]
	fn test_complex_schemars_schema() {
		// Realistic schema generated by schemars for a struct with Option<T> fields.
		let mut schema = json!({
			"$schema": "https://json-schema.org/draft/2020-12/schema",
			"type": "object",
			"properties": {
				"name": { "type": "string" },
				"age": {
					"anyOf": [
						{ "type": "integer" },
						{ "type": "null" }
					]
				},
				"address": {
					"anyOf": [
						{ "$ref": "#/$defs/Address" },
						{ "type": "null" }
					]
				}
			},
			"required": ["name"],
			"additionalProperties": false,
			"$defs": {
				"Address": {
					"type": "object",
					"properties": {
						"street": { "type": "string" },
						"zip": { "type": ["string", "null"] }
					},
					"required": ["street"],
					"additionalProperties": false
				}
			}
		});
		to_openapi_schema(&mut schema);

		// Top-level cleanup
		assert!(schema.get("$defs").is_none());
		assert!(schema.get("additionalProperties").is_none());

		// age: anyOf flattened
		assert_eq!(schema["properties"]["age"]["type"], "integer");
		assert!(schema["properties"]["age"].get("anyOf").is_none());

		// address: anyOf with $ref resolved and flattened
		assert_eq!(schema["properties"]["address"]["type"], "object");
		assert!(schema["properties"]["address"].get("anyOf").is_none());
		assert!(schema["properties"]["address"].get("$ref").is_none());
		assert!(schema["properties"]["address"].get("additionalProperties").is_none());

		// address.zip: nullable array type normalized
		assert_eq!(schema["properties"]["address"]["properties"]["zip"]["type"], "string");
	}

	#[test]
	fn test_nested_recursive_simplification() {
		let mut schema = json!({
			"type": "object",
			"properties": {
				"items": {
					"type": "array",
					"items": {
						"type": "object",
						"properties": {
							"value": { "type": ["number", "null"] }
						},
						"additionalProperties": false
					}
				}
			}
		});
		to_openapi_schema(&mut schema);

		let items_schema = &schema["properties"]["items"]["items"];
		assert_eq!(items_schema["properties"]["value"]["type"], "number");
		assert!(items_schema.get("additionalProperties").is_none());
	}

	#[test]
	fn test_multi_variant_composite_preserved() {
		// Multiple non-null variants should be preserved (not flattened).
		let mut schema = json!({
			"type": "object",
			"properties": {
				"data": {
					"oneOf": [
						{ "type": "string" },
						{ "type": "integer" }
					]
				}
			}
		});
		to_openapi_schema(&mut schema);

		// Two non-null variants: oneOf should be preserved.
		assert!(schema["properties"]["data"].get("oneOf").is_some());
		let variants = schema["properties"]["data"]["oneOf"].as_array().unwrap();
		assert_eq!(variants.len(), 2);
	}

	#[test]
	fn test_non_object_schema_passthrough() {
		let mut schema = json!("string");
		to_openapi_schema(&mut schema);
		assert_eq!(schema, json!("string"));
	}

	#[test]
	fn test_nested_ref_chain() {
		// A references B which references C.
		let mut schema = json!({
			"type": "object",
			"properties": {
				"a": { "$ref": "#/$defs/A" }
			},
			"$defs": {
				"A": {
					"type": "object",
					"properties": {
						"b": { "$ref": "#/$defs/B" }
					}
				},
				"B": {
					"type": "object",
					"properties": {
						"value": { "type": "string" }
					},
					"additionalProperties": false
				}
			}
		});
		to_openapi_schema(&mut schema);

		assert!(schema.get("$defs").is_none());
		assert_eq!(schema["properties"]["a"]["type"], "object");
		assert_eq!(schema["properties"]["a"]["properties"]["b"]["type"], "object");
		assert_eq!(
			schema["properties"]["a"]["properties"]["b"]["properties"]["value"]["type"],
			"string"
		);
		assert!(
			schema["properties"]["a"]["properties"]["b"]
				.get("additionalProperties")
				.is_none()
		);
	}
}

// endregion: --- Tests

use crate::chat::{ContentPart, ToolCall};
use crate::{Error, Result};
use serde_json::Value;
use value_ext::JsonValueExt;

/// Convert a OpenAI response output Item to a ContentPart
///
/// NOTE: At this point this is infallible, will ignore item that cannot be transformed
impl ContentPart {
	pub fn from_resp_output_item(mut item_value: Value) -> Result<Vec<Self>> {
		let mut parts = Vec::new();
		let Some(item_type) = ItemType::from_item_value(&item_value) else {
			return Ok(parts);
		};

		match item_type {
			ItemType::Message => {
				if let Ok(content) = item_value.x_remove::<Vec<Value>>("content") {
					// each content item {}
					for mut content_item in content {
						if let Ok("output_text") = content_item.x_get_str("type")
							&& let Ok(text) = content_item.x_remove::<String>("text")
						{
							parts.push(text.into())
						}
					}
				}
			}
			ItemType::FunctionCall => {
				let fn_name = item_value.x_remove::<String>("name")?;
				let call_id = item_value.x_remove::<String>("call_id")?;
				let arguments = item_value.x_remove::<String>("arguments")?;
				let fn_arguments: Value =
					serde_json::from_str(&arguments).map_err(|_| Error::InvalidJsonResponseElement {
						info: "tool call arguments is not an object.\nCause",
					})?;

				let tool_call = ToolCall {
					call_id,
					fn_name,
					fn_arguments,
					thought_signatures: None,
				};

				parts.push(tool_call.into());
			}
		}

		Ok(parts)
	}
}

// region:    --- Support Type

/// The managed
enum ItemType {
	Message,
	FunctionCall,
}

impl ItemType {
	fn from_item_value(item_value: &Value) -> Option<Self> {
		let typ = item_value.x_get_str("type").ok()?;
		match typ {
			"message" => Some(ItemType::Message),
			"function_call" => Some(ItemType::FunctionCall),
			_ => None,
		}
	}
}

// endregion: --- Support Type

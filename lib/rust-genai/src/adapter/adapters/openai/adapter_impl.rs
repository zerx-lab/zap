use crate::adapter::openai::OpenAIStreamer;
use crate::adapter::{Adapter, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{
	ChatOptionsSet, ChatRequest, ChatResponse, ChatStream, ChatStreamResponse, MessageContent, StopReason, ToolCall,
};
use crate::resolver::{AuthData, Endpoint};
use crate::webc::{EventSourceStream, WebResponse};
use crate::{Error, Result};
use crate::{ModelIden, ServiceTarget};
use reqwest::RequestBuilder;
use serde::Deserialize;
use serde_json::Value;
use value_ext::JsonValueExt;

pub struct OpenAIAdapter;

impl OpenAIAdapter {
	pub const API_KEY_DEFAULT_ENV_NAME: &str = "OPENAI_API_KEY";
}

impl Adapter for OpenAIAdapter {
	const DEFAULT_API_KEY_ENV_NAME: Option<&'static str> = Some(Self::API_KEY_DEFAULT_ENV_NAME);

	fn default_auth() -> AuthData {
		match Self::DEFAULT_API_KEY_ENV_NAME {
			Some(env_name) => AuthData::from_env(env_name),
			None => AuthData::None,
		}
	}

	fn default_endpoint() -> Endpoint {
		const BASE_URL: &str = "https://api.openai.com/v1/";
		Endpoint::from_static(BASE_URL)
	}

	/// Note: Currently returns the common models (see above)
	async fn all_model_names(kind: AdapterKind, endpoint: Endpoint, auth: AuthData) -> Result<Vec<String>> {
		OpenAIAdapter::list_model_names_for_end_target(kind, endpoint, auth).await
	}

	fn get_service_url(model: &ModelIden, service_type: ServiceType, endpoint: Endpoint) -> Result<String> {
		Self::util_get_service_url(model, service_type, endpoint)
	}

	fn to_web_request_data(
		target: ServiceTarget,
		service_type: ServiceType,
		chat_req: ChatRequest,
		chat_options: ChatOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		OpenAIAdapter::util_to_web_request_data(target, service_type, chat_req, chat_options, None)
	}

	fn to_chat_response(
		model_iden: ModelIden,
		web_response: WebResponse,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<ChatResponse> {
		let WebResponse { mut body, .. } = web_response;

		// -- Capture the provider_model_iden
		let provider_model_name: Option<String> = body.x_remove("model").ok();
		let provider_model_iden = model_iden.from_optional_name(provider_model_name);

		// -- Capture the usage
		let usage = body
			.x_take("usage")
			.map(|value| OpenAIAdapter::into_usage(model_iden.adapter_kind, value))
			.unwrap_or_default();

		// -- Capture the content
		let mut content: MessageContent = MessageContent::default();
		let mut reasoning_content: Option<String> = None;
		let mut stop_reason: Option<StopReason> = None;

		if let Ok(Some(mut first_choice)) = body.x_take::<Option<Value>>("/choices/0") {
			stop_reason = first_choice
				.x_take::<Option<String>>("finish_reason")
				.ok()
				.flatten()
				.map(StopReason::from);

			// Check if reasoning is present
			// Can be in two places:
			// - /message/reasoning
			// - /message/reasoning_content
			// Extracted before content as some model can return reasoning without content
			reasoning_content = first_choice
				.x_take::<Option<String>>("/message/reasoning")
				.ok()
				.unwrap_or_else(|| {
					first_choice
						.x_take::<Option<String>>("/message/reasoning_content")
						.ok()
						.flatten()
				})
				.map(|s| s.trim().to_string());

			// -- Push eventual text message
			if let Ok(Some(mut text_content)) = first_choice.x_take::<Option<String>>("/message/content") {
				text_content = text_content.trim().to_string();
				// If not reasoning_content, but
				if reasoning_content.is_none() && options_set.normalize_reasoning_content().unwrap_or_default() {
					let (content_tmp, reasoning_content_tmp) = extract_think(text_content);
					reasoning_content = reasoning_content_tmp;
					text_content = content_tmp;
				}

				// After extracting reasoning_content, sometimes the content is empty.
				if !text_content.is_empty() {
					content.push(text_content);
				}
			}

			// -- Push eventual ToolCalls
			if let Some(tool_calls) = first_choice
				.x_take("/message/tool_calls")
				.ok()
				.map(parse_tool_calls)
				.transpose()?
				.map(MessageContent::from_tool_calls)
			{
				content.extend(tool_calls);
			}
		}

		Ok(ChatResponse {
			content,
			reasoning_content,
			model_iden,
			provider_model_iden,
			stop_reason,
			usage,
			captured_raw_body: None, // Set by the client exec_chat
			response_id: None,
		})
	}

	fn to_chat_stream(
		model_iden: ModelIden,
		reqwest_builder: RequestBuilder,
		options_sets: ChatOptionsSet<'_, '_>,
	) -> Result<ChatStreamResponse> {
		let event_source = EventSourceStream::new(reqwest_builder);
		let openai_stream = OpenAIStreamer::new(event_source, model_iden.clone(), options_sets);
		let chat_stream = ChatStream::from_inter_stream(openai_stream);

		Ok(ChatStreamResponse {
			model_iden,
			stream: chat_stream,
		})
	}

	fn to_embed_request_data(
		service_target: ServiceTarget,
		embed_req: crate::embed::EmbedRequest,
		options_set: crate::embed::EmbedOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		super::embed::to_embed_request_data(service_target, embed_req, options_set)
	}

	fn to_embed_response(
		model_iden: ModelIden,
		web_response: WebResponse,
		options_set: crate::embed::EmbedOptionsSet<'_, '_>,
	) -> Result<crate::embed::EmbedResponse> {
		super::embed::to_embed_response(model_iden, web_response, options_set)
	}
}

// region:    --- Support

fn extract_think(content: String) -> (String, Option<String>) {
	let start_tag = "<think>";
	let end_tag = "</think>";

	if let Some(start) = content.find(start_tag)
		&& let Some(end) = content[start + start_tag.len()..].find(end_tag)
	{
		let start_pos = start;
		let end_pos = start + start_tag.len() + end;

		let think_content = &content[start_pos + start_tag.len()..end_pos];
		let think_content = think_content.trim();

		// Extract parts of the original content without cloning until necessary
		let before_think = &content[..start_pos];
		let after_think = &content[end_pos + end_tag.len()..];

		// Remove a leading newline in `after_think` if it starts with '\n'
		let after_think = after_think.trim_start();

		// Construct the final cleaned content in one allocation
		let cleaned_content = format!("{before_think}{after_think}");

		return (cleaned_content, Some(think_content.to_string()));
	}

	(content, None)
}

fn parse_tool_calls(raw_tool_calls: Value) -> Result<Vec<ToolCall>> {
	// Some backends (like sglang) return null if no tool calls are present.
	if raw_tool_calls.is_null() {
		return Ok(vec![]);
	}

	let Value::Array(raw_tool_calls) = raw_tool_calls else {
		return Err(Error::InvalidJsonResponseElement {
			info: "tool calls is not an array",
		});
	};

	let tool_calls = raw_tool_calls.into_iter().map(parse_tool_call).collect::<Result<Vec<_>>>()?;

	Ok(tool_calls)
}

fn parse_tool_call(raw_tool_call: Value) -> Result<ToolCall> {
	// Define a helper struct to match the original JSON structure.
	#[derive(Deserialize)]
	struct IterimToolFnCall {
		id: String,
		#[allow(unused)]
		#[serde(rename = "type")]
		r#type: String,
		function: IterimFunction,
	}

	#[derive(Deserialize)]
	struct IterimFunction {
		name: String,
		arguments: Value,
	}

	let iterim = serde_json::from_value::<IterimToolFnCall>(raw_tool_call)?;

	let fn_name = iterim.function.name;

	// For now, support Object only, and parse the eventual string as a json value.
	// Eventually, we might check pricing
	let fn_arguments = match iterim.function.arguments {
		Value::Object(obj) => Value::Object(obj),
		Value::String(txt) => serde_json::from_str(&txt)?,
		_ => {
			return Err(Error::InvalidJsonResponseElement {
				info: "tool call arguments is not an object",
			});
		}
	};

	// Then, map the fields of the helper struct to the flat structure.
	Ok(ToolCall {
		call_id: iterim.id,
		fn_name,
		fn_arguments,
		thought_signatures: None,
	})
}

// endregion: --- Support

#[cfg(test)]
mod tests {
	use super::*;
	use crate::adapter::AdapterKind;
	use reqwest::StatusCode;

	fn test_model() -> ModelIden {
		ModelIden::new(AdapterKind::OpenAI, "gpt-4o-mini")
	}

	#[test]
	fn test_to_chat_response_captures_finish_reason_as_stop_reason() {
		let web_response = WebResponse {
			status: StatusCode::OK,
			body: serde_json::json!({
				"id": "chatcmpl-test",
				"model": "gpt-4o-mini-2024-07-18",
				"usage": {"prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15},
				"choices": [{
					"finish_reason": "stop",
					"message": {"role": "assistant", "content": "hello"}
				}]
			}),
		};

		let response = OpenAIAdapter::to_chat_response(test_model(), web_response, ChatOptionsSet::default())
			.expect("chat response");

		assert_eq!(response.stop_reason, Some(StopReason::Completed("stop".to_string())));
		assert_eq!(response.first_text(), Some("hello"));
	}

	#[test]
	fn test_to_chat_response_stop_reason_none_when_missing() {
		let web_response = WebResponse {
			status: StatusCode::OK,
			body: serde_json::json!({
				"id": "chatcmpl-test",
				"model": "gpt-4o-mini-2024-07-18",
				"usage": {"prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15},
				"choices": [{
					"message": {"role": "assistant", "content": "hello"}
				}]
			}),
		};

		let response = OpenAIAdapter::to_chat_response(test_model(), web_response, ChatOptionsSet::default())
			.expect("chat response");

		assert_eq!(response.stop_reason, None);
	}
}

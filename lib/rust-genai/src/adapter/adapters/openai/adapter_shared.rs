//! This is support implementation of the OpenAI Adapter which can also be called by other OpenAI Adapter Variants

use crate::adapter::adapters::support::get_api_key;
use crate::adapter::openai::OpenAIAdapter;
use crate::adapter::{AdapterDispatcher, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{
	BinarySource, CacheControl, ChatOptionsSet, ChatRequest, ChatResponseFormat, ChatRole, ContentPart,
	ReasoningEffort, Usage,
};
use crate::resolver::{AuthData, Endpoint};
use crate::{Error, Headers, Result};
use crate::{ModelIden, ServiceTarget};
use serde_json::{Value, json};
use tracing::error;
use tracing::warn;
use value_ext::JsonValueExt;

fn insert_openai_reasoning_effort(payload: &mut Value, effort: &ReasoningEffort) -> Result<()> {
	let keyword = match effort {
		ReasoningEffort::None => "none",
		ReasoningEffort::Low => "low",
		ReasoningEffort::Medium => "medium",
		ReasoningEffort::High => "high",
		ReasoningEffort::XHigh | ReasoningEffort::Max => "xhigh",
		ReasoningEffort::Minimal => "minimal",
		ReasoningEffort::Budget(_) => return Ok(()),
	};

	payload.x_insert("reasoning_effort", keyword)?;

	Ok(())
}

/// Support functions for other adapters that share OpenAI APIs
impl OpenAIAdapter {
	pub(in crate::adapter::adapters) fn util_get_service_url(
		_model: &ModelIden,
		service_type: ServiceType,
		// -- utility arguments
		default_endpoint: Endpoint,
	) -> Result<String> {
		let base_url = default_endpoint.base_url();
		// Parse into URL and query-params
		let base_url = reqwest::Url::parse(base_url)
			.map_err(|err| Error::Internal(format!("Cannot parse url: {base_url}. Cause:\n{err}")))?;
		let original_query_params = base_url.query().to_owned();

		let suffix = match service_type {
			ServiceType::Chat | ServiceType::ChatStream => "chat/completions",
			ServiceType::Embed => "embeddings",
		};
		let mut full_url = base_url.join(suffix).map_err(|err| {
			Error::Internal(format!(
				"Cannot join suffix '{suffix}' for url: {base_url}. Cause:\n{err}"
			))
		})?;
		full_url.set_query(original_query_params);
		Ok(full_url.to_string())
	}

	/// Shared OpenAI to_web_request_data for various OpenAI compatible adapters
	pub(in crate::adapter::adapters) fn util_to_web_request_data(
		target: ServiceTarget,
		service_type: ServiceType,
		chat_req: ChatRequest,
		options_set: ChatOptionsSet<'_, '_>,
		custom: Option<ToWebRequestCustom>,
	) -> Result<WebRequestData> {
		let ServiceTarget { model, auth, endpoint } = target;
		let (_, model_name) = model.model_name.namespace_and_name();
		let adapter_kind = model.adapter_kind;

		// -- url
		let url = AdapterDispatcher::get_service_url(&model, service_type, endpoint)?;

		// -- api_key / headers
		let api_key = get_api_key(auth, &model)?;
		let headers = Headers::from(("Authorization".to_string(), format!("Bearer {api_key}")));

		let stream = matches!(service_type, ServiceType::ChatStream);

		// -- compute reasoning_effort and eventual trimmed model_name
		// OpenWarp fork: openai-compat providers that accept top-level `reasoning_effort`
		// per their own docs (DeepSeek's thinking_mode, Kimi via openai compat, etc).
		// Upstream genai gates this on `OpenAI` only; we widen to DeepSeek here.
		// DeepSeek docs: https://api-docs.deepseek.com/zh-cn/guides/thinking_mode
		let (reasoning_effort, model_name): (Option<ReasoningEffort>, &str) =
			if matches!(adapter_kind, AdapterKind::OpenAI | AdapterKind::DeepSeek) {
				let (reasoning_effort, model_name) = options_set
					.reasoning_effort()
					.cloned()
					.map(|v| (Some(v), model_name))
					.unwrap_or_else(|| ReasoningEffort::from_model_name(model_name));

				(reasoning_effort, model_name)
			} else {
				(None, model_name)
			};

		// -- Build the basic payload

		let OpenAIRequestParts { messages, tools } = Self::into_openai_request_parts(&model, chat_req)?;
		let mut payload = json!({
			"model": model_name,
			"messages": messages,
			"stream": stream
		});

		// -- Set reasoning effort
		if let Some(reasoning_effort) = reasoning_effort {
			insert_openai_reasoning_effort(&mut payload, &reasoning_effort)?;
		}

		// -- Set verbosity
		if let Some(verbosity) = options_set.verbosity()
			&& let Some(keyword) = verbosity.as_keyword()
		{
			payload.x_insert("verbosity", keyword)?;
		}

		// -- Tools
		if let Some(tools) = tools {
			payload.x_insert("/tools", tools)?;
		}

		// -- Add options
		let response_format = if let Some(response_format) = options_set.response_format() {
			match response_format {
				ChatResponseFormat::JsonMode => Some(json!({"type": "json_object"})),
				ChatResponseFormat::JsonSpec(st_json) => {
					// "type": "json_schema", "json_schema": {...}
					Some(json!({
						"type": "json_schema",
						"json_schema": {
							"name": st_json.name.clone(),
							"strict": true,
							// TODO: add description
							"schema": st_json.schema_with_additional_properties_false(),
						}
					}))
				}
			}
		} else {
			None
		};

		if let Some(response_format) = response_format {
			payload["response_format"] = response_format;
		}

		// -- Add supported ChatOptions
		if stream & options_set.capture_usage().unwrap_or(false) {
			payload.x_insert("stream_options", json!({"include_usage": true}))?;
		}

		if let Some(temperature) = options_set.temperature() {
			payload.x_insert("temperature", temperature)?;
		}

		if !options_set.stop_sequences().is_empty() {
			payload.x_insert("stop", options_set.stop_sequences())?;
		}

		// GPT-5.x and o-series models require "max_completion_tokens" instead of "max_tokens"
		let max_tokens_key = if model_name.starts_with("gpt-5")
			|| model_name.starts_with("o1")
			|| model_name.starts_with("o3")
			|| model_name.starts_with("o4")
		{
			"max_completion_tokens"
		} else {
			"max_tokens"
		};
		if let Some(max_tokens) = options_set.max_tokens() {
			payload.x_insert(max_tokens_key, max_tokens)?;
		} else if let Some(custom) = custom.as_ref()
			&& let Some(max_tokens) = custom.default_max_tokens
		{
			payload.x_insert(max_tokens_key, max_tokens)?;
		}
		if let Some(top_p) = options_set.top_p() {
			payload.x_insert("top_p", top_p)?;
		}
		if let Some(seed) = options_set.seed() {
			payload.x_insert("seed", seed)?;
		}
		if let Some(service_tier) = options_set.service_tier()
			&& let Some(keyword) = service_tier.as_keyword()
		{
			payload.x_insert("service_tier", keyword)?;
		}

		// -- OpenAI prompt cache options
		if let Some(prompt_cache_key) = options_set.prompt_cache_key() {
			payload.x_insert("prompt_cache_key", prompt_cache_key)?;
		}
		if let Some(cache_control) = options_set.cache_control() {
			let prompt_cache_retention = match cache_control {
				CacheControl::Memory | CacheControl::Ephemeral => Some("in_memory"),
				CacheControl::Ephemeral24h => Some("24h"),
				CacheControl::Ephemeral5m | CacheControl::Ephemeral1h => None,
			};
			if let Some(prompt_cache_retention) = prompt_cache_retention {
				payload.x_insert("prompt_cache_retention", prompt_cache_retention)?;
			}
		}

		// -- OpenWarp fork: shallow-merge extra_body into payload at top level.
		// Provider-specific fields (e.g. DeepSeek `thinking.type=disabled`) the typed
		// API does not yet expose. Existing keys are not overwritten — typed fields win.
		if let Some(extra_body) = options_set.extra_body()
			&& let (Some(extra_obj), Some(payload_obj)) = (extra_body.as_object(), payload.as_object_mut())
		{
			for (k, v) in extra_obj {
				payload_obj.entry(k.clone()).or_insert_with(|| v.clone());
			}
		}

		Ok(WebRequestData { url, headers, payload })
	}

	/// Note: Needs to be called from super::streamer as well
	pub(super) fn into_usage(adapter: AdapterKind, usage_value: Value) -> Usage {
		// NOTE: here we make sure we do not fail since we do not want to break a response because usage parsing fail
		let usage = serde_json::from_value(usage_value).map_err(|err| {
			error!("Fail to deserialize usage. Cause: {err}");
			err
		});
		let mut usage: Usage = usage.unwrap_or_default();
		// Will set details to None if no values
		usage.compact_details();

		// Unfortunately, xAI grok-3 does not compute reasoning tokens correctly.
		// Example: completion_tokens: 35, completion_tokens_details.reasoning_tokens: 192
		// BUT completion_tokens should be 35 + 192.
		// TODO: We might want to do this for other token details as well.
		// TODO: We could check if the math adds up first with the total token count, and only change it if it does not.
		//       This will allow us to be forward compatible if/when they fix this bug (yes, it is a bug).
		if matches!(adapter, AdapterKind::Xai)
			&& let Some(reasoning_tokens) = usage.completion_tokens_details.as_ref().and_then(|d| d.reasoning_tokens)
		{
			let completion_tokens = usage.completion_tokens.unwrap_or(0);
			usage.completion_tokens = Some(completion_tokens + reasoning_tokens)
		}

		usage
	}

	/// Takes the genai ChatMessages and builds the OpenAIChatRequestParts
	/// - `genai::ChatRequest.system`, if present, is added as the first message with role 'system'.
	/// - All messages get added with the corresponding roles (tools are not supported for now)
	fn into_openai_request_parts(_model_iden: &ModelIden, chat_req: ChatRequest) -> Result<OpenAIRequestParts> {
		let mut messages: Vec<Value> = Vec::new();

		// -- Process the system
		if let Some(system_msg) = chat_req.system {
			messages.push(json!({"role": "system", "content": system_msg}));
		}

		// -- Process the messages
		for msg in chat_req.messages {
			// Note: Will handle more types later
			match msg.role {
				// For now, system and tool messages go to the system
				ChatRole::System => {
					if let Some(content) = msg.content.into_joined_texts() {
						messages.push(json!({"role": "system", "content": content}))
					}
					// TODO: Probably need to warn if it is a ToolCalls type of content
				}

				// User - For now support Text and Binary
				ChatRole::User => {
					// -- If we have only text, then, we jjust returned the joined_texts
					if msg.content.is_text_only() {
						// NOTE: for now, if no content, just return empty string (respect current logic)
						let content = json!(msg.content.joined_texts().unwrap_or_else(String::new));
						messages.push(json! ({"role": "user", "content": content}));
					} else {
						let mut values: Vec<Value> = Vec::new();
						for part in msg.content {
							match part {
								ContentPart::Text(content) => values.push(json!({"type": "text", "text": content})),
								ContentPart::Binary(binary) => {
									let is_audio = binary.is_audio();
									let is_image = binary.is_image();

									// let Binary {
									// 	content_type, source, ..
									// } = binary;

									if is_audio {
										match &binary.source {
											BinarySource::Url(_url) => {
												warn!(
													"OpenAI doesn't support audio from URL, need to handle it gracefully"
												);
											}
											BinarySource::Base64(content) => {
												let mut format =
													binary.content_type.split('/').next_back().unwrap_or("");
												if format == "mpeg" {
													format = "mp3";
												}
												values.push(json!({
													"type": "input_audio",
													"input_audio": {
														"data": content,
														"format": format
													}
												}));
											}
										}
									} else if is_image {
										let image_url = binary.into_url();
										values.push(json!({"type": "image_url", "image_url": {"url": image_url}}));
									} else if matches!(&binary.source, BinarySource::Url(_)) {
										// TODO: Need to return error
										warn!("OpenAI doesn't support file from URL, need to handle it gracefully");
									} else {
										let filename = binary.name.clone();
										let file_base64_url = binary.into_url();
										values.push(json!({"type": "file", "file": {
											"filename": filename,
											"file_data": file_base64_url
										}}))
									}
								}

								// Use `match` instead of `if let`. This will allow to future-proof this
								// implementation in case some new message content types would appear,
								// this way library would not compile if not all methods are implemented
								// continue would allow to gracefully skip pushing unserializable message
								// TODO: Probably need to warn if it is a ToolCalls type of content
								ContentPart::ToolCall(_) => (),
								ContentPart::ToolResponse(_) => (),
								ContentPart::ThoughtSignature(_) => (),
								ContentPart::ReasoningContent(_) => (),
								// Custom are ignored for this logic
								ContentPart::Custom(_) => {}
							}
						}
						messages.push(json! ({"role": "user", "content": values}));
					}
				}

				// Assistant - For now support Text and ToolCalls
				ChatRole::Assistant => {
					let mut texts: Vec<String> = Vec::new();
					let mut tool_calls: Vec<Value> = Vec::new();
					let mut reasoning_parts: Vec<String> = Vec::new();
					for part in msg.content {
						match part {
							ContentPart::Text(text) => texts.push(text),
							ContentPart::ToolCall(tool_call) => {
								//
								tool_calls.push(json!({
									"type": "function",
									"id": tool_call.call_id,
									"function": {
										"name": tool_call.fn_name,
										"arguments": tool_call.fn_arguments.to_string(),
									}
								}))
							}
							// Extract reasoning content parts to hoist into sibling field
							ContentPart::ReasoningContent(reasoning) => reasoning_parts.push(reasoning),

							// TODO: Probably need towarn on this one (probably need to add binary here)
							ContentPart::Binary(_) => (),
							ContentPart::ToolResponse(_) => (),
							ContentPart::ThoughtSignature(_) => {}
							// Custom are ignored for this logic
							ContentPart::Custom(_) => {}
						}
					}
					let content = texts.join("\n\n");
					let mut message = json!({"role": "assistant", "content": content});
					if !tool_calls.is_empty() {
						message.x_insert("tool_calls", tool_calls)?;
					}
					// Echo reasoning_content back for providers that require it (Kimi, DeepSeek)
					// Note: In practice there is at most one ReasoningContent part per message,
					//       but we join defensively in case multiple parts are present.
					if !reasoning_parts.is_empty() {
						message.x_insert("reasoning_content", reasoning_parts.join("\n"))?;
					}
					messages.push(message);
				}

				// Tool - For now, support only tool responses
				ChatRole::Tool => {
					for part in msg.content {
						if let ContentPart::ToolResponse(tool_response) = part {
							messages.push(json!({
								"role": "tool",
								"content": tool_response.content,
								"tool_call_id": tool_response.call_id,
							}))
						}
					}

					// TODO: Probably need to trace/warn that this will be ignored
				}
			}
		}

		// -- Process the tools
		let tools = chat_req.tools.map(|tools| {
			tools
				.into_iter()
				.map(|tool| {
					let strict = tool.strict.unwrap_or(false);
					let mut parameters = tool.schema;

					// When strict mode is enabled, OpenAI requires `additionalProperties: false`
					// on every object node in the schema.
					if strict && let Some(ref mut schema_val) = parameters {
						schema_val.x_walk(|parent_map, prop_name| {
							if prop_name == "type" {
								let typ = parent_map.get("type").and_then(|v| v.as_str()).unwrap_or("");
								if typ == "object" {
									parent_map.insert("additionalProperties".to_string(), false.into());
								}
							}
							true
						});
					}

					json!({
						"type": "function",
						"function": {
							"name": tool.name,
							"description": tool.description,
							"parameters": parameters,
							"strict": strict,
						}
					})
				})
				.collect::<Vec<Value>>()
		});

		Ok(OpenAIRequestParts { messages, tools })
	}

	pub(in crate::adapter::adapters) async fn list_model_names_for_end_target(
		kind: AdapterKind,
		endpoint: Endpoint,
		auth: AuthData,
	) -> Result<Vec<String>> {
		// -- url
		let base_url = endpoint.base_url();
		let url = format!("{base_url}models");

		// -- auth / headers
		let api_key = auth.single_key_value().ok();
		let headers = api_key
			.map(|api_key| Headers::from(("Authorization".to_string(), format!("Bearer {api_key}"))))
			.unwrap_or_default();

		// -- Exec request
		let web_c = crate::webc::WebClient::default();
		let mut res = web_c.do_get(&url, &headers).await.map_err(|webc_error| Error::WebAdapterCall {
			adapter_kind: kind,
			webc_error,
		})?;

		// -- Format result
		let mut models: Vec<String> = Vec::new();

		if let Value::Array(models_value) = res.body.x_take("data")? {
			for mut model in models_value {
				let model_name: String = model.x_take("id")?;
				models.push(model_name);
			}
		} else {
			// TODO: Need to add tracing
			// error!("OllamaAdapter::list_models did not have any models {res:?}");
		}

		Ok(models)
	}
}

/// Custom OpenAI structure for Adapters to use to customize
/// the default [`OpenAIAdapter::util_to_web_request_data`]
pub struct ToWebRequestCustom {
	pub default_max_tokens: Option<u32>,
}

// region:    --- Support

struct OpenAIRequestParts {
	messages: Vec<Value>,
	tools: Option<Vec<Value>>,
}

// endregion: --- Support

// region:    --- Tests

#[cfg(test)]
mod tests {
	use super::*;
	use crate::adapter::AdapterKind;
	use crate::chat::{ChatMessage, ContentPart, MessageContent, ToolCall};

	fn test_model() -> ModelIden {
		ModelIden::new(AdapterKind::OpenAI, "test-model")
	}

	/// When an assistant message carries reasoning_content, it must appear
	/// in the serialized JSON so providers that require it (Kimi, DeepSeek)
	/// don't reject the request.
	#[test]
	fn test_reasoning_content_serialized_on_assistant_message() {
		let tool_call = ToolCall {
			call_id: "call_1".to_string(),
			fn_name: "get_weather".to_string(),
			fn_arguments: serde_json::json!({"city": "Paris"}),
			thought_signatures: None,
		};

		let assistant_msg = ChatMessage::assistant(MessageContent::from_parts(vec![
			ContentPart::Text("Let me check.".to_string()),
			ContentPart::ToolCall(tool_call),
		]))
		.with_reasoning_content(Some("I should look up the weather.".to_string()));

		let chat_req = ChatRequest::new(vec![ChatMessage::user("What's the weather in Paris?"), assistant_msg]);

		let parts = OpenAIAdapter::into_openai_request_parts(&test_model(), chat_req).expect("should serialize");

		// The assistant message is the second message (after user)
		let assistant_json = &parts.messages[1];
		assert_eq!(assistant_json["role"], "assistant");
		assert_eq!(
			assistant_json["reasoning_content"], "I should look up the weather.",
			"reasoning_content should be present in serialized assistant message"
		);
	}

	/// When reasoning_content is None, the field should not appear in the JSON.
	#[test]
	fn test_no_reasoning_content_when_absent() {
		let chat_req = ChatRequest::new(vec![ChatMessage::user("Hello"), ChatMessage::assistant("Hi there!")]);

		let parts = OpenAIAdapter::into_openai_request_parts(&test_model(), chat_req).expect("should serialize");

		let assistant_json = &parts.messages[1];
		assert_eq!(assistant_json["role"], "assistant");
		assert!(
			assistant_json.get("reasoning_content").is_none(),
			"reasoning_content should be absent when not set"
		);
	}
}

// endregion: --- Tests

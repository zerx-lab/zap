use crate::adapter::adapters::support::get_api_key;
use crate::adapter::gemini::GeminiStreamer;
use crate::adapter::{Adapter, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{
	Binary, BinarySource, ChatOptionsSet, ChatRequest, ChatResponse, ChatResponseFormat, ChatRole, ChatStream,
	ChatStreamResponse, CompletionTokensDetails, ContentPart, MessageContent, PromptTokensDetails, ReasoningEffort,
	StopReason, Tool, ToolCall, ToolConfig, ToolName, Usage,
};
use crate::resolver::{AuthData, Endpoint};
use crate::webc::{WebResponse, WebStream};
use crate::{Error, Headers, ModelIden, Result, ServiceTarget};
use reqwest::RequestBuilder;
use serde_json::{Value, json};
use value_ext::JsonValueExt;

pub struct GeminiAdapter;

// Per gemini doc (https://x.com/jeremychone/status/1916501987371438372)
pub(in crate::adapter) const REASONING_ZERO: u32 = 0;
pub(in crate::adapter) const REASONING_LOW: u32 = 1000;
pub(in crate::adapter) const REASONING_MEDIUM: u32 = 8000;
pub(in crate::adapter) const REASONING_HIGH: u32 = 24000;

/// Important
/// - For now Low and Minimal aare the same for geminia
/// -
fn insert_gemini_thinking_budget_value(payload: &mut Value, effort: &ReasoningEffort) -> Result<()> {
	// -- for now, match minimal to Low (because zero is not supported by 2.5 pro)
	let budget = match effort {
		ReasoningEffort::None => None,
		ReasoningEffort::Low | ReasoningEffort::Minimal => Some(REASONING_LOW),
		ReasoningEffort::Medium => Some(REASONING_MEDIUM),
		ReasoningEffort::High | ReasoningEffort::Max | ReasoningEffort::XHigh => Some(REASONING_HIGH),
		ReasoningEffort::Budget(budget) => Some(*budget),
	};

	if let Some(budget) = budget {
		payload.x_insert("/generationConfig/thinkingConfig/thinkingBudget", budget)?;
	}
	Ok(())
}

// curl \
//   -H 'Content-Type: application/json' \
//   -d '{"contents":[{"parts":[{"text":"Explain how AI works"}]}]}' \
//   -X POST 'https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-flash-latest:generateContent?key=YOUR_API_KEY'

impl GeminiAdapter {
	pub const API_KEY_DEFAULT_ENV_NAME: &str = "GEMINI_API_KEY";
}

impl Adapter for GeminiAdapter {
	const DEFAULT_API_KEY_ENV_NAME: Option<&'static str> = Some(Self::API_KEY_DEFAULT_ENV_NAME);

	fn default_endpoint() -> Endpoint {
		const BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta/";
		Endpoint::from_static(BASE_URL)
	}

	fn default_auth() -> AuthData {
		match Self::DEFAULT_API_KEY_ENV_NAME {
			Some(env_name) => AuthData::from_env(env_name),
			None => AuthData::None,
		}
	}

	async fn all_model_names(kind: AdapterKind, endpoint: Endpoint, auth: AuthData) -> Result<Vec<String>> {
		// -- url
		let base_url = endpoint.base_url();
		let url = format!("{base_url}models");

		// -- auth / headers
		let api_key = auth.single_key_value().ok();
		let headers = api_key
			.map(|api_key| Headers::from(("x-goog-api-key".to_string(), api_key)))
			.unwrap_or_default();

		// -- Exec request
		let web_c = crate::webc::WebClient::default();
		let mut res = web_c.do_get(&url, &headers).await.map_err(|webc_error| Error::WebAdapterCall {
			adapter_kind: kind,
			webc_error,
		})?;

		// -- Format result
		let mut models: Vec<String> = Vec::new();

		if let Value::Array(models_value) = res.body.x_take("models")? {
			for mut model in models_value {
				let model_name: String = model.x_take("name")?;
				// Gemini model names are usually prefixed with "models/"
				let model_name = model_name.strip_prefix("models/").unwrap_or(&model_name).to_string();
				models.push(model_name);
			}
		}

		Ok(models)
	}

	/// NOTE: As Google Gemini has decided to put their API_KEY in the URL,
	///       this will return the URL without the API_KEY in it. The API_KEY will need to be added by the caller.
	fn get_service_url(model: &ModelIden, service_type: ServiceType, endpoint: Endpoint) -> Result<String> {
		let base_url = endpoint.base_url();
		let (_, model_name) = model.model_name.namespace_and_name();
		let url = match service_type {
			ServiceType::Chat => format!("{base_url}models/{model_name}:generateContent"),
			ServiceType::ChatStream => format!("{base_url}models/{model_name}:streamGenerateContent"),
			ServiceType::Embed => format!("{base_url}models/{model_name}:embedContent"), // Gemini embeddings API
		};
		Ok(url)
	}

	fn to_web_request_data(
		target: ServiceTarget,
		service_type: ServiceType,
		chat_req: ChatRequest,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		let ServiceTarget { endpoint, auth, model } = target;
		let (_, model_name) = model.model_name.namespace_and_name();

		// -- api_key
		let api_key = get_api_key(auth, &model)?;
		let headers = Headers::from(("x-goog-api-key".to_string(), api_key.to_string()));

		let (payload, provider_model_name) =
			Self::build_gemini_request_payload(&model, model_name, chat_req, options_set)?;

		// -- url
		let provider_model = model.from_name(&provider_model_name);
		let url = Self::get_service_url(&provider_model, service_type, endpoint)?;

		Ok(WebRequestData { url, headers, payload })
	}

	fn to_chat_response(
		model_iden: ModelIden,
		web_response: WebResponse,
		_options_set: ChatOptionsSet<'_, '_>,
	) -> Result<ChatResponse> {
		let WebResponse { mut body, .. } = web_response;

		// -- Capture the provider_model_iden
		// TODO: Need to be implemented (if available), for now, just clone model_iden
		let provider_model_name: Option<String> = body.x_remove("modelVersion").ok();
		let provider_model_iden = model_iden.from_optional_name(provider_model_name);
		let gemini_response = Self::body_to_gemini_chat_response(&model_iden.clone(), body)?;
		let GeminiChatResponse {
			content: gemini_content,
			usage,
			stop_reason,
		} = gemini_response;
		let stop_reason = stop_reason.map(StopReason::from);

		let mut thoughts: Vec<String> = Vec::new();
		let mut reasonings: Vec<String> = Vec::new();
		let mut texts: Vec<String> = Vec::new();
		let mut tool_calls: Vec<ToolCall> = Vec::new();
		let mut binary_parts: Vec<Binary> = Vec::new();

		for g_item in gemini_content {
			match g_item {
				GeminiChatContent::Text(text) => texts.push(text),
				GeminiChatContent::Binary(binary) => binary_parts.push(binary),
				GeminiChatContent::ToolCall(tool_call) => tool_calls.push(tool_call),
				GeminiChatContent::ThoughtSignature(thought) => thoughts.push(thought),
				GeminiChatContent::Reasoning(reasoning_text) => reasonings.push(reasoning_text),
			}
		}

		let thought_signatures_for_call = (!thoughts.is_empty() && !tool_calls.is_empty()).then(|| thoughts.clone());
		let mut parts: Vec<ContentPart> = thoughts.into_iter().map(ContentPart::ThoughtSignature).collect();

		if let Some(signatures) = thought_signatures_for_call
			&& let Some(first_call) = tool_calls.first_mut()
		{
			first_call.thought_signatures = Some(signatures);
		}

		if !texts.is_empty() {
			let total_len: usize = texts.iter().map(|t| t.len()).sum();
			let mut combined_text = String::with_capacity(total_len);
			for text in texts {
				combined_text.push_str(&text);
			}
			if !combined_text.is_empty() {
				parts.push(ContentPart::Text(combined_text));
			}
		}
		let mut reasoning_text = String::new();
		if !reasonings.is_empty() {
			for reasoning in &reasonings {
				reasoning_text.push_str(reasoning);
			}
		}

		if !binary_parts.is_empty() {
			for binary in binary_parts {
				parts.push(ContentPart::Binary(binary));
			}
		}

		parts.extend(tool_calls.into_iter().map(ContentPart::ToolCall));
		let content = MessageContent::from_parts(parts);

		Ok(ChatResponse {
			content,
			reasoning_content: Some(reasoning_text),
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
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<ChatStreamResponse> {
		let web_stream = WebStream::new_with_pretty_json_array(reqwest_builder);

		let gemini_stream = GeminiStreamer::new(web_stream, model_iden.clone(), options_set);
		let chat_stream = ChatStream::from_inter_stream(gemini_stream);

		Ok(ChatStreamResponse {
			model_iden,
			stream: chat_stream,
		})
	}

	fn to_embed_request_data(
		service_target: crate::ServiceTarget,
		embed_req: crate::embed::EmbedRequest,
		options_set: crate::embed::EmbedOptionsSet<'_, '_>,
	) -> Result<crate::adapter::WebRequestData> {
		super::embed::to_embed_request_data(service_target, embed_req, options_set)
	}

	fn to_embed_response(
		model_iden: crate::ModelIden,
		web_response: crate::webc::WebResponse,
		options_set: crate::embed::EmbedOptionsSet<'_, '_>,
	) -> Result<crate::embed::EmbedResponse> {
		super::embed::to_embed_response(model_iden, web_response, options_set)
	}
}

// region:    --- Support

/// Support functions for GeminiAdapter
impl GeminiAdapter {
	pub(in crate::adapter) fn body_to_gemini_chat_response(
		model_iden: &ModelIden,
		mut body: Value,
	) -> Result<GeminiChatResponse> {
		// If the body has an `error` property, then it is assumed to be an error.
		if body.get("error").is_some() {
			return Err(Error::ChatResponse {
				model_iden: model_iden.clone(),
				body,
			});
		}

		let mut content: Vec<GeminiChatContent> = Vec::new();

		// Extract usage before content/parts so it is available even in
		// usage-only tail frames (finishReason + usageMetadata but no content).
		let usage = body.x_take::<Value>("usageMetadata").map(Self::into_usage).unwrap_or_default();

		// -- Read multipart
		let parts = match body.x_take::<Vec<Value>>("/candidates/0/content/parts") {
			Ok(parts) => parts,
			Err(_) => {
				let finish_reason = body
					.x_remove::<String>("/candidates/0/finishReason")
					.ok()
					.or_else(|| body.x_remove::<String>("/candidates/finishReason").ok());
				let saw_usage_only_tail = body.get("candidates").is_some() || finish_reason.is_some();

				// Gemini streaming sends a final frame with finishReason + usageMetadata
				// but no content.parts. This is normal — return Ok with empty content.
				if saw_usage_only_tail {
					return Ok(GeminiChatResponse {
						content,
						usage,
						stop_reason: finish_reason,
					});
				}

				let body = json!({
					"finishReason": finish_reason,
					"usageMetadata": Value::Null,
				});
				return Err(Error::ChatResponse {
					model_iden: model_iden.clone(),
					body,
				});
			}
		};

		let mut tool_call_counter: usize = 0;
		for mut part in parts {
			// Each Gemini response part may contain one or more of:
			// thoughtSignature, thought+text (reasoning), functionCall, text.
			// We extract them in priority order.

			// -- Thought signature (Gemini 3+) or legacy thought boolean
			if let Some(sig) = take_string(&mut part, "thoughtSignature") {
				content.push(GeminiChatContent::ThoughtSignature(sig));
			} else if take_bool(&mut part, "thought") {
				// Legacy: `thought: true` + `text` = reasoning content
				if let Some(reasoning) = take_string(&mut part, "text") {
					content.push(GeminiChatContent::Reasoning(reasoning));
				}
			}

			// -- Function call
			if let Ok(fc) = part.x_take::<Value>("functionCall") {
				let fn_name: String = fc.x_get("name").unwrap_or_default();
				// Gemini omits call_id; synthesize a unique one to avoid
				// collisions when the same tool is called multiple times.
				let call_id = format!("call#{}#{}", fn_name, tool_call_counter);
				tool_call_counter += 1;
				content.push(GeminiChatContent::ToolCall(ToolCall {
					call_id,
					fn_name,
					fn_arguments: fc.x_get("args").unwrap_or(Value::Null),
					thought_signatures: None,
				}));
			}

			// -- Plain text
			if let Some(text) = take_string(&mut part, "text") {
				content.push(GeminiChatContent::Text(text));
			}

			// -- Capture eventual inlineData (Image)
			if let Ok(inline_data) = part.x_take::<Value>("inlineData") {
				// Note: Gemini may send inline data in multiple parts, but for now, we will treat each part as a separate binary content. We can consider concatenating them if needed in the future.
				if let Ok(mime_type) = inline_data.x_get::<String>("mimeType")
					&& let Ok(data) = inline_data.x_get::<String>("data")
				{
					let binary = Binary::from_base64(mime_type, data, None);
					content.push(GeminiChatContent::Binary(binary));
				}
			}
		}
		let stop_reason: Option<String> = body.x_take("/candidates/0/finishReason").ok();

		Ok(GeminiChatResponse {
			content,
			usage,
			stop_reason,
		})
	}

	/// Builds the Gemini JSON payload from a ChatRequest, including reasoning budget
	/// resolution, system instruction, tools, response format, and chat options.
	/// Returns (payload, provider_model_name) where provider_model_name may differ
	/// from model_name if a reasoning suffix was stripped.
	pub(in crate::adapter) fn build_gemini_request_payload(
		model: &ModelIden,
		model_name: &str,
		chat_req: ChatRequest,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<(Value, String)> {
		// -- Reasoning Budget
		let (provider_model_name, computed_reasoning_effort) = match (model_name, options_set.reasoning_effort()) {
			// No explicit reasoning_effort, try to infer from model name suffix (supports -zero)
			(model, None) => {
				if let Some((prefix, last)) = model_name.rsplit_once('-') {
					let reasoning = match last {
						// 'zero' is a gemini special
						"zero" => Some(ReasoningEffort::Budget(REASONING_ZERO)),
						"none" => Some(ReasoningEffort::None),
						"low" | "minimal" => Some(ReasoningEffort::Low),
						"medium" => Some(ReasoningEffort::Medium),
						"high" => Some(ReasoningEffort::High),
						"xhigh" => Some(ReasoningEffort::XHigh),
						"max" => Some(ReasoningEffort::Max),
						_ => None,
					};
					// strip the reasoning suffix from the model name if one was matched
					let model = if reasoning.is_some() { prefix } else { model };
					(model, reasoning)
				} else {
					(model, None)
				}
			}
			// TOOD: make it more elegant
			(model, Some(effort)) => (model, Some(effort.clone())),
		};

		// -- parts
		let GeminiChatRequestParts {
			system,
			contents,
			tools,
		} = Self::into_gemini_request_parts(model, chat_req)?;

		let mut payload = json!({ "contents": contents });

		// -- Set the reasoning effort
		if let Some(computed_reasoning_effort) = computed_reasoning_effort {
			// -- For gemini-3 use the thinkingLevel if Low or High (does not support medium for now)
			if provider_model_name.contains("gemini-3") {
				match computed_reasoning_effort {
					ReasoningEffort::Low | ReasoningEffort::Minimal => {
						payload.x_insert("/generationConfig/thinkingConfig/thinkingLevel", "LOW")?;
					}
					ReasoningEffort::High | ReasoningEffort::Max => {
						payload.x_insert("/generationConfig/thinkingConfig/thinkingLevel", "HIGH")?;
					}
					// Fallback on thinkingBudget
					other => {
						insert_gemini_thinking_budget_value(&mut payload, &other)?;
					}
				}
			}
			// -- Otherwise, Do thinking budget
			else {
				insert_gemini_thinking_budget_value(&mut payload, &computed_reasoning_effort)?;
			}
		}

		// -- Opt-in for includeThoughts: only request thought content when
		// the caller explicitly asks for reasoning content capture.
		// Thought *signatures* are always returned by the API regardless of this flag.
		if options_set.capture_reasoning_content() == Some(true) {
			payload.x_insert("/generationConfig/thinkingConfig/includeThoughts", true)?;
		}

		// Note: It's unclear from the spec if the content of systemInstruction should have a role.
		//       Right now, it is omitted (since the spec states it can only be "user" or "model")
		//       It seems to work. https://ai.google.dev/api/rest/v1beta/models/generateContent
		if let Some(system) = system {
			payload.x_insert("systemInstruction", json!({ "parts": [{ "text": system }] }))?;
		}

		// -- Tools
		if let Some(tools) = tools {
			payload.x_insert("tools", tools)?;
		}

		// -- Response Format
		if let Some(ChatResponseFormat::JsonSpec(st_json)) = options_set.response_format() {
			payload.x_insert("/generationConfig/responseMimeType", "application/json")?;
			let mut schema = st_json.schema.clone();
			super::openapi_schema::to_openapi_schema(&mut schema);
			payload.x_insert("/generationConfig/responseJsonSchema", schema)?;
		}

		// -- Add supported ChatOptions
		if let Some(temperature) = options_set.temperature() {
			payload.x_insert("/generationConfig/temperature", temperature)?;
		}
		if !options_set.stop_sequences().is_empty() {
			payload.x_insert("/generationConfig/stopSequences", options_set.stop_sequences())?;
		}
		if let Some(max_tokens) = options_set.max_tokens() {
			payload.x_insert("/generationConfig/maxOutputTokens", max_tokens)?;
		}
		if let Some(top_p) = options_set.top_p() {
			payload.x_insert("/generationConfig/topP", top_p)?;
		}

		Ok((payload, provider_model_name.to_string()))
	}

	/// See gemini doc: https://ai.google.dev/api/generate-content#UsageMetadata
	pub(in crate::adapter) fn into_usage(mut usage_value: Value) -> Usage {
		let total_tokens: Option<i32> = usage_value.x_take("totalTokenCount").ok();

		// -- Compute prompt tokens
		let prompt_tokens: Option<i32> = usage_value.x_take("promptTokenCount").ok();
		// Note: https://developers.googleblog.com/en/gemini-2-5-models-now-support-implicit-caching/
		//       It does say `cached_content_token_count`, but in the json, it's probably
		//       `cachedContenTokenCount` (Could not verify for implicit cache, did not see it yet)
		// Note: It seems the promptTokenCount is inclusive of the cachedContentTokenCount
		//       see: https://ai.google.dev/gemini-api/docs/caching?lang=python#generate-content
		//       (this was for explicit caching, but should be the same for implicit)
		//       ```
		//       prompt_token_count: 696219
		//       cached_content_token_count: 696190
		//       candidates_token_count: 214
		//       total_token_count: 696433
		//       ```
		//       So, in short same as Open asi
		let g_cached_tokens: Option<i32> = usage_value.x_take("cachedContentTokenCount").ok();
		let prompt_tokens_details = g_cached_tokens.map(|g_cached_tokens| PromptTokensDetails {
			cache_creation_tokens: None,
			cache_creation_details: None,
			cached_tokens: Some(g_cached_tokens),
			audio_tokens: None,
		});

		// -- Compute completion tokens
		let g_candidate_tokens: Option<i32> = usage_value.x_take("candidatesTokenCount").ok();
		let g_thoughts_tokens: Option<i32> = usage_value.x_take("thoughtsTokenCount").ok();
		// IMPORTANT: For Gemini, the `thoughtsTokenCount` (~reasoning_tokens) is not included
		//            in the root `candidatesTokenCount` (~completion_tokens).
		//            Therefore, some computation is needed to normalize it in the "OpenAI API Way,"
		//            meaning `completion_tokens` represents the total of completion tokens,
		//            and the details provide a breakdown of the specific components.
		let (completion_tokens, completion_tokens_details) = match (g_candidate_tokens, g_thoughts_tokens) {
			(Some(c_tokens), Some(t_tokens)) => (
				Some(c_tokens + t_tokens),
				Some(CompletionTokensDetails {
					accepted_prediction_tokens: None,
					rejected_prediction_tokens: None,
					reasoning_tokens: Some(t_tokens),
					audio_tokens: None,
				}),
			),
			(None, Some(t_tokens)) => {
				(
					None,
					Some(CompletionTokensDetails {
						accepted_prediction_tokens: None,
						rejected_prediction_tokens: None,
						reasoning_tokens: Some(t_tokens), // should be safe enough
						audio_tokens: None,
					}),
				)
			}
			(c_tokens, None) => (c_tokens, None),
		};

		Usage {
			prompt_tokens,
			// for now, None for Gemini
			prompt_tokens_details,

			completion_tokens,

			completion_tokens_details,

			total_tokens,
		}
	}

	/// Takes the genai ChatMessages and builds the System string and JSON Messages for Gemini.
	/// - Role mapping `ChatRole:User -> role: "user"`, `ChatRole::Assistant -> role: "model"`
	/// - `ChatRole::System` is concatenated (with an empty line) into a single `system` for the system instruction.
	///   - This adapter uses version v1beta, which supports `systemInstruction`
	/// - The eventual `chat_req.system` is pushed first into the "systemInstruction"
	pub(in crate::adapter) fn into_gemini_request_parts(
		model_iden: &ModelIden, // use for error reporting
		chat_req: ChatRequest,
	) -> Result<GeminiChatRequestParts> {
		let mut contents: Vec<Value> = Vec::new();
		let mut systems: Vec<String> = Vec::new();

		if let Some(system) = chat_req.system {
			systems.push(system);
		}

		// -- Build
		for msg in chat_req.messages {
			match msg.role {
				// For now, system goes as "user" (later, we might have adapter_config.system_to_user_impl)
				ChatRole::System => {
					if let Some(content) = msg.content.into_joined_texts() {
						systems.push(content);
					}
				}
				ChatRole::User => {
					let mut parts_values: Vec<Value> = Vec::new();
					for part in msg.content {
						match part {
							ContentPart::Text(text) => parts_values.push(json!({"text": text})),
							ContentPart::Binary(binary) => {
								let Binary {
									content_type, source, ..
								} = binary;
								match &source {
									BinarySource::Url(url) => parts_values.push(json!({
										"file_data": {
											"mime_type": content_type,
											"file_uri": url
										}
									})),
									BinarySource::Base64(content) => parts_values.push(json!({
										"inline_data": {
											"mime_type": content_type,
											"data": content
										}
									})),
								}
							}
							ContentPart::ToolCall(tool_call) => {
								parts_values.push(json!({
									"functionCall": {
										"name": tool_call.fn_name,
										"args": tool_call.fn_arguments,
									}
								}));
							}
							ContentPart::ToolResponse(tool_response) => {
								parts_values.push(json!({
									"functionResponse": {
										"name": tool_response.call_id,
										"response": {
											"name": tool_response.call_id,
											"content": tool_response.content,
										}
									}
								}));
							}
							ContentPart::ThoughtSignature(thought) => {
								parts_values.push(json!({
									"thoughtSignature": thought
								}));
							}

							ContentPart::ReasoningContent(_) => {}
							// Custom are ignored for this logic
							ContentPart::Custom(_) => {}
						}
					}

					contents.push(json!({"role": "user", "parts": parts_values}));
				}
				ChatRole::Assistant => {
					let mut parts_values: Vec<Value> = Vec::new();
					let mut pending_thought: Option<String> = None;
					let mut is_first_tool_call = true;

					for part in msg.content {
						match part {
							ContentPart::Text(text) => {
								if let Some(thought) = pending_thought.take() {
									parts_values.push(json!({"thoughtSignature": thought}));
								}
								parts_values.push(json!({"text": text}));
							}
							ContentPart::ToolCall(tool_call) => {
								let mut part_obj = serde_json::Map::new();
								part_obj.insert(
									"functionCall".to_string(),
									json!({
										"name": tool_call.fn_name,
										"args": tool_call.fn_arguments,
									}),
								);

								match pending_thought.take() {
									Some(thought) => {
										// Inject thoughtSignature alongside functionCall in the same Part object
										part_obj.insert("thoughtSignature".to_string(), json!(thought));
									}
									None => {
										// For Gemini 3 models, if there haven't been any thoughts, and this is
										// still the first tool call, we are required to inject a special flag.
										// See: https://ai.google.dev/gemini-api/docs/thought-signatures#faqs
										let is_gemini_3 = model_iden.model_name.contains("gemini-3");
										if is_gemini_3 && is_first_tool_call {
											part_obj.insert(
												"thoughtSignature".to_string(),
												json!("skip_thought_signature_validator"),
											);
										}
									}
								}

								parts_values.push(Value::Object(part_obj));
								is_first_tool_call = false;
							}
							ContentPart::ThoughtSignature(thought) => {
								if let Some(prev_thought) = pending_thought.take() {
									parts_values.push(json!({"thoughtSignature": prev_thought}));
								}
								pending_thought = Some(thought);
							}
							// Ignore unsupported parts for Assistant role
							ContentPart::Binary(_) => {
								if let Some(thought) = pending_thought.take() {
									parts_values.push(json!({"thoughtSignature": thought}));
								}
							}
							ContentPart::ToolResponse(_) => {
								if let Some(thought) = pending_thought.take() {
									parts_values.push(json!({"thoughtSignature": thought}));
								}
							}
							ContentPart::ReasoningContent(_) => {}
							// Custom are ignored for this logic
							ContentPart::Custom(_) => {}
						}
					}
					if let Some(thought) = pending_thought {
						parts_values.push(json!({"thoughtSignature": thought}));
					}
					if !parts_values.is_empty() {
						contents.push(json!({"role": "model", "parts": parts_values}));
					}
				}
				ChatRole::Tool => {
					let mut parts_values: Vec<Value> = Vec::new();
					for part in msg.content {
						match part {
							ContentPart::ToolCall(tool_call) => {
								parts_values.push(json!({
									"functionCall": {
										"name": tool_call.fn_name,
										"args": tool_call.fn_arguments,
									}
								}));
							}
							ContentPart::ToolResponse(tool_response) => {
								parts_values.push(json!({
									"functionResponse": {
										"name": tool_response.call_id,
										"response": {
											"name": tool_response.call_id,
											"content": tool_response.content,
										}
									}
								}));
							}
							ContentPart::ThoughtSignature(thought) => {
								parts_values.push(json!({
									"thoughtSignature": thought
								}));
							}
							ContentPart::ReasoningContent(_) => {}
							_ => {
								return Err(Error::MessageContentTypeNotSupported {
									model_iden: model_iden.clone(),
									cause: "ChatRole::Tool can only contain ToolCall, ToolResponse, or Thought content parts",
								});
							}
						}
					}

					contents.push(json!({"role": "user", "parts": parts_values}));
				}
			}
		}

		let system = if !systems.is_empty() {
			Some(systems.join("\n"))
		} else {
			None
		};

		// -- Post-process: merge consecutive tool-response "user" entries into a single entry.
		// Gemini FC protocol requires all functionResponse parts to be in one "user" turn
		// following the "model" turn with functionCall parts.
		let contents = Self::merge_consecutive_tool_response_entries(contents);

		// -- Build tools
		let tools = if let Some(req_tools) = chat_req.tools {
			let mut tools: Vec<Value> = Vec::new();
			// Note: This is to add only one function_declarations in the tools as per the gemini spec
			//       The rest are builtins
			let mut function_declarations: Vec<Value> = Vec::new();
			for req_tool in req_tools {
				match Self::tool_to_gemini_tool(req_tool)? {
					GeminiTool::Builtin(value) => tools.push(value),
					GeminiTool::User(value) => function_declarations.push(value),
				}
			}
			if !function_declarations.is_empty() {
				tools.push(json!({"functionDeclarations": function_declarations}));
			}
			Some(tools)
		} else {
			None
		};

		Ok(GeminiChatRequestParts {
			system,
			contents,
			tools,
		})
	}

	fn tool_to_gemini_tool(tool: Tool) -> Result<GeminiTool> {
		let Tool {
			name,
			description,
			schema,
			config,
			..
		} = tool;

		// Built-in WebSearch for Gemini
		let name_str = match &name {
			ToolName::WebSearch => "googleSearch",
			ToolName::Custom(name) => name.as_str(),
		};

		// -- if it is a builtin tool
		if matches!(
			name_str,
			"googleSearch" | "googleSearchRetrieval" | "codeExecution" | "urlContext"
		) {
			let config = match config {
				// GoogleSearch does not take any config for now
				Some(ToolConfig::WebSearch(_config)) => Some(json!({})),
				// If custom, user knows better
				Some(ToolConfig::Custom(config)) => Some(config),
				// For now, none is empty
				None => None,
			};
			Ok(GeminiTool::Builtin(json!({ name_str: config })))
		}
		// -- otherwise, user tool
		else {
			let mut parameters = schema.unwrap_or(Value::Null);
			super::openapi_schema::to_openapi_schema(&mut parameters);
			let parameters = if parameters.is_null() { None } else { Some(parameters) };
			Ok(GeminiTool::User(json!({
				"name": name_str,
				"description": description,
				"parameters": parameters,
			})))
		}
	}
}

impl GeminiAdapter {
	/// Merge consecutive "user" entries that contain only functionResponse parts
	/// into a single entry. Gemini requires all function responses in one turn.
	fn merge_consecutive_tool_response_entries(contents: Vec<Value>) -> Vec<Value> {
		fn is_tool_response_entry(entry: &Value) -> bool {
			if entry.get("role").and_then(|r| r.as_str()) != Some("user") {
				return false;
			}
			if let Some(parts) = entry.get("parts").and_then(|p| p.as_array()) {
				!parts.is_empty() && parts.iter().all(|p| p.get("functionResponse").is_some())
			} else {
				false
			}
		}

		let mut result: Vec<Value> = Vec::with_capacity(contents.len());
		for entry in contents {
			if is_tool_response_entry(&entry) {
				// Check if previous entry is also a tool response — merge
				if let Some(prev) = result.last_mut()
					&& is_tool_response_entry(prev)
					&& let (Some(prev_parts), Some(new_parts)) = (
						prev.get_mut("parts").and_then(|p| p.as_array_mut()),
						entry.get("parts").and_then(|p| p.as_array()),
					) {
					prev_parts.extend(new_parts.iter().cloned());
					continue;
				}
			}
			result.push(entry);
		}
		result
	}
}

pub enum GeminiTool {
	Builtin(Value),
	User(Value),
}

/// FIXME: need to be Vec<GeminiChatContent>
pub(in crate::adapter) struct GeminiChatResponse {
	pub content: Vec<GeminiChatContent>,
	pub usage: Usage,
	pub stop_reason: Option<String>,
}

pub(in crate::adapter) enum GeminiChatContent {
	Text(String),
	Binary(Binary),
	ToolCall(ToolCall),
	Reasoning(String),
	ThoughtSignature(String),
}

pub(in crate::adapter) struct GeminiChatRequestParts {
	pub system: Option<String>,
	/// The chat history (user and assistant, except for the last user message which is a message)
	pub contents: Vec<Value>,

	/// The tools to use
	pub tools: Option<Vec<Value>>,
}

// region:    --- Helpers

/// Extract and remove a string field from a JSON Value.
fn take_string(v: &mut Value, key: &str) -> Option<String> {
	v.as_object_mut()
		.and_then(|m| m.remove(key))
		.and_then(|v| if let Value::String(s) = v { Some(s) } else { None })
}

/// Extract and remove a boolean field from a JSON Value, defaulting to false.
fn take_bool(v: &mut Value, key: &str) -> bool {
	v.as_object_mut()
		.and_then(|m| m.remove(key))
		.and_then(|v| v.as_bool())
		.unwrap_or(false)
}

// endregion: --- Helpers

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn merge_consecutive_tool_responses() {
		let contents = vec![
			json!({"role": "model", "parts": [{"functionCall": {"name": "read", "args": {}}}]}),
			json!({"role": "user", "parts": [{"functionResponse": {"name": "call1", "response": {"content": "a"}}}]}),
			json!({"role": "user", "parts": [{"functionResponse": {"name": "call2", "response": {"content": "b"}}}]}),
		];
		let merged = GeminiAdapter::merge_consecutive_tool_response_entries(contents);
		assert_eq!(merged.len(), 2); // model + single merged user
		let parts = merged[1].get("parts").unwrap().as_array().unwrap();
		assert_eq!(parts.len(), 2); // both functionResponse in one entry
	}

	#[test]
	fn merge_does_not_merge_non_tool_user_entries() {
		let contents = vec![
			json!({"role": "user", "parts": [{"text": "hello"}]}),
			json!({"role": "user", "parts": [{"functionResponse": {"name": "c1", "response": {"content": "x"}}}]}),
		];
		let merged = GeminiAdapter::merge_consecutive_tool_response_entries(contents);
		assert_eq!(merged.len(), 2); // not merged — first is text, not tool response
	}

	#[test]
	fn merge_three_consecutive_tool_responses() {
		let contents = vec![
			json!({"role": "user", "parts": [{"functionResponse": {"name": "c1", "response": {"content": "a"}}}]}),
			json!({"role": "user", "parts": [{"functionResponse": {"name": "c2", "response": {"content": "b"}}}]}),
			json!({"role": "user", "parts": [{"functionResponse": {"name": "c3", "response": {"content": "c"}}}]}),
		];
		let merged = GeminiAdapter::merge_consecutive_tool_response_entries(contents);
		assert_eq!(merged.len(), 1);
		let parts = merged[0].get("parts").unwrap().as_array().unwrap();
		assert_eq!(parts.len(), 3);
	}

	#[test]
	fn tool_call_id_uses_counter() {
		// Simulate Gemini response with two functionCalls for the same tool
		let body = json!({
			"candidates": [{
				"content": {
					"role": "model",
					"parts": [
						{"functionCall": {"name": "read_file", "args": {"path": "a.rs"}}},
						{"functionCall": {"name": "read_file", "args": {"path": "b.rs"}}}
					]
				}
			}],
			"usageMetadata": {"totalTokenCount": 100}
		});
		let model_iden = ModelIden::new(AdapterKind::Gemini, "gemini-test");
		let response = GeminiAdapter::body_to_gemini_chat_response(&model_iden, body).unwrap();
		let tool_calls: Vec<_> = response
			.content
			.into_iter()
			.filter_map(|c| {
				if let GeminiChatContent::ToolCall(tc) = c {
					Some(tc)
				} else {
					None
				}
			})
			.collect();
		assert_eq!(tool_calls.len(), 2);
		assert_ne!(tool_calls[0].call_id, tool_calls[1].call_id);
		assert!(tool_calls[0].call_id.contains("read_file"));
		assert!(tool_calls[1].call_id.contains("read_file"));
	}

	#[test]
	fn body_to_gemini_chat_response_accepts_usage_only_stream_tail() {
		let model_iden = ModelIden::new(AdapterKind::Gemini, "gemini-2.5-flash");
		let response = GeminiAdapter::body_to_gemini_chat_response(
			&model_iden,
			json!({
				"candidates": [
					{
						"finishReason": "STOP"
					}
				],
				"usageMetadata": {
					"promptTokenCount": 10,
					"candidatesTokenCount": 4,
					"totalTokenCount": 14
				}
			}),
		)
		.expect("usage-only stream tail should not be treated as an error");

		assert!(response.content.is_empty());
		assert_eq!(response.usage.total_tokens, Some(14));
		assert_eq!(response.usage.prompt_tokens, Some(10));
		assert_eq!(response.usage.completion_tokens, Some(4));
		assert_eq!(response.stop_reason.as_deref(), Some("STOP"));
	}

	#[test]
	fn body_to_gemini_chat_response_accepts_tail_with_null_finish_reason() {
		let model_iden = ModelIden::new(AdapterKind::Gemini, "gemini-2.5-flash");
		let response = GeminiAdapter::body_to_gemini_chat_response(
			&model_iden,
			json!({
				"candidates": [
					{
						"finishReason": null
					}
				],
				"usageMetadata": {
					"promptTokenCount": 10,
					"candidatesTokenCount": 4,
					"totalTokenCount": 14
				}
			}),
		)
		.expect("usage-only stream tail with null finishReason should not be an error");

		assert!(response.content.is_empty());
		assert_eq!(response.usage.total_tokens, Some(14));
	}

	#[test]
	fn body_to_gemini_chat_response_still_rejects_missing_candidates() {
		let model_iden = ModelIden::new(AdapterKind::Gemini, "gemini-2.5-flash");
		let result = GeminiAdapter::body_to_gemini_chat_response(
			&model_iden,
			json!({
				"usageMetadata": {
					"promptTokenCount": 10,
					"candidatesTokenCount": 4,
					"totalTokenCount": 14
				}
			}),
		);

		let err = match result {
			Err(e) => e,
			Ok(_) => panic!("missing candidates should still be rejected"),
		};
		assert!(matches!(err, Error::ChatResponse { .. }));
	}
}

use crate::adapter::adapters::support::get_api_key;
use crate::adapter::cohere::CohereStreamer;
use crate::adapter::{Adapter, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{
	ChatOptionsSet, ChatRequest, ChatResponse, ChatRole, ChatStream, ChatStreamResponse, MessageContent, StopReason,
	Usage,
};
use crate::resolver::{AuthData, Endpoint};
use crate::webc::{WebResponse, WebStream};
use crate::{Error, Headers, Result};
use crate::{ModelIden, ServiceTarget};
use reqwest::RequestBuilder;
use serde_json::{Value, json};
use value_ext::JsonValueExt;

pub struct CohereAdapter;

const MODELS: &[&str] = &[
	"command-r-plus",
	"command-r",
	"command",
	"command-nightly",
	"command-light",
	"command-light-nightly",
];

impl CohereAdapter {
	pub const API_KEY_DEFAULT_ENV_NAME: &str = "COHERE_API_KEY";
}

impl Adapter for CohereAdapter {
	const DEFAULT_API_KEY_ENV_NAME: Option<&'static str> = Some(Self::API_KEY_DEFAULT_ENV_NAME);

	fn default_endpoint() -> Endpoint {
		const BASE_URL: &str = "https://api.cohere.com/v1/";
		Endpoint::from_static(BASE_URL)
	}

	fn default_auth() -> AuthData {
		match Self::DEFAULT_API_KEY_ENV_NAME {
			Some(env_name) => AuthData::from_env(env_name),
			None => AuthData::None,
		}
	}

	/// Note: For now, it returns the common ones (see above)
	async fn all_model_names(_kind: AdapterKind, _endpoint: Endpoint, _auth: AuthData) -> Result<Vec<String>> {
		Ok(MODELS.iter().map(|s| s.to_string()).collect())
	}

	fn get_service_url(_model: &ModelIden, service_type: ServiceType, endpoint: Endpoint) -> Result<String> {
		let base_url = endpoint.base_url();
		let url = match service_type {
			ServiceType::Chat | ServiceType::ChatStream => format!("{base_url}chat"),
			ServiceType::Embed => {
				//HACK: Cohere embeddings use v2 API, but base_url is v1, so we need to replace it
				let base_without_version = base_url.trim_end_matches("v1/");
				format!("{base_without_version}v2/embed")
			}
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

		// -- api_key (this Adapter requires it)
		let api_key = get_api_key(auth, &model)?;

		// -- url
		let url = Self::get_service_url(&model, service_type, endpoint)?;

		// -- headers
		let headers = Headers::from(("Authorization".to_string(), format!("Bearer {api_key}")));

		// -- parts
		let CohereChatRequestParts {
			preamble,
			message,
			chat_history,
		} = Self::into_cohere_request_parts(model.clone(), chat_req)?;

		// -- Build the basic payload
		let (_, model_name) = model.model_name.namespace_and_name();
		let stream = matches!(service_type, ServiceType::ChatStream);
		let mut payload = json!({
			"model": model_name.to_string(),
			"message": message,
			"stream": stream
		});

		if !chat_history.is_empty() {
			payload.x_insert("chat_history", chat_history)?;
		}
		if let Some(preamble) = preamble {
			payload.x_insert("preamble", preamble)?;
		}

		// -- Add supported ChatOptions
		if let Some(temperature) = options_set.temperature() {
			payload.x_insert("temperature", temperature)?;
		}

		if !options_set.stop_sequences().is_empty() {
			payload.x_insert("stop_sequences", options_set.stop_sequences())?;
		}

		if let Some(max_tokens) = options_set.max_tokens() {
			payload.x_insert("max_tokens", max_tokens)?;
		}

		if let Some(top_p) = options_set.top_p() {
			payload.x_insert("p", top_p)?;
		}

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
		// let provider_model_name: Option<String> = body.x_remove("model").ok();
		let provider_model_name = None;
		let provider_model_iden = model_iden.from_optional_name(provider_model_name);

		// -- Get stop_reason
		let stop_reason = body
			.x_take::<Option<String>>("finish_reason")
			.ok()
			.flatten()
			.map(StopReason::from);

		// -- Get usage
		let usage = body.x_take("/meta/tokens").map(Self::into_usage).unwrap_or_default();

		// -- Get response
		let Some(mut last_chat_history_item) = body.x_take::<Vec<Value>>("chat_history")?.pop() else {
			return Err(Error::NoChatResponse { model_iden });
		};

		let content: MessageContent = last_chat_history_item
			.x_take::<Option<String>>("message")?
			.map(MessageContent::from)
			.unwrap_or_default();

		Ok(ChatResponse {
			content,
			reasoning_content: None,
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
		let web_stream = WebStream::new_with_delimiter(reqwest_builder, "\n");
		let cohere_stream = CohereStreamer::new(web_stream, model_iden.clone(), options_set);
		let chat_stream = ChatStream::from_inter_stream(cohere_stream);

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

/// Support function
impl CohereAdapter {
	/// Convert usage from '/meta/tokens'
	/// ```json
	///  "tokens": {
	///    "input_tokens": 20,
	///    "output_tokens": 24
	///  }
	/// ```
	pub(super) fn into_usage(mut usage_value: Value) -> Usage {
		let prompt_tokens: Option<i32> = usage_value.x_take("input_tokens").ok();
		let completion_tokens: Option<i32> = usage_value.x_take("output_tokens").ok();

		// Compute total tokens
		let total_tokens = if prompt_tokens.is_some() || completion_tokens.is_some() {
			Some(prompt_tokens.unwrap_or(0) + completion_tokens.unwrap_or(0))
		} else {
			None
		};

		#[allow(deprecated)]
		Usage {
			prompt_tokens,
			// for now, None for Cohere
			prompt_tokens_details: None,

			completion_tokens,
			// for now, None for Cohere
			completion_tokens_details: None,

			total_tokens,
		}
	}

	/// Takes the GenAI ChatMessages and builds the system string and JSON messages for Cohere.
	/// - Pops the last chat user message and sets it as the message
	/// - Sets any eventual `system` as the first `preamble`
	/// - Adds all of the system messages into the 'preamble' (this might change when ChatReq has a `.system`)
	/// - Builds the chat history with the remaining messages
	fn into_cohere_request_parts(
		model_iden: ModelIden, // for error only
		mut chat_req: ChatRequest,
	) -> Result<CohereChatRequestParts> {
		let mut chat_history: Vec<Value> = Vec::new();
		let mut systems: Vec<String> = Vec::new();

		// -- Add the eventual system as preamble
		if let Some(system) = chat_req.system {
			systems.push(system);
		}

		// -- Build and extract the last user message
		let last_chat_msg = chat_req.messages.pop().ok_or_else(|| Error::ChatReqHasNoMessages {
			model_iden: model_iden.clone(),
		})?;
		if !matches!(last_chat_msg.role, ChatRole::User) {
			return Err(Error::LastChatMessageIsNotUser {
				model_iden,
				actual_role: last_chat_msg.role,
			});
		}

		// TODO: Needs to implement tool_calls
		let Some(message) = last_chat_msg.content.into_joined_texts() else {
			return Err(Error::MessageContentTypeNotSupported {
				model_iden,
				cause: "Only MessageContent::Text supported for this model (for now)",
			});
		};

		// -- Build
		for msg in chat_req.messages {
			let Some(content) = msg.content.into_joined_texts() else {
				return Err(Error::MessageContentTypeNotSupported {
					model_iden,
					cause: "Only MessageContent::Text supported for this model (for now)",
				});
			};

			match msg.role {
				// For now, system and tool messages go to the system
				ChatRole::System => systems.push(content),
				ChatRole::User => chat_history.push(json! ({"role": "USER", "content": content})),
				ChatRole::Assistant => chat_history.push(json! ({"role": "CHATBOT", "content": content})),
				ChatRole::Tool => {
					return Err(Error::MessageRoleNotSupported {
						model_iden,
						role: ChatRole::Tool,
					});
				}
			}
		}

		// -- Build the preamble
		// Note: For now, we just concatenate the system messages into the preamble as recommended by Cohere
		//       Later, the ChatRequest should have a `.system` property
		let preamble = if !systems.is_empty() {
			Some(systems.join("\n"))
		} else {
			None
		};

		Ok(CohereChatRequestParts {
			preamble,
			message,
			chat_history,
		})
	}
}

struct CohereChatRequestParts {
	/// The "system" in the Cohere context
	preamble: Option<String>,
	/// The last user message
	message: String,
	/// The chat history (user and assistant, except the last user message which is the message)
	chat_history: Vec<Value>,
}

// endregion: --- Support

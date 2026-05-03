// region:    --- AliyunAdapter

use crate::adapter::openai::OpenAIAdapter;
use crate::adapter::{Adapter, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{ChatOptionsSet, ChatRequest, ChatResponse, ChatStreamResponse};
use crate::resolver::{AuthData, Endpoint};
use crate::webc::WebResponse;
use crate::{ModelIden, Result, ServiceTarget};
use reqwest::RequestBuilder;

/// Aliyun Adapter - Uses OpenAI-compatible API for Dashscope (Aliyun)
///
/// Aliyun Dashscope API provides OpenAI-compatible endpoints for chat, streaming, and embedding.
/// This adapter delegates most of the implementation to OpenAIAdapter utilities.
#[derive(Debug)]
pub struct AliyunAdapter;
// All Aliyun supported models
// NOTE: These are sourced from the official Aliyun Dashscope documentation
pub const MODELS: &[&str] = &[
	// Chat models
	"qwen-turbo",
	"qwen-plus",
	"qwen-max",
	"qwen-max-longcontext",
	"qwen-turbo-latest",
	"qwen-plus-latest",
	"qwen-max-latest",
	// Vision-language models
	"qwen-vl-plus",
	"qwen-vl-max",
	"qwen-vl-plus-latest",
	// Open source models
	"qwen-7b-chat",
	"qwen-14b-chat",
	"qwen-72b-chat",
	"qwen-72b-chat-int4",
	// Math models
	"qwen-math-plus",
	"qwen-math-turbo",
	"qwen-math-plus-latest",
	"qwen-math-turbo-latest",
	// Audio models
	"qwen-audio-turbo",
	"qwen-audio-plus",
	"qwen-audio-chat-v1",
	// Code models
	"qwen-coder-plus",
	"qwen-coder-turbo",
	"qwen-coder-latest",
];
impl AliyunAdapter {
	pub const API_KEY_DEFAULT_ENV_NAME: &str = "ALIYUN_API_KEY";
}

impl Adapter for AliyunAdapter {
	const DEFAULT_API_KEY_ENV_NAME: Option<&'static str> = Some(Self::API_KEY_DEFAULT_ENV_NAME);

	/// Returns the default endpoint for Aliyun Dashscope API
	fn default_endpoint() -> Endpoint {
		const BASE_URL: &str = "https://dashscope.aliyuncs.com/compatible-mode/v1/";
		Endpoint::from_static(BASE_URL)
	}

	/// Returns authentication data with API key prefix AILIYUN and environment variable ALIYUN_API_KEY
	fn default_auth() -> AuthData {
		match Self::DEFAULT_API_KEY_ENV_NAME {
			Some(env_name) => AuthData::from_env(env_name),
			None => AuthData::None,
		}
	}

	/// Returns all supported model names for Aliyun
	async fn all_model_names(_kind: AdapterKind, _endpoint: Endpoint, _auth: AuthData) -> Result<Vec<String>> {
		Ok(MODELS.iter().map(|s| s.to_string()).collect())
	}

	/// Returns the service URL for the given model
	///
	/// Since Aliyun Dashscope API is OpenAI-compatible, we use the OpenAI URL pattern.
	fn get_service_url(_model: &ModelIden, service_type: ServiceType, endpoint: Endpoint) -> Result<String> {
		let base_url = endpoint.base_url();

		let url = match service_type {
			ServiceType::Chat | ServiceType::ChatStream => format!("{base_url}chat/completions"),
			ServiceType::Embed => format!("{base_url}embeddings"),
		};
		Ok(url)
	}

	/// Converts chat request data to web request format
	///
	/// Delegates to OpenAIAdapter utilities due to API compatibility.
	fn to_web_request_data(
		target: ServiceTarget,
		service_type: ServiceType,
		chat_req: ChatRequest,
		chat_options: ChatOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		// Parse model name and determine appropriate endpoint
		OpenAIAdapter::util_to_web_request_data(target, service_type, chat_req, chat_options, None)
	}

	/// Converts response bytes to ChatResponse
	///
	/// Delegates to OpenAIAdapter due to API compatibility.
	fn to_chat_response(
		model_iden: ModelIden,
		web_response: WebResponse,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<ChatResponse> {
		OpenAIAdapter::to_chat_response(model_iden, web_response, options_set)
	}

	/// Converts response stream to ChatStream
	///
	/// Delegates to OpenAIAdapter due to API compatibility.
	fn to_chat_stream(
		model_iden: ModelIden,
		reqwest_builder: RequestBuilder,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<ChatStreamResponse> {
		OpenAIAdapter::to_chat_stream(model_iden, reqwest_builder, options_set)
	}

	/// Converts embedding request data to web request format
	///
	/// Delegates to OpenAIAdapter utilities due to API compatibility.
	fn to_embed_request_data(
		service_target: crate::ServiceTarget,
		embed_req: crate::embed::EmbedRequest,
		options_set: crate::embed::EmbedOptionsSet<'_, '_>,
	) -> Result<crate::adapter::WebRequestData> {
		OpenAIAdapter::to_embed_request_data(service_target, embed_req, options_set)
	}

	/// Converts response bytes to EmbedResponse
	///
	/// Delegates to OpenAIAdapter due to API compatibility.
	fn to_embed_response(
		model_iden: crate::ModelIden,
		web_response: crate::webc::WebResponse,
		options_set: crate::embed::EmbedOptionsSet<'_, '_>,
	) -> Result<crate::embed::EmbedResponse> {
		OpenAIAdapter::to_embed_response(model_iden, web_response, options_set)
	}
}

// endregion: --- AliyunAdapter

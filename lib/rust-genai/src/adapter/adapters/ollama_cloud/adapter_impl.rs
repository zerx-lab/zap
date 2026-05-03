use crate::Headers;
use crate::adapter::adapters::ollama::OllamaAdapter;
use crate::adapter::adapters::ollama::OllamaRequestParts;
use crate::adapter::adapters::support::get_api_key;
use crate::adapter::{Adapter, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{ChatOptionsSet, ChatRequest, ChatResponse, ChatStreamResponse};
use crate::embed::{EmbedOptionsSet, EmbedRequest, EmbedResponse};
use crate::resolver::{AuthData, Endpoint};
use crate::webc::WebResponse;
use crate::{ModelIden, Result, ServiceTarget};
use reqwest::RequestBuilder;
use serde_json::json;
use value_ext::JsonValueExt;

pub struct OllamaCloudAdapter;

impl OllamaCloudAdapter {
	pub const API_KEY_DEFAULT_ENV_NAME: &str = "OLLAMA_API_KEY";
}

impl Adapter for OllamaCloudAdapter {
	const DEFAULT_API_KEY_ENV_NAME: Option<&'static str> = Some(Self::API_KEY_DEFAULT_ENV_NAME);

	fn default_endpoint() -> Endpoint {
		const BASE_URL: &str = "https://ollama.com/";
		Endpoint::from_static(BASE_URL)
	}

	fn default_auth() -> AuthData {
		AuthData::from_env(Self::API_KEY_DEFAULT_ENV_NAME)
	}

	async fn all_model_names(adapter_kind: AdapterKind, endpoint: Endpoint, auth: AuthData) -> Result<Vec<String>> {
		let api_key = get_api_key(auth, &ModelIden::new(adapter_kind, ""))?;
		let headers = Headers::from(vec![("Authorization", format!("Bearer {api_key}"))]);
		OllamaAdapter::list_model_names(adapter_kind, endpoint, headers).await
	}

	fn get_service_url(model: &ModelIden, service_type: ServiceType, endpoint: Endpoint) -> Result<String> {
		OllamaAdapter::get_service_url(model, service_type, endpoint)
	}

	fn to_web_request_data(
		target: ServiceTarget,
		service_type: ServiceType,
		chat_req: ChatRequest,
		chat_options: ChatOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		let ServiceTarget {
			model, endpoint, auth, ..
		} = target;
		let api_key = get_api_key(auth, &model)?;
		let url = OllamaAdapter::get_service_url(&model, service_type, endpoint)?;
		let OllamaRequestParts { messages, tools } = OllamaAdapter::into_ollama_request_parts(chat_req)?;

		let mut options = json!({});
		if let Some(temperature) = chat_options.temperature() {
			options.x_insert("temperature", temperature)?;
		}
		if let Some(top_p) = chat_options.top_p() {
			options.x_insert("top_p", top_p)?;
		}
		if let Some(max_tokens) = chat_options.max_tokens() {
			options.x_insert("num_predict", max_tokens)?;
		}
		if let Some(seed) = chat_options.seed() {
			options.x_insert("seed", seed)?;
		}
		if !chat_options.stop_sequences().is_empty() {
			options.x_insert("stop", chat_options.stop_sequences())?;
		}

		let stream = matches!(service_type, ServiceType::ChatStream);
		let (_, model_name) = model.model_name.namespace_and_name();

		let mut payload = json!({
			"model": model_name,
			"messages": messages,
			"stream": stream,
		});

		if !options.as_object().unwrap().is_empty() {
			payload.x_insert("options", options)?;
		}

		if let Some(tools) = tools {
			payload.x_insert("tools", tools)?;
		}

		if let Some(format) = chat_options.response_format()
			&& matches!(format, crate::chat::ChatResponseFormat::JsonMode)
		{
			payload.x_insert("format", "json")?;
		}

		let mut headers = Headers::default();
		if let Some(extra_headers) = chat_options.extra_headers() {
			headers.merge_with(extra_headers);
		}
		headers.merge(Headers::from(("Authorization", format!("Bearer {api_key}"))));

		Ok(WebRequestData { url, headers, payload })
	}

	fn to_chat_response(
		model_iden: ModelIden,
		web_response: WebResponse,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<ChatResponse> {
		OllamaAdapter::to_chat_response(model_iden, web_response, options_set)
	}

	fn to_chat_stream(
		model_iden: ModelIden,
		reqwest_builder: RequestBuilder,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<ChatStreamResponse> {
		OllamaAdapter::to_chat_stream(model_iden, reqwest_builder, options_set)
	}

	fn to_embed_request_data(
		service_target: ServiceTarget,
		embed_req: EmbedRequest,
		options_set: EmbedOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		let ServiceTarget {
			model, endpoint, auth, ..
		} = service_target;
		let api_key = get_api_key(auth, &model)?;
		let url = OllamaAdapter::get_service_url(&model, ServiceType::Embed, endpoint)?;
		let (_, model_name) = model.model_name.namespace_and_name();

		let mut payload = json!({
			"model": model_name,
			"input": embed_req.inputs(),
		});

		if let Some(dimensions) = options_set.dimensions() {
			payload.x_insert("dimensions", dimensions)?;
		}
		if let Some(truncate) = options_set.truncate() {
			payload.x_insert("truncate", truncate)?;
		}

		let mut headers = Headers::default();
		if let Some(extra_headers) = options_set.headers() {
			headers.merge_with(extra_headers);
		}
		headers.merge(Headers::from(("Authorization", format!("Bearer {api_key}"))));

		Ok(WebRequestData { url, headers, payload })
	}

	fn to_embed_response(
		model_iden: ModelIden,
		web_response: WebResponse,
		options_set: EmbedOptionsSet<'_, '_>,
	) -> Result<EmbedResponse> {
		OllamaAdapter::to_embed_response(model_iden, web_response, options_set)
	}
}

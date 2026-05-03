use crate::adapter::adapters::support::get_api_key;
use crate::adapter::anthropic::{AnthropicAdapter, AnthropicRequestParts};
use crate::adapter::gemini::GeminiAdapter;
use crate::adapter::{Adapter, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{ChatOptionsSet, ChatRequest, ChatResponse, ChatStreamResponse};
use crate::resolver::{AuthData, Endpoint};
use crate::webc::WebResponse;
use crate::{Error, Headers, ModelIden, Result, ServiceTarget};
use reqwest::RequestBuilder;
use serde_json::json;
use tracing::warn;
use value_ext::JsonValueExt;

pub struct VertexAdapter;

const VERTEX_ANTHROPIC_VERSION: &str = "vertex-2023-10-16";

impl VertexAdapter {
	pub const API_KEY_DEFAULT_ENV_NAME: &str = "VERTEX_API_KEY";
}

// region:    --- VertexPublisher

/// Internal enum to dispatch wire format based on the model's publisher.
enum VertexPublisher {
	Google,
	Anthropic,
}

impl VertexPublisher {
	fn from_model_name(model_name: &str) -> Result<Self> {
		if model_name.starts_with("gemini") {
			Ok(Self::Google)
		} else if model_name.starts_with("claude") {
			Ok(Self::Anthropic)
		} else {
			Err(Error::AdapterNotSupported {
				adapter_kind: AdapterKind::Vertex,
				feature: format!("model '{model_name}' (unknown Vertex AI publisher)"),
			})
		}
	}

	fn publisher_path(&self) -> &'static str {
		match self {
			Self::Google => "publishers/google",
			Self::Anthropic => "publishers/anthropic",
		}
	}
}

// endregion: --- VertexPublisher

impl Adapter for VertexAdapter {
	const DEFAULT_API_KEY_ENV_NAME: Option<&'static str> = Some(Self::API_KEY_DEFAULT_ENV_NAME);

	fn default_endpoint() -> Endpoint {
		let project_id = std::env::var("VERTEX_PROJECT_ID").unwrap_or_else(|_| {
			warn!("VERTEX_PROJECT_ID env var is not set; Vertex AI requests will use a malformed URL");
			String::new()
		});
		// Model availability varies by region. See https://docs.cloud.google.com/vertex-ai/generative-ai/docs/learn/locations for details.
		let base_url = match std::env::var("VERTEX_LOCATION") {
			Ok(location) => {
				format!("https://{location}-aiplatform.googleapis.com/v1/projects/{project_id}/locations/{location}/")
			}
			// When no location is set, fall back to "global"
			Err(_) => format!("https://aiplatform.googleapis.com/v1/projects/{project_id}/locations/global/"),
		};
		Endpoint::from_owned(base_url)
	}
	fn default_auth() -> AuthData {
		match Self::DEFAULT_API_KEY_ENV_NAME {
			Some(env_name) => AuthData::from_env(env_name),
			None => AuthData::None,
		}
	}

	async fn all_model_names(_kind: AdapterKind, _endpoint: Endpoint, _auth: AuthData) -> Result<Vec<String>> {
		Ok(vec![
			"gemini-2.5-pro".to_string(),
			"gemini-2.5-flash".to_string(),
			"gemini-2.5-flash-lite".to_string(),
			"claude-sonnet-4-6".to_string(),
			"claude-opus-4-6".to_string(),
			"claude-haiku-4-5".to_string(),
		])
	}

	/// Note: An unrecognized model prefix falls back to `VertexPublisher::Google` with a warning.
	/// In practice, `to_web_request_data` validates the publisher first and will return
	/// an error before this fallback is ever reached.
	fn get_service_url(model: &ModelIden, service_type: ServiceType, endpoint: Endpoint) -> Result<String> {
		let base_url = endpoint.base_url();
		let (_, model_name) = model.model_name.namespace_and_name();
		let publisher = VertexPublisher::from_model_name(model_name).unwrap_or_else(|_| {
			warn!("Unknown Vertex AI publisher for model '{model_name}'; falling back to Google publisher");
			VertexPublisher::Google
		});
		let publisher_path = publisher.publisher_path();

		let url = match publisher {
			VertexPublisher::Google => match service_type {
				ServiceType::Chat => format!("{base_url}{publisher_path}/models/{model_name}:generateContent"),
				ServiceType::ChatStream => {
					format!("{base_url}{publisher_path}/models/{model_name}:streamGenerateContent")
				}
				ServiceType::Embed => format!("{base_url}{publisher_path}/models/{model_name}:predict"),
			},
			VertexPublisher::Anthropic => match service_type {
				ServiceType::Chat | ServiceType::ChatStream => {
					format!("{base_url}{publisher_path}/models/{model_name}:rawPredict")
				}
				ServiceType::Embed => format!("{base_url}{publisher_path}/models/{model_name}:predict"),
			},
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
		let publisher = VertexPublisher::from_model_name(model_name)?;
		let model_name = model_name.to_string();

		// For Vertex AI the "api key" is an OAuth2 Bearer token supplied by the AuthResolver
		let api_key = get_api_key(auth, &model)?;
		let headers = Headers::from(("Authorization".to_string(), format!("Bearer {api_key}")));

		match publisher {
			VertexPublisher::Google => Self::to_gemini_web_request_data(
				model,
				&model_name,
				endpoint,
				headers,
				service_type,
				chat_req,
				options_set,
			),
			VertexPublisher::Anthropic => Self::to_anthropic_web_request_data(
				model,
				&model_name,
				endpoint,
				headers,
				service_type,
				chat_req,
				options_set,
			),
		}
	}

	fn to_chat_response(
		model_iden: ModelIden,
		web_response: WebResponse,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<ChatResponse> {
		let (_, model_name) = model_iden.model_name.namespace_and_name();
		let publisher = VertexPublisher::from_model_name(model_name)?;

		match publisher {
			VertexPublisher::Google => GeminiAdapter::to_chat_response(model_iden, web_response, options_set),
			VertexPublisher::Anthropic => AnthropicAdapter::to_chat_response(model_iden, web_response, options_set),
		}
	}

	fn to_chat_stream(
		model_iden: ModelIden,
		reqwest_builder: RequestBuilder,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<ChatStreamResponse> {
		let (_, model_name) = model_iden.model_name.namespace_and_name();
		let publisher = VertexPublisher::from_model_name(model_name)?;

		match publisher {
			VertexPublisher::Google => GeminiAdapter::to_chat_stream(model_iden, reqwest_builder, options_set),
			VertexPublisher::Anthropic => AnthropicAdapter::to_chat_stream(model_iden, reqwest_builder, options_set),
		}
	}

	fn to_embed_request_data(
		_service_target: ServiceTarget,
		_embed_req: crate::embed::EmbedRequest,
		_options_set: crate::embed::EmbedOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		Err(Error::AdapterNotSupported {
			adapter_kind: AdapterKind::Vertex,
			feature: "embeddings".to_string(),
		})
	}

	fn to_embed_response(
		_model_iden: ModelIden,
		_web_response: WebResponse,
		_options_set: crate::embed::EmbedOptionsSet<'_, '_>,
	) -> Result<crate::embed::EmbedResponse> {
		Err(Error::AdapterNotSupported {
			adapter_kind: AdapterKind::Vertex,
			feature: "embeddings".to_string(),
		})
	}
}

// region:    --- Gemini Publisher Support

impl VertexAdapter {
	fn to_gemini_web_request_data(
		model: ModelIden,
		model_name: &str,
		endpoint: Endpoint,
		headers: Headers,
		service_type: ServiceType,
		chat_req: ChatRequest,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		let (payload, provider_model_name) =
			GeminiAdapter::build_gemini_request_payload(&model, model_name, chat_req, options_set)?;

		let provider_model = model.from_name(&provider_model_name);
		let url = Self::get_service_url(&provider_model, service_type, endpoint)?;

		Ok(WebRequestData { url, headers, payload })
	}
}

// endregion: --- Gemini Publisher Support

// region:    --- Anthropic Publisher Support

impl VertexAdapter {
	fn to_anthropic_web_request_data(
		model: ModelIden,
		model_name: &str,
		endpoint: Endpoint,
		headers: Headers,
		service_type: ServiceType,
		chat_req: ChatRequest,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		let AnthropicRequestParts {
			system,
			messages,
			tools,
		} = AnthropicAdapter::into_anthropic_request_parts(chat_req)?;

		// Vertex Anthropic: model is in URL, not body; anthropic_version goes in body
		let stream = matches!(service_type, ServiceType::ChatStream);
		let mut payload = json!({
			"anthropic_version": VERTEX_ANTHROPIC_VERSION,
			"messages": messages,
			"stream": stream,
		});

		if let Some(system) = system {
			payload.x_insert("system", system)?;
		}

		if let Some(tools) = tools {
			payload.x_insert("/tools", tools)?;
		}

		if let Some(temperature) = options_set.temperature() {
			payload.x_insert("temperature", temperature)?;
		}

		if !options_set.stop_sequences().is_empty() {
			payload.x_insert("stop_sequences", options_set.stop_sequences())?;
		}

		let max_tokens = AnthropicAdapter::resolve_max_tokens(model_name, &options_set);
		payload.x_insert("max_tokens", max_tokens)?; // required for Anthropic

		if let Some(top_p) = options_set.top_p() {
			payload.x_insert("top_p", top_p)?;
		}

		let url = Self::get_service_url(&model, service_type, endpoint)?;

		Ok(WebRequestData { url, headers, payload })
	}
}

// endregion: --- Anthropic Publisher Support

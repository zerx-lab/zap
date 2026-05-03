use crate::ModelIden;
use crate::adapter::openai::OpenAIAdapter;
use crate::adapter::{Adapter, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{ChatOptionsSet, ChatRequest, ChatResponse, ChatStreamResponse};
use crate::resolver::{AuthData, Endpoint};
use crate::webc::WebResponse;
use crate::{Headers, Result, ServiceTarget};
use reqwest::RequestBuilder;

pub struct GithubCopilotAdapter;

impl GithubCopilotAdapter {
	pub const API_KEY_DEFAULT_ENV_NAME: &str = "GITHUB_TOKEN";
}

/// The GitHub Copilot adapter uses the GitHub Models inference API,
/// which is OpenAI-compatible with additional GitHub-specific headers.
/// Supports multiple publishers — the model name after `github_copilot::` is sent verbatim to the API:
/// - `github_copilot::openai/gpt-4.1-mini`
/// - `github_copilot::anthropic/claude-sonnet-4-6`
/// - `github_copilot::google/gemini-2.5-pro`
/// - `github_copilot::xai/grok-3-mini`
impl Adapter for GithubCopilotAdapter {
	const DEFAULT_API_KEY_ENV_NAME: Option<&'static str> = Some(Self::API_KEY_DEFAULT_ENV_NAME);

	fn default_auth() -> AuthData {
		match Self::DEFAULT_API_KEY_ENV_NAME {
			Some(env_name) => AuthData::from_env(env_name),
			None => AuthData::None,
		}
	}

	fn default_endpoint() -> Endpoint {
		const BASE_URL: &str = "https://models.github.ai/inference/";
		Endpoint::from_static(BASE_URL)
	}

	async fn all_model_names(kind: AdapterKind, endpoint: Endpoint, auth: AuthData) -> Result<Vec<String>> {
		OpenAIAdapter::list_model_names_for_end_target(kind, endpoint, auth).await
	}

	fn get_service_url(model: &ModelIden, service_type: ServiceType, endpoint: Endpoint) -> Result<String> {
		OpenAIAdapter::util_get_service_url(model, service_type, endpoint)
	}

	fn to_web_request_data(
		target: ServiceTarget,
		service_type: ServiceType,
		chat_req: ChatRequest,
		chat_options: ChatOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		let mut data = OpenAIAdapter::util_to_web_request_data(target, service_type, chat_req, chat_options, None)?;
		// GitHub Models API requires additional headers
		data.headers.merge(Headers::from(vec![(
			"Accept".to_string(),
			"application/vnd.github+json".to_string(),
		)]));
		Ok(data)
	}

	fn to_chat_response(
		model_iden: ModelIden,
		web_response: WebResponse,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<ChatResponse> {
		OpenAIAdapter::to_chat_response(model_iden, web_response, options_set)
	}

	fn to_chat_stream(
		model_iden: ModelIden,
		reqwest_builder: RequestBuilder,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<ChatStreamResponse> {
		OpenAIAdapter::to_chat_stream(model_iden, reqwest_builder, options_set)
	}

	fn to_embed_request_data(
		service_target: crate::ServiceTarget,
		embed_req: crate::embed::EmbedRequest,
		options_set: crate::embed::EmbedOptionsSet<'_, '_>,
	) -> Result<crate::adapter::WebRequestData> {
		OpenAIAdapter::to_embed_request_data(service_target, embed_req, options_set)
	}

	fn to_embed_response(
		model_iden: crate::ModelIden,
		web_response: crate::webc::WebResponse,
		options_set: crate::embed::EmbedOptionsSet<'_, '_>,
	) -> Result<crate::embed::EmbedResponse> {
		OpenAIAdapter::to_embed_response(model_iden, web_response, options_set)
	}
}

// region:    --- Tests

#[cfg(test)]
mod tests {
	use super::*;
	use crate::ServiceTarget;
	use crate::adapter::{Adapter, ServiceType};
	use crate::chat::{ChatOptionsSet, ChatRequest};
	use crate::resolver::AuthData;

	fn test_target() -> ServiceTarget {
		ServiceTarget {
			endpoint: GithubCopilotAdapter::default_endpoint(),
			auth: AuthData::from_single("test-key"),
			model: ModelIden::new(AdapterKind::GithubCopilot, "openai/gpt-4.1-mini"),
		}
	}

	fn make_request(service_type: ServiceType) -> WebRequestData {
		GithubCopilotAdapter::to_web_request_data(
			test_target(),
			service_type,
			ChatRequest::from_user("hello"),
			ChatOptionsSet::default(),
		)
		.expect("to_web_request_data should succeed")
	}

	#[test]
	fn test_url_construction() {
		let data = make_request(ServiceType::Chat);
		assert_eq!(data.url, "https://models.github.ai/inference/chat/completions");
	}

	#[test]
	fn test_accept_header() {
		let data = make_request(ServiceType::Chat);
		let accept = data
			.headers
			.iter()
			.find(|(k, _)| k.eq_ignore_ascii_case("Accept"))
			.map(|(_, v)| v.as_str());
		assert_eq!(accept, Some("application/vnd.github+json"));
	}

	#[test]
	fn test_authorization_header() {
		let data = make_request(ServiceType::Chat);
		let auth = data
			.headers
			.iter()
			.find(|(k, _)| k.eq_ignore_ascii_case("Authorization"))
			.map(|(_, v)| v.as_str());
		assert_eq!(auth, Some("Bearer test-key"));
	}

	#[test]
	fn test_payload_model_name() {
		let data = make_request(ServiceType::Chat);
		let model = data.payload.get("model").and_then(|v| v.as_str());
		assert_eq!(model, Some("openai/gpt-4.1-mini"));
	}

	#[test]
	fn test_payload_messages() {
		let data = make_request(ServiceType::Chat);
		let messages = data.payload.get("messages").and_then(|v| v.as_array());
		assert!(messages.is_some(), "payload should have messages array");
		let messages = messages.unwrap();
		assert!(!messages.is_empty(), "messages array should not be empty");
		let last = messages.last().unwrap();
		let content = last.get("content").and_then(|v| v.as_str());
		assert_eq!(content, Some("hello"));
	}

	#[test]
	fn test_payload_stream_false_for_chat() {
		let data = make_request(ServiceType::Chat);
		let stream = data.payload.get("stream").and_then(|v| v.as_bool());
		assert_eq!(stream, Some(false));
	}

	#[test]
	fn test_payload_stream_true_for_chat_stream() {
		let data = make_request(ServiceType::ChatStream);
		let stream = data.payload.get("stream").and_then(|v| v.as_bool());
		assert_eq!(stream, Some(true));
	}
}

// endregion: --- Tests

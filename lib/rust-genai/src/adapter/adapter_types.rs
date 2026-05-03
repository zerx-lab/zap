use crate::adapter::AdapterKind;
use crate::chat::{ChatOptionsSet, ChatRequest, ChatResponse, ChatStreamResponse};
use crate::embed::{EmbedOptionsSet, EmbedRequest, EmbedResponse};
use crate::resolver::{AuthData, Endpoint};
use crate::webc::WebResponse;
use crate::{Headers, ModelIden};
use crate::{Result, ServiceTarget};
use reqwest::RequestBuilder;
use serde_json::Value;

pub trait Adapter {
	const DEFAULT_API_KEY_ENV_NAME: Option<&'static str>;

	fn default_auth() -> AuthData;

	fn default_endpoint() -> Endpoint;

	// NOTE: Adapter is a crate trait, so it is acceptable to use async fn here.
	async fn all_model_names(kind: AdapterKind, endpoint: Endpoint, auth: AuthData) -> Result<Vec<String>>;

	/// The base service URL for this AdapterKind for the given service type.
	/// NOTE: For some services, the URL will be further updated in the to_web_request_data method.
	fn get_service_url(model_iden: &ModelIden, service_type: ServiceType, endpoint: Endpoint) -> Result<String>;

	/// To be implemented by Adapters.
	fn to_web_request_data(
		service_target: ServiceTarget,
		service_type: ServiceType,
		chat_req: ChatRequest,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<WebRequestData>;

	/// To be implemented by Adapters.
	fn to_chat_response(
		model_iden: ModelIden,
		web_response: WebResponse,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<ChatResponse>;

	/// To be implemented by Adapters.
	fn to_chat_stream(
		model_iden: ModelIden,
		reqwest_builder: RequestBuilder,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<ChatStreamResponse>;

	/// To be implemented by Adapters.
	fn to_embed_request_data(
		service_target: ServiceTarget,
		embed_req: EmbedRequest,
		options_set: EmbedOptionsSet<'_, '_>,
	) -> Result<WebRequestData>;

	/// To be implemented by Adapters.
	fn to_embed_response(
		model_iden: ModelIden,
		web_response: WebResponse,
		options_set: EmbedOptionsSet<'_, '_>,
	) -> Result<EmbedResponse>;
}

// region:    --- ServiceType

#[derive(Debug, Clone, Copy)]
pub enum ServiceType {
	Chat,
	ChatStream,
	Embed,
}

// endregion: --- ServiceType

// region:    --- WebRequestData

// NOTE: This cannot really move to `webc` because it must be public with the adapter, and `webc` is private for now.
#[derive(Debug, Clone)]
pub struct WebRequestData {
	pub url: String,
	pub headers: Headers,
	pub payload: Value,
}

// endregion: --- WebRequestData

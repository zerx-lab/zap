use crate::ModelIden;
use crate::adapter::dispatcher_macros::dispatch_adapter;
use crate::adapter::{Adapter, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{ChatOptionsSet, ChatRequest, ChatResponse, ChatStreamResponse};
use crate::embed::{EmbedOptionsSet, EmbedRequest, EmbedResponse};
use crate::resolver::{AuthData, Endpoint};
use crate::webc::WebResponse;
use crate::{Result, ServiceTarget};
use reqwest::RequestBuilder;

/// A construct that allows dispatching calls to the Adapters.
///
/// Note 1: This struct does not need to implement the Adapter trait, as some of its methods take the adapter kind as a parameter.
///
/// Note 2: This struct might be renamed to avoid confusion with the traditional Rust dispatcher pattern.
pub struct AdapterDispatcher;

impl AdapterDispatcher {
	pub fn default_endpoint(kind: AdapterKind) -> Endpoint {
		dispatch_adapter!(kind, A::default_endpoint())
	}

	pub fn default_auth(kind: AdapterKind) -> AuthData {
		dispatch_adapter!(kind, A::default_auth())
	}

	pub async fn all_model_names(kind: AdapterKind, endpoint: Endpoint, auth: AuthData) -> Result<Vec<String>> {
		dispatch_adapter!(kind, A::all_model_names(kind, endpoint, auth).await)
	}

	pub fn get_service_url(model: &ModelIden, service_type: ServiceType, endpoint: Endpoint) -> Result<String> {
		dispatch_adapter!(model.adapter_kind, A::get_service_url(model, service_type, endpoint))
	}

	pub fn to_web_request_data(
		target: ServiceTarget,
		service_type: ServiceType,
		chat_req: ChatRequest,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		let adapter_kind = target.model.adapter_kind;
		dispatch_adapter!(
			adapter_kind,
			A::to_web_request_data(target, service_type, chat_req, options_set)
		)
	}

	pub fn to_chat_response(
		model_iden: ModelIden,
		web_response: WebResponse,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<ChatResponse> {
		let adapter_kind = model_iden.adapter_kind;
		dispatch_adapter!(adapter_kind, A::to_chat_response(model_iden, web_response, options_set))
	}

	pub fn to_chat_stream(
		model_iden: ModelIden,
		reqwest_builder: RequestBuilder,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<ChatStreamResponse> {
		let adapter_kind = model_iden.adapter_kind;
		dispatch_adapter!(
			adapter_kind,
			A::to_chat_stream(model_iden, reqwest_builder, options_set)
		)
	}

	pub fn to_embed_request_data(
		target: ServiceTarget,
		embed_req: EmbedRequest,
		options_set: EmbedOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		let adapter_kind = target.model.adapter_kind;
		dispatch_adapter!(adapter_kind, A::to_embed_request_data(target, embed_req, options_set))
	}

	pub fn to_embed_response(
		model_iden: ModelIden,
		web_response: WebResponse,
		options_set: EmbedOptionsSet<'_, '_>,
	) -> Result<EmbedResponse> {
		let adapter_kind = model_iden.adapter_kind;
		dispatch_adapter!(
			adapter_kind,
			A::to_embed_response(model_iden, web_response, options_set)
		)
	}
}

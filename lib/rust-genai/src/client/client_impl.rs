use crate::adapter::{AdapterDispatcher, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{ChatOptions, ChatOptionsSet, ChatRequest, ChatResponse, ChatStreamResponse};
use crate::client::ModelSpec;
use crate::embed::{EmbedOptions, EmbedOptionsSet, EmbedRequest, EmbedResponse};
use crate::resolver::AuthData;
use crate::{Client, Error, ModelIden, Result, ServiceTarget};

/// High-level client APIs.
impl Client {
	/// Lists model names for the given adapter.
	///
	/// Notes:
	///
	/// - Non-Ollama adapters use a static list.
	///
	/// - Ollama queries the default host (http://localhost:11434/v1/).
	///
	/// - May evolve to accept a custom endpoint.
	///
	/// - For most adapters, names also drive AdapterKind detection (see [`AdapterKind`]).
	///
	/// - Adapters should filter non-chat models until more skills are supported.
	///   Future: `model_names(adapter_kind, Option<&[Skill]>)`.
	pub async fn all_model_names(&self, adapter_kind: AdapterKind) -> Result<Vec<String>> {
		let (auth, endpoint) = self.config().resolve_adapter_config(adapter_kind).await?;
		let models = AdapterDispatcher::all_model_names(adapter_kind, endpoint, auth).await?;
		Ok(models)
	}

	/// Builds a ModelIden by inferring AdapterKind from the model name.
	pub fn default_model(&self, model_name: &str) -> Result<ModelIden> {
		// -- First get the default ModelInfo
		let adapter_kind = AdapterKind::from_model(model_name)?;
		let model_iden = ModelIden::new(adapter_kind, model_name);
		Ok(model_iden)
	}

	/// Deprecated: use `Client::resolve_service_target`.
	#[deprecated(note = "use `client.resolve_service_target(model_name)`")]
	pub async fn resolve_model_iden(&self, model_name: &str) -> Result<ModelIden> {
		let model = self.default_model(model_name)?;
		let target = self.config().resolve_service_target(model).await?;
		Ok(target.model)
	}

	/// Resolves the service target (endpoint, auth, and model) for the given model.
	///
	/// Accepts any type that implements `Into<ModelSpec>`:
	/// - `&str` or `String`: Model name with full inference
	/// - `ModelIden`: Explicit adapter, resolves auth/endpoint
	/// - `ServiceTarget`: Uses directly, bypasses model mapping and auth resolution
	pub async fn resolve_service_target(&self, model: impl Into<ModelSpec>) -> Result<ServiceTarget> {
		self.config().resolve_model_spec(model.into()).await
	}

	/// Sends a chat request and returns the full response.
	///
	/// Accepts any type that implements `Into<ModelSpec>`:
	/// - `&str` or `String`: Model name with full inference
	/// - `ModelIden`: Explicit adapter, resolves auth/endpoint
	/// - `ServiceTarget`: Uses directly, bypasses model mapping and auth resolution
	pub async fn exec_chat(
		&self,
		model: impl Into<ModelSpec>,
		chat_req: ChatRequest,
		options: Option<&ChatOptions>,
	) -> Result<ChatResponse> {
		let options_set = ChatOptionsSet::default()
			.with_chat_options(options)
			.with_client_options(self.config().chat_options());

		let target = self.config().resolve_model_spec(model.into()).await?;
		let model = target.model.clone();
		let auth_data = target.auth.clone();

		let WebRequestData {
			mut url,
			mut headers,
			payload,
		} = AdapterDispatcher::to_web_request_data(target, ServiceType::Chat, chat_req, options_set.clone())?;

		if let Some(extra_headers) = options.and_then(|o| o.extra_headers.as_ref()) {
			headers.merge_with(extra_headers);
		}

		if let AuthData::RequestOverride {
			url: override_url,
			headers: override_headers,
		} = auth_data
		{
			url = override_url;
			headers = override_headers;
		};

		let web_res = self
			.web_client()
			.do_post(&url, &headers, &payload)
			.await
			.map_err(|webc_error| Error::WebModelCall {
				model_iden: model.clone(),
				webc_error,
			})?;

		// Note: here we capture/clone the raw body if set in the options_set
		let captured_raw_body = options_set.capture_raw_body().unwrap_or_default().then(|| web_res.body.clone());

		match AdapterDispatcher::to_chat_response(model.clone(), web_res, options_set) {
			Ok(mut chat_res) => {
				chat_res.captured_raw_body = captured_raw_body;
				Ok(chat_res)
			}
			Err(err) => {
				let response_body = captured_raw_body.unwrap_or_else(|| {
					"Raw response not captured. Use the ChatOptions.capturre_raw_body flag to see raw response in this error".into()
				});
				let err = Error::ChatResponseGeneration {
					model_iden: model,
					request_payload: Box::new(payload),
					response_body: Box::new(response_body),
					cause: err.to_string(),
				};
				Err(err)
			}
		}
	}

	/// Streams a chat response.
	///
	/// Accepts any type that implements `Into<ModelSpec>`:
	/// - `&str` or `String`: Model name with full inference
	/// - `ModelIden`: Explicit adapter, resolves auth/endpoint
	/// - `ServiceTarget`: Uses directly, bypasses model mapping and auth resolution
	pub async fn exec_chat_stream(
		&self,
		model: impl Into<ModelSpec>,
		chat_req: ChatRequest,
		options: Option<&ChatOptions>,
	) -> Result<ChatStreamResponse> {
		let options_set = ChatOptionsSet::default()
			.with_chat_options(options)
			.with_client_options(self.config().chat_options());

		let target = self.config().resolve_model_spec(model.into()).await?;
		let model = target.model.clone();
		let auth_data = target.auth.clone();

		let WebRequestData {
			mut url,
			mut headers,
			payload,
		} = AdapterDispatcher::to_web_request_data(target, ServiceType::ChatStream, chat_req, options_set.clone())?;

		if let Some(extra_headers) = options.and_then(|o| o.extra_headers.as_ref()) {
			headers.merge_with(extra_headers);
		}

		// TODO: Need to check this.
		//       This was part of the 429c5cee2241dbef9f33699b9c91202233c22816 commit
		//       But now it is missing in the the exec_chat(..) above, which is probably an issue.
		if let AuthData::RequestOverride {
			url: override_url,
			headers: override_headers,
		} = auth_data
		{
			url = override_url;
			headers = override_headers;
		};

		let reqwest_builder = self
			.web_client()
			.new_req_builder(&url, &headers, &payload)
			.map_err(|webc_error| Error::WebModelCall {
				model_iden: model.clone(),
				webc_error,
			})?;

		let res = AdapterDispatcher::to_chat_stream(model, reqwest_builder, options_set)?;

		Ok(res)
	}

	/// Creates embeddings for a single input string.
	///
	/// Accepts any type that implements `Into<ModelSpec>` for the model parameter.
	pub async fn embed(
		&self,
		model: impl Into<ModelSpec>,
		input: impl Into<String>,
		options: Option<&EmbedOptions>,
	) -> Result<EmbedResponse> {
		let embed_req = EmbedRequest::new(input);
		self.exec_embed(model, embed_req, options).await
	}

	/// Creates embeddings for multiple input strings.
	///
	/// Accepts any type that implements `Into<ModelSpec>` for the model parameter.
	pub async fn embed_batch(
		&self,
		model: impl Into<ModelSpec>,
		inputs: Vec<String>,
		options: Option<&EmbedOptions>,
	) -> Result<EmbedResponse> {
		let embed_req = EmbedRequest::new_batch(inputs);
		self.exec_embed(model, embed_req, options).await
	}

	/// Sends an embedding request and returns the response.
	///
	/// Accepts any type that implements `Into<ModelSpec>`:
	/// - `&str` or `String`: Model name with full inference
	/// - `ModelIden`: Explicit adapter, resolves auth/endpoint
	/// - `ServiceTarget`: Uses directly, bypasses model mapping and auth resolution
	pub async fn exec_embed(
		&self,
		model: impl Into<ModelSpec>,
		embed_req: EmbedRequest,
		options: Option<&EmbedOptions>,
	) -> Result<EmbedResponse> {
		let options_set = EmbedOptionsSet::new()
			.with_request_options(options)
			.with_client_options(self.config().embed_options());

		let target = self.config().resolve_model_spec(model.into()).await?;
		let model = target.model.clone();

		let WebRequestData { headers, payload, url } =
			AdapterDispatcher::to_embed_request_data(target, embed_req, options_set.clone())?;

		let web_res = self
			.web_client()
			.do_post(&url, &headers, &payload)
			.await
			.map_err(|webc_error| Error::WebModelCall {
				model_iden: model.clone(),
				webc_error,
			})?;

		let res = AdapterDispatcher::to_embed_response(model, web_res, options_set)?;

		Ok(res)
	}
}

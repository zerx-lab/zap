//! OpenAI Embeddings API implementation
//! API Documentation: <https://platform.openai.com/docs/api-reference/embeddings>

use crate::adapter::adapters::support::get_api_key;
use crate::adapter::{Adapter, ServiceType, WebRequestData};
use crate::chat::Usage;
use crate::embed::{EmbedOptionsSet, EmbedRequest, EmbedResponse, Embedding};
use crate::webc::WebResponse;
use crate::{Error, Headers, ModelIden, Result, ServiceTarget};
use serde::{Deserialize, Serialize};

// region:    --- OpenAI Embed Request

#[derive(Debug, Serialize)]
struct OpenAIEmbedRequest {
	input: OpenAIEmbedInput,
	model: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	encoding_format: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	dimensions: Option<usize>,
	#[serde(skip_serializing_if = "Option::is_none")]
	user: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum OpenAIEmbedInput {
	Single(String),
	Batch(Vec<String>),
}

// endregion: --- OpenAI Embed Request

// region:    --- OpenAI Embed Response

#[derive(Debug, Deserialize)]
struct OpenAIEmbedResponse {
	data: Vec<OpenAIEmbedData>,
	model: String,
	usage: OpenAIEmbedUsage,
}

#[derive(Debug, Deserialize)]
struct OpenAIEmbedData {
	embedding: Vec<f32>,
	index: usize,
}

#[derive(Debug, Deserialize)]
struct OpenAIEmbedUsage {
	prompt_tokens: u32,
	total_tokens: u32,
}

// endregion: --- OpenAI Embed Response

// region:    --- Public Functions

pub fn to_embed_request_data(
	service_target: ServiceTarget,
	embed_req: EmbedRequest,
	options_set: EmbedOptionsSet<'_, '_>,
) -> Result<WebRequestData> {
	let ServiceTarget { model, auth, .. } = service_target;
	let api_key = get_api_key(auth, &model)?;

	// Build headers
	let mut headers = Headers::from(vec![
		("Authorization".to_string(), format!("Bearer {api_key}")),
		("Content-Type".to_string(), "application/json".to_string()),
	]);

	// Add custom headers from options
	if let Some(custom_headers) = options_set.headers() {
		headers.merge_with(custom_headers);
	}

	// Convert EmbedRequest to OpenAI format
	let input = match embed_req.input {
		crate::embed::EmbedInput::Single(text) => OpenAIEmbedInput::Single(text),
		crate::embed::EmbedInput::Batch(texts) => OpenAIEmbedInput::Batch(texts),
	};

	// Extract the actual model name (without namespace)
	let (_, model_name) = model.model_name.namespace_and_name();

	let openai_req = OpenAIEmbedRequest {
		input,
		model: model_name.to_string(),
		encoding_format: options_set.encoding_format().map(|s| s.to_string()),
		dimensions: options_set.dimensions(),
		user: options_set.user().map(|s| s.to_string()),
	};

	let payload = serde_json::to_value(openai_req).map_err(|serde_error| Error::StreamParse {
		model_iden: model.clone(),
		serde_error,
	})?;

	// Get the service URL
	let url = <crate::adapter::openai::OpenAIAdapter as Adapter>::get_service_url(
		&model,
		ServiceType::Embed,
		service_target.endpoint,
	)?;

	Ok(WebRequestData { url, headers, payload })
}

pub fn to_embed_response(
	model_iden: ModelIden,
	web_response: WebResponse,
	options_set: EmbedOptionsSet<'_, '_>,
) -> Result<EmbedResponse> {
	let WebResponse { body, .. } = web_response;

	// Parse the OpenAI response
	let openai_res: OpenAIEmbedResponse =
		serde_json::from_value(body.clone()).map_err(|serde_error| Error::StreamParse {
			model_iden: model_iden.clone(),
			serde_error,
		})?;

	// Convert to our format
	let embeddings: Vec<Embedding> = openai_res
		.data
		.into_iter()
		.map(|data| Embedding::new(data.embedding, data.index))
		.collect();

	// Create usage information
	let usage = Usage {
		prompt_tokens: Some(openai_res.usage.prompt_tokens as i32),
		completion_tokens: None, // Embeddings don't have output tokens
		total_tokens: Some(openai_res.usage.total_tokens as i32),
		prompt_tokens_details: None,
		completion_tokens_details: None,
	};

	// Create provider model identifier
	let provider_model_iden = ModelIden {
		adapter_kind: model_iden.adapter_kind,
		model_name: openai_res.model.into(),
	};

	let mut response = EmbedResponse::new(embeddings, model_iden, provider_model_iden, usage);

	// Capture raw body if requested
	if options_set.capture_raw_body() {
		response = response.with_captured_raw_body(body);
	}

	Ok(response)
}

// endregion: --- Public Functions

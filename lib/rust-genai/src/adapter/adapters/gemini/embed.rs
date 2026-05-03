//! Gemini Embeddings API implementation
//! API Documentation: <https://ai.google.dev/gemini-api/docs/embeddings>

use crate::adapter::adapters::support::get_api_key;
use crate::adapter::{Adapter, ServiceType, WebRequestData};
use crate::chat::Usage;
use crate::embed::{EmbedOptionsSet, EmbedRequest, EmbedResponse, Embedding};
use crate::webc::WebResponse;
use crate::{Error, Headers, ModelIden, Result, ServiceTarget};
use serde::{Deserialize, Serialize};

// region:    --- Gemini Embed Request

#[derive(Debug, Serialize)]
struct GeminiEmbedRequest {
	model: String,
	content: GeminiContent,
	#[serde(skip_serializing_if = "Option::is_none")]
	#[serde(rename = "taskType")]
	task_type: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	#[serde(rename = "outputDimensionality")]
	output_dimensionality: Option<usize>,
}

#[derive(Debug, Serialize)]
struct GeminiBatchEmbedRequest {
	requests: Vec<GeminiEmbedContentRequest>,
}

#[derive(Debug, Serialize)]
struct GeminiEmbedContentRequest {
	model: String,
	content: GeminiContent,
	#[serde(skip_serializing_if = "Option::is_none")]
	#[serde(rename = "taskType")]
	task_type: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	#[serde(rename = "outputDimensionality")]
	output_dimensionality: Option<usize>,
}

#[derive(Debug, Serialize)]
struct GeminiContent {
	parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize)]
struct GeminiPart {
	text: String,
}

// endregion: --- Gemini Embed Request

// region:    --- Gemini Embed Response

#[derive(Debug, Deserialize)]
struct GeminiEmbedResponse {
	#[serde(rename = "embedding")]
	embedding: GeminiEmbedding,
}

#[derive(Debug, Deserialize)]
struct GeminiBatchEmbedResponse {
	#[serde(rename = "embeddings")]
	embeddings: Vec<GeminiEmbedding>,
}

#[derive(Debug, Deserialize)]
struct GeminiEmbedding {
	values: Vec<f32>,
}

// endregion: --- Gemini Embed Response

// region:    --- Public Functions

pub fn to_embed_request_data(
	service_target: ServiceTarget,
	embed_req: EmbedRequest,
	options_set: EmbedOptionsSet<'_, '_>,
) -> Result<WebRequestData> {
	let ServiceTarget { model, auth, .. } = service_target;
	let api_key = get_api_key(auth, &model)?;

	// Build headers - Gemini uses x-goog-api-key header
	let mut headers = Headers::from(vec![
		("x-goog-api-key".to_string(), api_key),
		("Content-Type".to_string(), "application/json".to_string()),
	]);

	// Add custom headers from options
	if let Some(custom_headers) = options_set.headers() {
		headers.merge_with(custom_headers);
	}

	// Extract the actual model name (without namespace) - not needed for Gemini request body
	// Get the model name for the request
	let (_, model_name) = model.model_name.namespace_and_name();
	let full_model_name = format!("models/{model_name}",);

	// Convert EmbedRequest to Gemini format and determine URL
	let (payload, is_batch) = match embed_req.input {
		crate::embed::EmbedInput::Single(text) => {
			// Handle empty text edge case - Gemini API returns 429 for empty strings
			let processed_text = if text.trim().is_empty() {
				" ".to_string() // Use a single space instead of empty string
			} else {
				text
			};

			// Single embedding request
			let gemini_req = GeminiEmbedRequest {
				model: full_model_name,
				content: GeminiContent {
					parts: vec![GeminiPart { text: processed_text }],
				},
				task_type: options_set
					.embedding_type()
					.map(|s| s.to_string())
					.or_else(|| Some("SEMANTIC_SIMILARITY".to_string())),
				output_dimensionality: options_set.dimensions(),
			};

			let payload = serde_json::to_value(gemini_req).map_err(|serde_error| Error::StreamParse {
				model_iden: model.clone(),
				serde_error,
			})?;

			(payload, false)
		}
		crate::embed::EmbedInput::Batch(texts) => {
			// Batch embedding request
			let requests: Vec<GeminiEmbedContentRequest> = texts
				.into_iter()
				.map(|text| {
					// Handle empty text edge case - Gemini API returns 429 for empty strings
					let processed_text = if text.trim().is_empty() {
						" ".to_string() // Use a single space instead of empty string
					} else {
						text
					};

					GeminiEmbedContentRequest {
						model: full_model_name.clone(),
						content: GeminiContent {
							parts: vec![GeminiPart { text: processed_text }],
						},
						task_type: options_set
							.embedding_type()
							.map(|s| s.to_string())
							.or_else(|| Some("SEMANTIC_SIMILARITY".to_string())),
						output_dimensionality: options_set.dimensions(),
					}
				})
				.collect();

			let gemini_req = GeminiBatchEmbedRequest { requests };

			let payload = serde_json::to_value(gemini_req).map_err(|serde_error| Error::StreamParse {
				model_iden: model.clone(),
				serde_error,
			})?;

			(payload, true)
		}
	};

	// Get the service URL and modify it for batch requests
	let mut url = <crate::adapter::gemini::GeminiAdapter as Adapter>::get_service_url(
		&model,
		ServiceType::Embed,
		service_target.endpoint,
	)?;

	// For batch requests, change :embedContent to :batchEmbedContents
	if is_batch {
		url = url.replace(":embedContent", ":batchEmbedContents");
	}

	Ok(WebRequestData { url, headers, payload })
}

pub fn to_embed_response(
	model_iden: ModelIden,
	web_response: WebResponse,
	options_set: EmbedOptionsSet<'_, '_>,
) -> Result<EmbedResponse> {
	let WebResponse { body, .. } = web_response;

	// Parse the Gemini response - try single first, then batch
	let embedding_vectors = if let Ok(single_res) = serde_json::from_value::<GeminiEmbedResponse>(body.clone()) {
		// Single embedding response
		vec![single_res.embedding.values]
	} else if let Ok(batch_res) = serde_json::from_value::<GeminiBatchEmbedResponse>(body.clone()) {
		// Batch embedding response
		batch_res.embeddings.into_iter().map(|e| e.values).collect()
	} else {
		return Err(Error::StreamParse {
			model_iden: model_iden.clone(),
			serde_error: serde_json::from_str::<()>("").unwrap_err(), // Create a dummy serde error
		});
	};

	// Convert to our format
	let embeddings: Vec<Embedding> = embedding_vectors
		.into_iter()
		.enumerate()
		.map(|(index, vector)| Embedding::new(vector, index))
		.collect();

	// Create usage information - Gemini doesn't provide token counts in embedding responses
	let usage = Usage {
		prompt_tokens: None, // Gemini doesn't provide token counts for embeddings
		completion_tokens: None,
		total_tokens: None,
		prompt_tokens_details: None,
		completion_tokens_details: None,
	};

	// Create provider model identifier
	let provider_model_iden = ModelIden {
		adapter_kind: model_iden.adapter_kind,
		model_name: model_iden.model_name.clone(),
	};

	let mut response = EmbedResponse::new(embeddings, model_iden, provider_model_iden, usage);

	// Capture raw body if requested
	if options_set.capture_raw_body() {
		response = response.with_captured_raw_body(body);
	}

	Ok(response)
}

// endregion: --- Public Functions

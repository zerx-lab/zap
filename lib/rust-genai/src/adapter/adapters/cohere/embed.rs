//! Cohere Embeddings API implementation
//! API Documentation: <https://docs.cohere.com/reference/embed>

use crate::adapter::adapters::support::get_api_key;
use crate::adapter::{Adapter, ServiceType, WebRequestData};
use crate::chat::Usage;
use crate::embed::{EmbedOptionsSet, EmbedRequest, EmbedResponse, Embedding};
use crate::webc::WebResponse;
use crate::{Error, Headers, ModelIden, Result, ServiceTarget};
use serde::{Deserialize, Serialize};

// region:    --- Cohere Embed Request

#[derive(Debug, Serialize)]
struct CohereEmbedRequest {
	#[serde(skip_serializing_if = "Option::is_none")]
	texts: Option<Vec<String>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	inputs: Option<Vec<CohereInput>>,
	model: String,
	input_type: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	embedding_types: Option<Vec<String>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	truncate: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	output_dimension: Option<usize>,
}

#[derive(Debug, Serialize)]
struct CohereInput {
	content: Vec<CohereContent>,
}

#[derive(Debug, Serialize)]
struct CohereContent {
	#[serde(rename = "type")]
	content_type: String,
	text: String,
}

// endregion: --- Cohere Embed Request

// region:    --- Cohere Embed Response

#[derive(Debug, Deserialize)]
struct CohereEmbedResponse {
	embeddings: CohereEmbeddings,
	meta: Option<CohereMeta>,
}

#[derive(Debug, Deserialize)]
struct CohereEmbeddings {
	#[serde(rename = "float")]
	float_embeddings: Option<Vec<Vec<f32>>>,
	int8: Option<Vec<Vec<i8>>>,
	uint8: Option<Vec<Vec<u8>>>,
	binary: Option<Vec<Vec<i8>>>,
	ubinary: Option<Vec<Vec<u8>>>,
}

#[derive(Debug, Deserialize)]
struct CohereMeta {
	billed_units: Option<CohereBilledUnits>,
	warnings: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct CohereBilledUnits {
	input_tokens: Option<u32>,
}

// endregion: --- Cohere Embed Response

// region:    --- Public Functions

pub fn to_embed_request_data(
	service_target: ServiceTarget,
	embed_req: EmbedRequest,
	options_set: EmbedOptionsSet<'_, '_>,
) -> Result<WebRequestData> {
	let ServiceTarget { model, auth, .. } = service_target;
	let api_key = get_api_key(auth, &model)?;

	// Extract the actual model name (without namespace)
	let (_, model_name) = model.model_name.namespace_and_name();

	// Build headers
	let mut headers = Headers::from(vec![
		("Authorization".to_string(), format!("Bearer {api_key}")),
		("Content-Type".to_string(), "application/json".to_string()),
	]);

	// Add custom headers from options
	if let Some(custom_headers) = options_set.headers() {
		headers.merge_with(custom_headers);
	}

	// Convert EmbedRequest to Cohere format
	let (texts, inputs) = match embed_req.input {
		crate::embed::EmbedInput::Single(text) => {
			// For single text, use the simpler texts array format
			(Some(vec![text]), None)
		}
		crate::embed::EmbedInput::Batch(texts) => {
			// For batch, use the texts array format
			(Some(texts), None)
		}
	};

	// Determine embedding types - default to float
	let embedding_types = {
		let format = options_set.encoding_format().unwrap_or("float");
		let embedding_type = match format {
			"float" | "int8" | "uint8" | "binary" | "ubinary" => format,
			_ => "float",
		};
		Some(vec![embedding_type.to_string()])
	};

	let cohere_req = CohereEmbedRequest {
		texts,
		inputs,
		model: model_name.to_string(),
		input_type: options_set.embedding_type().unwrap_or("search_document").to_string(),
		embedding_types,
		truncate: options_set
			.truncate()
			.map(|s| s.to_string())
			.or_else(|| Some("END".to_string())),
		output_dimension: options_set.dimensions(),
	};

	let payload = serde_json::to_value(cohere_req).map_err(|serde_error| Error::StreamParse {
		model_iden: model.clone(),
		serde_error,
	})?;

	// Get the service URL
	let url = <crate::adapter::cohere::CohereAdapter as Adapter>::get_service_url(
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

	// Parse the Cohere response
	let cohere_res: CohereEmbedResponse =
		serde_json::from_value(body.clone()).map_err(|serde_error| Error::StreamParse {
			model_iden: model_iden.clone(),
			serde_error,
		})?;

	// Extract embedding vectors, converting all types to f32
	let embedding_vectors = {
		let embeddings = &cohere_res.embeddings;

		if let Some(float_embeddings) = &embeddings.float_embeddings {
			float_embeddings.clone()
		} else if let Some(int8_embeddings) = &embeddings.int8 {
			int8_embeddings
				.iter()
				.map(|vec| vec.iter().map(|&v| v as f32).collect())
				.collect()
		} else if let Some(uint8_embeddings) = &embeddings.uint8 {
			uint8_embeddings
				.iter()
				.map(|vec| vec.iter().map(|&v| v as f32).collect())
				.collect()
		} else if let Some(binary_embeddings) = &embeddings.binary {
			binary_embeddings
				.iter()
				.map(|vec| vec.iter().map(|&v| v as f32).collect())
				.collect()
		} else if let Some(ubinary_embeddings) = &embeddings.ubinary {
			ubinary_embeddings
				.iter()
				.map(|vec| vec.iter().map(|&v| v as f32).collect())
				.collect()
		} else {
			return Err(Error::StreamParse {
				model_iden: model_iden.clone(),
				serde_error: serde_json::from_str::<()>("No embedding data found in response").unwrap_err(),
			});
		}
	};

	// Convert to our format
	let embeddings: Vec<Embedding> = embedding_vectors
		.into_iter()
		.enumerate()
		.map(|(index, vector)| Embedding::new(vector, index))
		.collect();

	// Log any API warnings and debug info
	if let Some(meta) = &cohere_res.meta
		&& let Some(warnings) = &meta.warnings
	{
		for warning in warnings {
			eprintln!("Cohere API Warning: {warning}");
		}
	}

	// Create usage information
	let usage = Usage {
		prompt_tokens: cohere_res
			.meta
			.as_ref()
			.and_then(|m| m.billed_units.as_ref())
			.and_then(|b| b.input_tokens)
			.map(|t| t as i32),
		completion_tokens: None, // Embeddings don't have output tokens
		total_tokens: cohere_res
			.meta
			.as_ref()
			.and_then(|m| m.billed_units.as_ref())
			.and_then(|b| b.input_tokens)
			.map(|t| t as i32),
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

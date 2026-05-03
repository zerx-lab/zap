//! This module contains all the types related to an Embed Response.

use crate::ModelIden;
use crate::chat::Usage;
use serde::{Deserialize, Serialize};

// region:    --- EmbedResponse

/// The Embed response when performing a direct `Client::embed` or `Client::embed_batch`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedResponse {
	/// The embedding vectors.
	pub embeddings: Vec<Embedding>,

	/// The resolved Model Identifier (AdapterKind/ModelName) used for this request.
	pub model_iden: ModelIden,

	/// The provider model iden. Will be `model_iden` if not returned or mapped, but can be different.
	/// For example, `text-embedding-3-small` model_iden might have a provider_model_iden as `text-embedding-3-small-2024-01-01`
	pub provider_model_iden: ModelIden,

	/// The eventual usage of the embed response (token counts, etc.)
	pub usage: Usage,

	/// The raw value of the response body, which can be used for provider specific features.
	pub captured_raw_body: Option<serde_json::Value>,
}

/// Constructors
impl EmbedResponse {
	/// Create a new EmbedResponse.
	pub fn new(
		embeddings: Vec<Embedding>,
		model_iden: ModelIden,
		provider_model_iden: ModelIden,
		usage: Usage,
	) -> Self {
		Self {
			embeddings,
			model_iden,
			provider_model_iden,
			usage,
			captured_raw_body: None,
		}
	}

	/// Create a new EmbedResponse with captured raw body.
	pub fn with_captured_raw_body(mut self, raw_body: serde_json::Value) -> Self {
		self.captured_raw_body = Some(raw_body);
		self
	}
}

/// Getters
impl EmbedResponse {
	/// Get the first embedding if available.
	pub fn first_embedding(&self) -> Option<&Embedding> {
		self.embeddings.first()
	}

	/// Get the first embedding vector if available.
	pub fn first_vector(&self) -> Option<&Vec<f32>> {
		self.first_embedding().map(|e| &e.vector)
	}

	/// Get all embedding vectors.
	pub fn vectors(&self) -> Vec<&Vec<f32>> {
		self.embeddings.iter().map(|e| &e.vector).collect()
	}

	/// Get all embedding vectors as owned data.
	pub fn into_vectors(self) -> Vec<Vec<f32>> {
		self.embeddings.into_iter().map(|e| e.vector).collect()
	}

	/// Check if this is a single embedding response.
	pub fn is_single(&self) -> bool {
		self.embeddings.len() == 1
	}

	/// Check if this is a batch embedding response.
	pub fn is_batch(&self) -> bool {
		self.embeddings.len() > 1
	}

	/// Get the number of embeddings.
	pub fn embedding_count(&self) -> usize {
		self.embeddings.len()
	}
}

// endregion: --- EmbedResponse

// region:    --- Embedding

/// A single embedding vector with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Embedding {
	/// The embedding vector.
	pub vector: Vec<f32>,

	/// The index of this embedding in the original request (for batch operations).
	pub index: usize,

	/// The dimensionality of the embedding vector.
	pub dimensions: usize,
}

/// Constructors
impl Embedding {
	/// Create a new Embedding.
	pub fn new(vector: Vec<f32>, index: usize) -> Self {
		let dimensions = vector.len();
		Self {
			vector,
			index,
			dimensions,
		}
	}

	/// Create a new Embedding with explicit dimensions.
	pub fn with_dimensions(vector: Vec<f32>, index: usize, dimensions: usize) -> Self {
		Self {
			vector,
			index,
			dimensions,
		}
	}
}

/// Getters
impl Embedding {
	/// Get the embedding vector.
	pub fn vector(&self) -> &Vec<f32> {
		&self.vector
	}

	/// Get the embedding vector as owned data.
	pub fn into_vector(self) -> Vec<f32> {
		self.vector
	}

	/// Get the index of this embedding.
	pub fn index(&self) -> usize {
		self.index
	}

	/// Get the dimensionality of the embedding.
	pub fn dimensions(&self) -> usize {
		self.dimensions
	}
}

// endregion: --- Embedding

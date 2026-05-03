mod support;

use crate::support::{TestResult, common_tests};
use genai::Client;
use genai::embed::{EmbedOptions, EmbedRequest};

const MODEL: &str = "text-embedding-3-small";
const MODEL_LARGE: &str = "text-embedding-3-large";
const MODEL_NS: &str = "openai::text-embedding-3-small";

// region:    --- Single Embedding Tests

#[tokio::test]
async fn test_embed_single_simple_ok() -> TestResult<()> {
	common_tests::common_test_embed_single_simple_ok(MODEL).await
}

#[tokio::test]
async fn test_embed_single_namespaced_ok() -> TestResult<()> {
	common_tests::common_test_embed_single_simple_ok(MODEL_NS).await
}

#[tokio::test]
async fn test_embed_single_with_options_ok() -> TestResult<()> {
	common_tests::common_test_embed_single_with_options_ok(MODEL).await
}

// endregion: --- Single Embedding Tests

// region:    --- Batch Embedding Tests

#[tokio::test]
async fn test_embed_batch_simple_ok() -> TestResult<()> {
	common_tests::common_test_embed_batch_simple_ok(MODEL).await
}

#[tokio::test]
async fn test_embed_batch_empty_should_fail() -> TestResult<()> {
	common_tests::common_test_embed_empty_batch_should_fail(MODEL).await
}

// endregion: --- Batch Embedding Tests

// region:    --- EmbedRequest Tests

#[tokio::test]
async fn test_embed_request_single_ok() -> TestResult<()> {
	let client = Client::default();
	let embed_req = EmbedRequest::from_text("Direct EmbedRequest test");

	let response = client.exec_embed(MODEL, embed_req, None).await?;

	assert_eq!(response.embedding_count(), 1);
	let embedding = response.first_embedding().ok_or("Should have embedding")?;
	assert!(embedding.dimensions() > 0);

	println!("✓ EmbedRequest single: {} dimensions", embedding.dimensions());

	Ok(())
}

#[tokio::test]
async fn test_embed_request_batch_ok() -> TestResult<()> {
	let client = Client::default();
	let embed_req = EmbedRequest::from_texts(vec![
		"Batch request text 1".to_string(),
		"Batch request text 2".to_string(),
	]);

	let response = client.exec_embed(MODEL, embed_req, None).await?;

	assert_eq!(response.embedding_count(), 2);
	for embedding in &response.embeddings {
		assert!(embedding.dimensions() > 0);
	}

	println!("✓ EmbedRequest batch: {} embeddings", response.embedding_count());

	Ok(())
}

// endregion: --- EmbedRequest Tests

// region:    --- Model Comparison Tests

#[tokio::test]
async fn test_embed_different_models_ok() -> TestResult<()> {
	let client = Client::default();
	let text = "Compare embedding models";

	// Test small model
	let response_small = client.embed(MODEL, text, None).await?;
	let dims_small = response_small.first_embedding().ok_or("Should have embedding")?.dimensions();

	// Test large model
	let response_large = client.embed(MODEL_LARGE, text, None).await?;
	let dims_large = response_large.first_embedding().ok_or("Should have embedding")?.dimensions();

	// Large model should have more dimensions
	assert!(dims_large >= dims_small);

	// println!("✓ Model comparison: small={dims_small} dims, large={dims_large} dims",);

	Ok(())
}

// endregion: --- Model Comparison Tests

// region:    --- Error Tests

#[tokio::test]
async fn test_embed_invalid_model_should_fail() -> TestResult<()> {
	let client = Client::default();
	let text = "Test with invalid model";

	let result = client.embed("invalid-embedding-model", text, None).await;

	// This should fail
	assert!(result.is_err());

	println!("✓ Invalid model correctly failed");

	Ok(())
}

#[tokio::test]
async fn test_embed_empty_text_should_work() -> TestResult<()> {
	let client = Client::default();
	let text = "";

	// Empty text should still work (though may have minimal dimensions)
	let response = client.embed(MODEL, text, None).await?;

	assert_eq!(response.embedding_count(), 1);
	let embedding = response.first_embedding().ok_or("Should have embedding")?;
	assert!(embedding.dimensions() > 0);

	println!("✓ Empty text embedding: {} dimensions", embedding.dimensions());

	Ok(())
}

// endregion: --- Error Tests

// region:    --- Utility Tests

#[tokio::test]
async fn test_embed_response_methods_ok() -> TestResult<()> {
	let client = Client::default();
	let texts = vec!["First".to_string(), "Second".to_string()];

	let response = client.embed_batch(MODEL, texts, None).await?;

	// Test response methods
	assert_eq!(response.embedding_count(), 2);
	assert!(response.is_batch());
	assert!(!response.is_single());

	// Test vector access methods
	let vectors = response.vectors();
	assert_eq!(vectors.len(), 2);

	let owned_vectors = response.clone().into_vectors();
	assert_eq!(owned_vectors.len(), 2);

	// Test first embedding access
	let first = response.first_embedding().ok_or("Should have embedding")?;
	let first_vector = response.first_vector().ok_or("should have first vector")?;
	assert_eq!(first.vector(), first_vector);

	println!("✓ Response utility methods work correctly");

	Ok(())
}

// endregion: --- Utility Tests

// region:    --- Provider-Specific Tests

#[tokio::test]
async fn test_embed_with_openai_specific_options_ok() -> TestResult<()> {
	// OpenAI supports encoding_format and user parameters
	let client = Client::default();
	let text = "Test with OpenAI-specific options";

	let options = EmbedOptions::new()
		.with_dimensions(512)
		.with_capture_usage(true)
		.with_encoding_format("float")
		.with_user("test-user");

	let response = client.embed(MODEL, text, Some(&options)).await?;

	let embedding = response.first_embedding().ok_or("Should have embedding")?;
	assert!(embedding.dimensions() > 0);
	assert!(response.usage.prompt_tokens.is_some());

	println!("✓ OpenAI-specific options: {} dimensions", embedding.dimensions());

	Ok(())
}

// endregion: --- Provider-Specific Tests

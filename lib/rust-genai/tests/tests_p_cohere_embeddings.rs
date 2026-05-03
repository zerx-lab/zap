mod support;

use crate::support::{TestResult, common_tests};

const MODEL: &str = "embed-english-v3.0";
const MODEL_V4: &str = "embed-v4.0";
const MODEL_NS: &str = "cohere::embed-english-v3.0";

// region:    --- Single Embedding Tests

#[tokio::test]
async fn test_cohere_embed_single_simple_ok() -> TestResult<()> {
	common_tests::common_test_embed_single_simple_ok(MODEL).await
}

#[tokio::test]
async fn test_cohere_embed_single_namespaced_ok() -> TestResult<()> {
	common_tests::common_test_embed_single_simple_ok(MODEL_NS).await
}

#[tokio::test]
async fn test_cohere_embed_single_with_options_ok() -> TestResult<()> {
	common_tests::common_test_embed_single_with_options_ok(MODEL_V4).await
}

// endregion: --- Single Embedding Tests

// region:    --- Batch Embedding Tests

#[tokio::test]
async fn test_cohere_embed_batch_simple_ok() -> TestResult<()> {
	common_tests::common_test_embed_batch_simple_ok(MODEL_V4).await
}

#[tokio::test]
async fn test_cohere_embed_batch_empty_should_fail() -> TestResult<()> {
	common_tests::common_test_embed_empty_batch_should_fail(MODEL).await
}

// endregion: --- Batch Embedding Tests

// region:    --- Provider-Specific Tests

#[tokio::test]
async fn test_cohere_embed_with_provider_specific_options_ok() -> TestResult<()> {
	common_tests::common_test_embed_provider_specific_options_ok(MODEL_V4, "search_query", Some("START")).await
}

// endregion: --- Provider-Specific Tests

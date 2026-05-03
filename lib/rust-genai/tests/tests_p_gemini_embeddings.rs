mod support;

use crate::support::{TestResult, common_tests};

const MODEL: &str = "gemini-embedding-001";
const MODEL_NS: &str = "gemini::gemini-embedding-001";

// region:    --- Single Embedding Tests

#[tokio::test]
async fn test_gemini_embed_single_simple_ok() -> TestResult<()> {
	common_tests::common_test_embed_single_simple_ok_with_usage_check(MODEL, false).await
}

#[tokio::test]
async fn test_gemini_embed_single_namespaced_ok() -> TestResult<()> {
	common_tests::common_test_embed_single_simple_ok_with_usage_check(MODEL_NS, false).await
}

#[tokio::test]
async fn test_gemini_embed_single_with_options_ok() -> TestResult<()> {
	common_tests::common_test_embed_single_with_options_ok_with_usage_check(MODEL, false).await
}

// endregion: --- Single Embedding Tests

// region:    --- Batch Embedding Tests

#[tokio::test]
async fn test_gemini_embed_batch_simple_ok() -> TestResult<()> {
	common_tests::common_test_embed_batch_simple_ok_with_usage_check(MODEL, false).await
}

#[tokio::test]
async fn test_gemini_embed_batch_empty_should_fail() -> TestResult<()> {
	common_tests::common_test_embed_empty_batch_should_fail(MODEL).await
}

// endregion: --- Batch Embedding Tests

// region:    --- Provider-Specific Tests

#[tokio::test]
async fn test_gemini_embed_with_provider_specific_options_ok() -> TestResult<()> {
	common_tests::common_test_embed_provider_specific_options_ok_with_usage_check(MODEL, "RETRIEVAL_QUERY", None, false)
		.await
}

// endregion: --- Provider-Specific Tests

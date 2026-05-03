mod support;

use crate::support::{Check, TestResult, common_tests};
use genai::adapter::AdapterKind;
use genai::resolver::AuthData;

// This will use the OpenAIRes adapter
const MODEL: &str = "gpt-5-codex";
// Also used for the verbosity (codex only supported medium)
const MODEL_NS: &str = "openai_resp::gpt-5-mini";

// region:    --- Provider Specific

// openai specific
#[tokio::test]
async fn test_chat_reasoning_minimal_ok() -> TestResult<()> {
	common_tests::common_test_chat_simple_ok("gpt-5-minimal", None).await
}

// endregion: --- Provider Specific

// region:    --- Chat

#[tokio::test]
async fn test_chat_simple_ok() -> TestResult<()> {
	common_tests::common_test_chat_simple_ok(MODEL, None).await
}

#[tokio::test]
async fn test_chat_verbosity_ok() -> TestResult<()> {
	common_tests::common_test_chat_verbosity_ok(MODEL_NS).await
}

#[tokio::test]
async fn test_chat_namespaced_ok() -> TestResult<()> {
	common_tests::common_test_chat_simple_ok(MODEL_NS, None).await
}

#[tokio::test]
async fn test_chat_top_system_ok() -> TestResult<()> {
	common_tests::common_test_chat_top_system_ok(MODEL).await
}

#[tokio::test]
async fn test_chat_multi_system_ok() -> TestResult<()> {
	common_tests::common_test_chat_multi_system_ok(MODEL).await
}

#[tokio::test]
async fn test_chat_json_mode_ok() -> TestResult<()> {
	common_tests::common_test_chat_json_mode_ok(MODEL, Some(Check::USAGE)).await
}

#[tokio::test]
async fn test_chat_json_structured_ok() -> TestResult<()> {
	common_tests::common_test_chat_json_structured_ok(MODEL, Some(Check::USAGE)).await
}

/// NOTE - Temprature not supported for gpt-5
// #[tokio::test]
// async fn test_chat_temperature_ok() -> TestResult<()> {
// 	common_tests::common_test_chat_temperature_ok(MODEL).await
// }
//
/// NOTE - Stop sequence does not need to be supported
// #[tokio::test]
// async fn test_chat_stop_sequences_ok() -> TestResult<()> {
// 	common_tests::common_test_chat_stop_sequences_ok(MODEL).await
// }

// endregion: --- Chat

// region:    --- Chat Implicit Cache

// NOTE - It seems `gpt-5-codex` does not cache often. gpt-5.. with same adapter cache better.
#[tokio::test]
async fn test_chat_cache_implicit_simple_ok() -> TestResult<()> {
	common_tests::common_test_chat_cache_implicit_simple_ok(MODEL_NS).await
}

// endregion: --- Chat Implicit Cache

// region:    --- Chat Stream Tests

#[tokio::test]
async fn test_chat_stream_simple_ok() -> TestResult<()> {
	common_tests::common_test_chat_stream_simple_ok(MODEL, None).await
}

#[tokio::test]
async fn test_chat_stream_capture_content_ok() -> TestResult<()> {
	common_tests::common_test_chat_stream_capture_content_ok(MODEL).await
}

#[tokio::test]
async fn test_chat_stream_capture_all_ok() -> TestResult<()> {
	common_tests::common_test_chat_stream_capture_all_ok(MODEL, None).await
}

// endregion: --- Chat Stream Tests

// region:    --- Binary Tests

#[tokio::test]
async fn test_chat_binary_image_url_ok() -> TestResult<()> {
	common_tests::common_test_chat_image_url_ok(MODEL).await
}

#[tokio::test]
async fn test_chat_binary_image_b64_ok() -> TestResult<()> {
	common_tests::common_test_chat_image_b64_ok(MODEL).await
}

#[tokio::test]
async fn test_chat_binary_pdf_b64_ok() -> TestResult<()> {
	common_tests::common_test_chat_pdf_b64_ok(MODEL).await
}

#[tokio::test]
async fn test_chat_binary_multi_b64_ok() -> TestResult<()> {
	common_tests::common_test_chat_multi_binary_b64_ok(MODEL).await
}

// endregion: --- Binary Tests

// region:    --- Tool Tests

#[tokio::test]
async fn test_tool_simple_ok() -> TestResult<()> {
	common_tests::common_test_tool_simple_ok(MODEL).await
}

#[tokio::test]
async fn test_tool_full_flow_ok() -> TestResult<()> {
	common_tests::common_test_tool_full_flow_ok(MODEL).await
}
// endregion: --- Tool Tests

// region:    --- Resolver Tests

#[tokio::test]
async fn test_resolver_auth_ok() -> TestResult<()> {
	common_tests::common_test_resolver_auth_ok(MODEL, AuthData::from_env("OPENAI_API_KEY")).await
}

// endregion: --- Resolver Tests

// region:    --- List

#[tokio::test]
async fn test_list_models() -> TestResult<()> {
	common_tests::common_test_list_models(AdapterKind::OpenAIResp, "gpt-5-codex").await
}

// endregion: --- List

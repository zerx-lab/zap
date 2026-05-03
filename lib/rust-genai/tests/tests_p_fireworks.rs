mod support;

use crate::support::{Check, TestResult, common_tests};
use genai::{adapter::AdapterKind, resolver::AuthData};

// const MODEL: &str = "fireworks::qwen3-coder-480b-a35b-instruct";
// const MODEL: &str = "fireworks::gpt-oss-120b";
const MODEL: &str = "accounts/fireworks/models/gpt-oss-20b";
const MODEL_NS: &str = "fireworks::deepseek-v4-pro";
const MODEL_IMAGE: &str = "fireworks::kimi-k2p5";

// region:    --- Chat

#[tokio::test]
async fn test_chat_simple_ok() -> TestResult<()> {
	common_tests::common_test_chat_simple_ok(MODEL, None).await
}

#[tokio::test]
async fn test_chat_namespaced_ok() -> TestResult<()> {
	common_tests::common_test_chat_simple_ok(MODEL_NS, None).await
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

#[tokio::test]
async fn test_chat_temperature_ok() -> TestResult<()> {
	common_tests::common_test_chat_temperature_ok(MODEL).await
}

#[tokio::test]
async fn test_chat_stop_sequences_ok() -> TestResult<()> {
	common_tests::common_test_chat_stop_sequences_ok(MODEL).await
}

// endregion: --- Chat

// region:    --- Chat Implicit Cache

/// Caching does not seem to be supported for fireworks (at leat not reported)
// #[tokio::test]
// async fn test_chat_cache_implicit_simple_ok() -> TestResult<()> {
// 	common_tests::common_test_chat_cache_implicit_simple_ok(MODEL).await
// }

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

// #[tokio::test]
// async fn test_chat_binary_image_url_ok() -> TestResult<()> {
// 	common_tests::common_test_chat_image_url_ok(MODEL_IMAGE).await
// }

#[tokio::test]
async fn test_chat_binary_image_b64_ok() -> TestResult<()> {
	common_tests::common_test_chat_image_b64_ok(MODEL_IMAGE).await
}

// PDF not supported for fireworks.ai

// #[tokio::test]
// async fn test_chat_binary_pdf_b64_ok() -> TestResult<()> {
// 	common_tests::common_test_chat_pdf_b64_ok(MODEL).await
// }

// #[tokio::test]
// async fn test_chat_binary_multi_b64_ok() -> TestResult<()> {
// 	common_tests::common_test_chat_multi_binary_b64_ok(MODEL).await
// }

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
	common_tests::common_test_resolver_auth_ok(MODEL, AuthData::from_env("FIREWORKS_API_KEY")).await
}

// endregion: --- Resolver Tests

// region:    --- List

#[tokio::test]
async fn test_list_models() -> TestResult<()> {
	common_tests::common_test_list_models(AdapterKind::Fireworks, "accounts/fireworks/models/gpt-oss-120b").await
}

// endregion: --- List

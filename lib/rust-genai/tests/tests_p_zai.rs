mod support;

use crate::support::{Check, TestResult, common_tests};
use genai::adapter::AdapterKind;
use genai::resolver::AuthData;

const MODEL: &str = "glm-4-plus";
const MODEL_NS: &str = "zai::glm-4-plus";
const MODEL_V: &str = "glm-4v-flash"; // Visual language model does not support function calling

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

/// NOTE - Disable for now, not supported by Zhipu as of 2025-07-08
// #[tokio::test]
// async fn test_chat_json_structured_ok() -> TestResult<()> {
// 	common_tests::common_test_chat_json_structured_ok(MODEL, Some(Check::USAGE)).await
// }

#[tokio::test]
async fn test_chat_temperature_ok() -> TestResult<()> {
	common_tests::common_test_chat_temperature_ok(MODEL).await
}

/// NOTE - Disabled for now, as the model currently includes the stop sequences as the last sequences in the generation as of 2025-07-08.
// #[tokio::test]
// async fn test_chat_stop_sequences_ok() -> TestResult<()> {
// 	common_tests::common_test_chat_stop_sequences_ok(MODEL).await
// }
// endregion: --- Chat
// region:    --- Chat Implicit Cache
/// NOTE - Disable for now, not supported by Zhipu as of 2025-07-08
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

#[tokio::test]
async fn test_chat_binary_image_url_ok() -> TestResult<()> {
	common_tests::common_test_chat_image_url_ok(MODEL).await
}

#[tokio::test]
async fn test_chat_binary_image_b64_ok() -> TestResult<()> {
	common_tests::common_test_chat_image_b64_ok(MODEL_V).await
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
	common_tests::common_test_resolver_auth_ok(MODEL, AuthData::from_env("ZAI_API_KEY")).await
}

// endregion: --- Resolver Tests

// region:    --- List

#[tokio::test]
async fn test_list_models() -> TestResult<()> {
	common_tests::common_test_list_models(AdapterKind::Zai, "glm-4-plus").await
}

// endregion: --- List

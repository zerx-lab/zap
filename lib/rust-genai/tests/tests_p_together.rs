mod support;

use crate::support::{Check, TestResult, common_tests};
use genai::resolver::AuthData;
use serial_test::serial;

// meta-llama/Llama-3-8b-chat-hf ($0.2) , Qwen/Qwen3-Coder-480B-A35B-Instruct-FP8 ($2)
// meta-llama/Llama-3.3-70B-Instruct-Turbo ($0.88) , Qwen/Qwen3-235B-A22B-Instruct-2507-tput ($0.2/$0.6)
const MODEL: &str = "together::Qwen/Qwen3-235B-A22B-Instruct-2507-tput";
//const MODEL_FOR_IMAGE: &str = "together::Qwen/Qwen3-Coder-480B-A35B-Instruct-FP8";
const MODEL_NS: &str = "together::Qwen/Qwen3-235B-A22B-Instruct-2507-tput";

// region:    --- Chat

#[tokio::test]
#[serial(together)]
async fn test_chat_simple_ok() -> TestResult<()> {
	common_tests::common_test_chat_simple_ok(MODEL, None).await
}

#[tokio::test]
#[serial(together)]
async fn test_chat_namespaced_ok() -> TestResult<()> {
	common_tests::common_test_chat_simple_ok(MODEL_NS, None).await
}

#[tokio::test]
#[serial(together)]
async fn test_chat_multi_system_ok() -> TestResult<()> {
	common_tests::common_test_chat_multi_system_ok(MODEL).await
}

#[tokio::test]
#[serial(together)]
async fn test_chat_json_mode_ok() -> TestResult<()> {
	common_tests::common_test_chat_json_mode_ok(MODEL, Some(Check::USAGE)).await
}

#[tokio::test]
#[serial(together)]
async fn test_chat_json_structured_ok() -> TestResult<()> {
	common_tests::common_test_chat_json_structured_ok(MODEL, Some(Check::USAGE)).await
}

#[tokio::test]
#[serial(together)]
async fn test_chat_temperature_ok() -> TestResult<()> {
	common_tests::common_test_chat_temperature_ok(MODEL).await
}

#[tokio::test]
#[serial(together)]
async fn test_chat_stop_sequences_ok() -> TestResult<()> {
	common_tests::common_test_chat_stop_sequences_ok(MODEL).await
}

// endregion: --- Chat

// region:    --- Chat Implicit Cache

/// Caching does not seem to be supported for together (at least not reported)
// #[tokio::test]
// #[serial(together)]
// async fn test_chat_cache_implicit_simple_ok() -> TestResult<()> {
// 	common_tests::common_test_chat_cache_implicit_simple_ok(MODEL).await
// }

// endregion: --- Chat Implicit Cache

// region:    --- Chat Stream Tests

#[tokio::test]
#[serial(together)]
async fn test_chat_stream_simple_ok() -> TestResult<()> {
	common_tests::common_test_chat_stream_simple_ok(MODEL, None).await
}

#[tokio::test]
#[serial(together)]
async fn test_chat_stream_capture_content_ok() -> TestResult<()> {
	common_tests::common_test_chat_stream_capture_content_ok(MODEL).await
}

#[tokio::test]
#[serial(together)]
async fn test_chat_stream_capture_all_ok() -> TestResult<()> {
	common_tests::common_test_chat_stream_capture_all_ok(MODEL, None).await
}

// endregion: --- Chat Stream Tests

// region:    --- Binary Tests

// #[tokio::test]
// #[serial(together)]
// async fn test_chat_binary_image_url_ok() -> TestResult<()> {
// 	common_tests::common_test_chat_image_url_ok(MODEL).await
// }

// NOTE: Only found that Qwen/Qwen3-Coder-480B-A35B-Instruct-FP8 was supporting image without returning
//       invalid format, but the rendered image could not be viewed yet.
// #[tokio::test]
// #[serial(together)]
// async fn test_chat_binary_image_b64_ok() -> TestResult<()> {
// 	common_tests::common_test_chat_image_b64_ok(MODEL_FOR_IMAGE).await
// }

// NOT SUPPORTED YET

// #[tokio::test]
// #[serial(together)]
// #[ignore = "Binary PDF is currently not supported by TogetherAI."]
// async fn test_chat_binary_pdf_b64_ok() -> TestResult<()> {
// 	common_tests::common_test_chat_pdf_b64_ok(MODEL).await
// }

// #[tokio::test]
// #[serial(together)]
// #[ignore = "Multiple binary payloads are currently not supported by TogetherAI."]
// async fn test_chat_binary_multi_b64_ok() -> TestResult<()> {
// 	common_tests::common_test_chat_multi_binary_b64_ok(MODEL).await
// }

// endregion: --- Binary Tests

// region:    --- Tool Tests

#[tokio::test]
#[serial(together)]
async fn test_tool_simple_ok() -> TestResult<()> {
	common_tests::common_test_tool_simple_ok(MODEL).await
}

// NOTE for now not working with Llama-3.3-70B-Instruct-Turbo
// TODO: need to investigate
#[tokio::test]
#[serial(together)]
async fn test_tool_full_flow_ok() -> TestResult<()> {
	common_tests::common_test_tool_full_flow_ok(MODEL).await
}

// endregion: --- Tool Tests

// region:    --- Resolver Tests

#[tokio::test]
#[serial(together)]
async fn test_resolver_auth_ok() -> TestResult<()> {
	common_tests::common_test_resolver_auth_ok(MODEL, AuthData::from_env("TOGETHER_API_KEY")).await
}

// endregion: --- Resolver Tests

// region:    --- List

// NOTE: 2026-02-15 - returns empty array
// #[tokio::test]
// #[serial(together)]
// async fn test_list_models() -> TestResult<()> {
// 	common_tests::common_test_list_models(AdapterKind::Together, "Qwen").await
// }

// endregion: --- List

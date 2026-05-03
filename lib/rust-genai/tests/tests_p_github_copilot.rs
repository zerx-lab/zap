mod support;

use crate::support::{Check, TestResult, common_tests};
use genai::resolver::AuthData;
use serial_test::serial;

const MODEL: &str = "github_copilot::openai/gpt-4.1-mini";
const MODEL_NS: &str = "github_copilot::openai/gpt-4.1-mini";

// NOTE: GitHub Copilot supports multiple publishers (openai/, anthropic/, google/, xai/, meta/).
// Tests use openai/gpt-4.1-mini as the default. Other publishers work with the same adapter.

// region:    --- Chat

#[tokio::test]
#[serial(github_copilot)]
async fn test_chat_simple_ok() -> TestResult<()> {
	common_tests::common_test_chat_simple_ok(MODEL, None).await
}

#[tokio::test]
#[serial(github_copilot)]
async fn test_chat_namespaced_ok() -> TestResult<()> {
	common_tests::common_test_chat_simple_ok(MODEL_NS, None).await
}

#[tokio::test]
#[serial(github_copilot)]
async fn test_chat_multi_system_ok() -> TestResult<()> {
	common_tests::common_test_chat_multi_system_ok(MODEL).await
}

#[tokio::test]
#[serial(github_copilot)]
async fn test_chat_json_mode_ok() -> TestResult<()> {
	common_tests::common_test_chat_json_mode_ok(MODEL, Some(Check::USAGE)).await
}

#[tokio::test]
#[serial(github_copilot)]
async fn test_chat_json_structured_ok() -> TestResult<()> {
	common_tests::common_test_chat_json_structured_ok(MODEL, Some(Check::USAGE)).await
}

#[tokio::test]
#[serial(github_copilot)]
async fn test_chat_temperature_ok() -> TestResult<()> {
	common_tests::common_test_chat_temperature_ok(MODEL).await
}

#[tokio::test]
#[serial(github_copilot)]
async fn test_chat_stop_sequences_ok() -> TestResult<()> {
	common_tests::common_test_chat_stop_sequences_ok(MODEL).await
}

// endregion: --- Chat

// region:    --- Chat Stream Tests

#[tokio::test]
#[serial(github_copilot)]
async fn test_chat_stream_simple_ok() -> TestResult<()> {
	common_tests::common_test_chat_stream_simple_ok(MODEL, None).await
}

#[tokio::test]
#[serial(github_copilot)]
async fn test_chat_stream_capture_content_ok() -> TestResult<()> {
	common_tests::common_test_chat_stream_capture_content_ok(MODEL).await
}

// GitHub Models API does not return usage tokens (prompt_tokens) in streaming responses
// #[tokio::test]
// #[serial(github_copilot)]
// async fn test_chat_stream_capture_all_ok() -> TestResult<()> {
// 	common_tests::common_test_chat_stream_capture_all_ok(MODEL, None).await
// }

#[tokio::test]
#[serial(github_copilot)]
async fn test_chat_stream_tool_capture_ok() -> TestResult<()> {
	common_tests::common_test_chat_stream_tool_capture_ok(MODEL).await
}

// endregion: --- Chat Stream Tests

// region:    --- Binary Tests

// GitHub Models API binary support not yet verified for this model

// #[tokio::test]
// async fn test_chat_binary_image_url_ok() -> TestResult<()> {
// 	common_tests::common_test_chat_image_url_ok(MODEL).await
// }

// #[tokio::test]
// async fn test_chat_binary_image_b64_ok() -> TestResult<()> {
// 	common_tests::common_test_chat_image_b64_ok(MODEL).await
// }

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
#[serial(github_copilot)]
async fn test_tool_simple_ok() -> TestResult<()> {
	common_tests::common_test_tool_simple_ok(MODEL).await
}

#[tokio::test]
#[serial(github_copilot)]
async fn test_tool_full_flow_ok() -> TestResult<()> {
	common_tests::common_test_tool_full_flow_ok(MODEL).await
}

// endregion: --- Tool Tests

// region:    --- Resolver Tests

#[tokio::test]
#[serial(github_copilot)]
async fn test_resolver_auth_ok() -> TestResult<()> {
	common_tests::common_test_resolver_auth_ok(MODEL, AuthData::from_env("GITHUB_TOKEN")).await
}

// endregion: --- Resolver Tests

// region:    --- List

// GitHub Models API does not support the /models listing endpoint (returns 404)
// #[tokio::test]
// #[serial(github_copilot)]
// async fn test_list_models() -> TestResult<()> {
// 	common_tests::common_test_list_models(AdapterKind::GithubCopilot, "gpt-4.1-mini").await
// }

// endregion: --- List

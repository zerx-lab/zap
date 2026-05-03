mod support;

use crate::support::{TestResult, common_tests};
use genai::adapter::AdapterKind;
use genai::resolver::AuthData;

// https://ollama.com/library
// "gemma3:4b" "phi3:latest" "gpt-oss:20b"
// NOTE: "gpt-oss:20b" has some issues on json_mode, stop_sequence
const MODEL: &str = "gemma3:4b"; // Text / Image
const MODEL_NS: &str = "ollama::gemma3:4b"; // Text / Image
const MODEL_TOOL: &str = "ollama::qwen3:4b";

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
	// Note: Ollama does not capture Uage when JSON mode.
	common_tests::common_test_chat_json_mode_ok(MODEL, None).await
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

// region:    --- Chat Stream Tests

#[tokio::test]
async fn test_chat_stream_simple_ok() -> TestResult<()> {
	common_tests::common_test_chat_stream_simple_ok(MODEL, None).await
}

#[tokio::test]
async fn test_chat_stream_capture_content_ok() -> TestResult<()> {
	common_tests::common_test_chat_stream_capture_content_ok(MODEL).await
}

// /// COMMENTED FOR NOW AS OLLAMA OpenAI Compatibility Layer does not support
// /// usage tokens when streaming. See https://github.com/ollama/ollama/issues/4448
// #[tokio::test]
// async fn test_chat_stream_capture_all_ok() -> TestResult<()> {
// 	common_tests::common_test_chat_stream_capture_all_ok(MODEL).await
// }

// endregion: --- Chat Stream Tests

// region:    --- Tool Tests

#[tokio::test]
async fn test_tool_simple_ok() -> TestResult<()> {
	common_tests::common_test_tool_simple_ok(MODEL_TOOL).await
}

#[tokio::test]
async fn test_tool_full_flow_ok() -> TestResult<()> {
	common_tests::common_test_tool_full_flow_ok(MODEL_TOOL).await
}
// endregion: --- Tool Tests

/* Added Binary Tests region (commented-out until Ollama supports binary inputs) */
// region:    --- Binary Tests

/// COMMENTED FOR NOW AS OLLAMA models currently do not support binary (image/pdf) inputs.
/// Once Ollama adds support, uncomment the tests below.

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

// region:    --- Resolver Tests

#[tokio::test]
async fn test_resolver_auth_ok() -> TestResult<()> {
	common_tests::common_test_resolver_auth_ok(MODEL, AuthData::from_single("ollama")).await
}

// endregion: --- Resolver Tests

// region:    --- List

/// NOTE this test assume the "gemma3:4b" is present.
#[tokio::test]
async fn test_list_models() -> TestResult<()> {
	common_tests::common_test_list_models(AdapterKind::Ollama, "gemma3:4b").await
}

// endregion: --- List

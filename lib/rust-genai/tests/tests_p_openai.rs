mod support;

use crate::support::{Check, TestResult, common_tests};
use genai::adapter::AdapterKind;
use genai::chat::ReasoningEffort;
use genai::resolver::AuthData;

// note: "gpt-4o-mini" has issue when image & pdf
// as for 2025-08-08 gpt-5-mini does not support temperature & stop sequence
const MODEL_LATEST: &str = "gpt-5.4";
const MODEL_MINI: &str = "gpt-5.4-mini"; // for the streaming reasoning test
const MODEL_MINI_XHIGH: &str = "gpt-5.4-mini-xhigh";
const AUDIO_MODEL: &str = "gpt-audio-mini";
const MODEL2: &str = "gpt-4.1-mini"; // for temperature & stop sequence
const MODEL_NS: &str = "openai::gpt-5.4-mini";

// region:    --- Provider Specific

// openai specific
#[tokio::test]
async fn test_chat_reasoning_minimal_ok() -> TestResult<()> {
	common_tests::common_test_chat_simple_ok("gpt-5-mini-minimal", None).await
}

// gpt-5-pro (different api than gpt-5)
// expensive, so, will be commented most of the time.
// #[tokio::test]
// async fn test_chat_gpt_5_pro_simple_ok() -> TestResult<()> {
// 	common_tests::common_test_chat_simple_ok("gpt-5-pro", None).await
// }

// endregion: --- Provider Specific

// region:    --- Chat

#[tokio::test]
async fn test_chat_simple_ok() -> TestResult<()> {
	common_tests::common_test_chat_simple_ok(MODEL_LATEST, None).await
}

#[tokio::test]
async fn test_chat_reasoning_ok() -> TestResult<()> {
	// For now, do not test Check::REASONING, for OpenAI as it is not captured
	common_tests::common_test_chat_reasoning_ok(MODEL_LATEST, ReasoningEffort::High, Some(Check::REASONING_USAGE)).await
}

#[tokio::test]
async fn test_chat_verbosity_ok() -> TestResult<()> {
	common_tests::common_test_chat_verbosity_ok(MODEL_MINI).await
}

#[tokio::test]
async fn test_chat_namespaced_ok() -> TestResult<()> {
	common_tests::common_test_chat_simple_ok(MODEL_NS, None).await
}

#[tokio::test]
async fn test_chat_multi_system_ok() -> TestResult<()> {
	common_tests::common_test_chat_multi_system_ok(MODEL_LATEST).await
}

#[tokio::test]
async fn test_chat_json_mode_ok() -> TestResult<()> {
	common_tests::common_test_chat_json_mode_ok(MODEL_LATEST, Some(Check::USAGE)).await
}

#[tokio::test]
async fn test_chat_json_structured_ok() -> TestResult<()> {
	common_tests::common_test_chat_json_structured_ok(MODEL_LATEST, Some(Check::USAGE)).await
}

#[tokio::test]
async fn test_chat_temperature_ok() -> TestResult<()> {
	common_tests::common_test_chat_temperature_ok(MODEL2).await
}

#[tokio::test]
async fn test_chat_stop_sequences_ok() -> TestResult<()> {
	common_tests::common_test_chat_stop_sequences_ok(MODEL2).await
}

// endregion: --- Chat

// region:    --- Chat Implicit Cache

#[tokio::test]
async fn test_chat_cache_implicit_simple_ok() -> TestResult<()> {
	common_tests::common_test_chat_cache_implicit_simple_ok(MODEL_MINI).await
}

// endregion: --- Chat Implicit Cache

// region:    --- Chat Stream Tests

#[tokio::test]
async fn test_chat_stream_simple_ok() -> TestResult<()> {
	common_tests::common_test_chat_stream_simple_ok(MODEL_LATEST, None).await
}

#[tokio::test]
async fn test_chat_stream_capture_content_ok() -> TestResult<()> {
	common_tests::common_test_chat_stream_capture_content_ok(MODEL_LATEST).await
}

#[tokio::test]
async fn test_chat_stream_capture_all_ok() -> TestResult<()> {
	// NOTE: gpt-5.1 even when reasoning is Medium, does not give reasoning when simple chat when streaming
	common_tests::common_test_chat_stream_capture_all_ok(MODEL_MINI_XHIGH, Some(Check::REASONING_USAGE)).await
}

#[tokio::test]
async fn test_chat_stream_tool_capture_ok() -> TestResult<()> {
	// NOTE: For now the OpenAI Adapter do not capture the thinking as not available in chat completions
	common_tests::common_test_chat_stream_tool_capture_ok(MODEL_LATEST).await
}

// endregion: --- Chat Stream Tests

// region:    --- Binary Tests

#[tokio::test]
async fn test_chat_binary_image_url_ok() -> TestResult<()> {
	common_tests::common_test_chat_image_url_ok(MODEL_LATEST).await
}

#[tokio::test]
async fn test_chat_binary_image_b64_ok() -> TestResult<()> {
	common_tests::common_test_chat_image_b64_ok(MODEL_LATEST).await
}

#[tokio::test]
async fn test_chat_binary_image_file_ok() -> TestResult<()> {
	common_tests::common_test_chat_image_file_ok(MODEL_LATEST).await
}

#[tokio::test]
async fn test_chat_binary_audio_b64_ok() -> TestResult<()> {
	common_tests::common_test_chat_audio_b64_ok(AUDIO_MODEL).await
}

#[tokio::test]
async fn test_chat_binary_pdf_b64_ok() -> TestResult<()> {
	common_tests::common_test_chat_pdf_b64_ok(MODEL_LATEST).await
}

#[tokio::test]
async fn test_chat_binary_multi_b64_ok() -> TestResult<()> {
	common_tests::common_test_chat_multi_binary_b64_ok(MODEL_LATEST).await
}

// endregion: --- Binary Tests

// region:    --- Tool Tests

#[tokio::test]
async fn test_tool_simple_ok() -> TestResult<()> {
	common_tests::common_test_tool_simple_ok(MODEL_LATEST).await
}

#[tokio::test]
async fn test_tool_full_flow_ok() -> TestResult<()> {
	common_tests::common_test_tool_full_flow_ok(MODEL_LATEST).await
}
// endregion: --- Tool Tests

// region:    --- Resolver Tests

#[tokio::test]
async fn test_resolver_auth_ok() -> TestResult<()> {
	common_tests::common_test_resolver_auth_ok(MODEL_LATEST, AuthData::from_env("OPENAI_API_KEY")).await
}

// endregion: --- Resolver Tests

// region:    --- List

#[tokio::test]
async fn test_list_models() -> TestResult<()> {
	common_tests::common_test_list_models(AdapterKind::OpenAI, "gpt-5.2-codex").await
}

// endregion: --- List

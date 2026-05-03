mod support;

use crate::support::{Check, TestResult, common_tests};
use genai::adapter::AdapterKind;
use genai::resolver::AuthData;

// deepseek-reasoner (note: Does not support json output and streaming - or different streaming -)
const MODEL: &str = "deepseek-reasoner";

// region:    --- Chat

#[tokio::test]
async fn test_chat_simple_ok() -> TestResult<()> {
	common_tests::common_test_chat_simple_ok(MODEL, Some(Check::REASONING_CONTENT)).await
}

#[tokio::test]
async fn test_chat_multi_system_ok() -> TestResult<()> {
	common_tests::common_test_chat_multi_system_ok(MODEL).await
}

/// NOTE: deepseek-reasonner does not support Json Output (2025-01-21)
// #[tokio::test]
// async fn test_chat_json_mode_ok() -> TestResult<()> {
// 	common_tests::common_test_chat_json_mode_ok(MODEL, true).await
// }

#[tokio::test]
async fn test_chat_temperature_ok() -> TestResult<()> {
	common_tests::common_test_chat_temperature_ok(MODEL).await
}

#[tokio::test]
async fn test_chat_stop_sequences_ok() -> TestResult<()> {
	common_tests::common_test_chat_stop_sequences_ok(MODEL).await
}

#[tokio::test]
async fn test_chat_reasoning_normalize_ok() -> TestResult<()> {
	common_tests::common_test_chat_reasoning_normalize_ok(MODEL).await
}
// endregion: --- Chat

// region:    --- Chat Stream Tests

// NOTE: genai does not support deepseek-reasonner stream yet.

#[tokio::test]
async fn test_chat_stream_simple_ok() -> TestResult<()> {
	common_tests::common_test_chat_stream_simple_ok(MODEL, Some(Check::REASONING_CONTENT)).await
}

#[tokio::test]
async fn test_chat_stream_capture_content_ok() -> TestResult<()> {
	common_tests::common_test_chat_stream_capture_content_ok(MODEL).await
}

#[tokio::test]
async fn test_chat_stream_capture_all_ok() -> TestResult<()> {
	common_tests::common_test_chat_stream_capture_all_ok(MODEL, Some(Check::REASONING_CONTENT)).await
}

// endregion: --- Chat Stream Tests

// region:    --- Resolver Tests

#[tokio::test]
async fn test_resolver_auth_ok() -> TestResult<()> {
	common_tests::common_test_resolver_auth_ok(MODEL, AuthData::from_env("DEEPSEEK_API_KEY")).await
}

// endregion: --- Resolver Tests

// region:    --- List

#[tokio::test]
async fn test_list_models() -> TestResult<()> {
	common_tests::common_test_list_models(AdapterKind::DeepSeek, "deepseek-chat").await
}

// endregion: --- List

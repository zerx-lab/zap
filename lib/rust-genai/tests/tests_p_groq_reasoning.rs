mod support;

use crate::support::{TestResult, common_tests};
use genai::adapter::AdapterKind;
use genai::resolver::AuthData;

// Note: In groq, the llama3.1 or gemma models fail to produce JSON without a proposed schema.
//       With the "tool-use" groq version, it will work correctly.
const MODEL: &str = "deepseek-r1-distill-llama-70b";

// region:    --- Chat

#[tokio::test]
async fn test_chat_simple_ok() -> TestResult<()> {
	// NOTE: For now, the Ollama Deepseek Distilled model does not add .reasoning_content,
	//       but has a <think> tag which is tested in test_chat_reasoning_normalize_ok.
	common_tests::common_test_chat_simple_ok(MODEL, None).await
}

#[tokio::test]
async fn test_chat_multi_system_ok() -> TestResult<()> {
	common_tests::common_test_chat_multi_system_ok(MODEL).await
}

// THis model does not seem to support json mode
// #[tokio::test]
// async fn test_chat_json_mode_ok() -> TestResult<()> {
// 	common_tests::common_test_chat_json_mode_ok(MODEL, Some(Check::USAGE)).await
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
	// NOTE: At this point, genai does not capture the <think> while streaming
	common_tests::common_test_chat_stream_capture_all_ok(MODEL, None).await
}

// endregion: --- Chat Stream Tests

// region:    --- Resolver Tests

#[tokio::test]
async fn test_resolver_auth_ok() -> TestResult<()> {
	common_tests::common_test_resolver_auth_ok(MODEL, AuthData::from_env("GROQ_API_KEY")).await
}

// endregion: --- Resolver Tests

// region:    --- List

#[tokio::test]
async fn test_list_models() -> TestResult<()> {
	common_tests::common_test_list_models(AdapterKind::Groq, "llama-3.1-70b-versatile").await
}

// endregion: --- List

mod support;

use crate::support::{Check, TestResult, common_tests};
use genai::adapter::AdapterKind;
use genai::resolver::AuthData;
use serial_test::serial;

// Vertex AI requires namespace-prefixed model names.
// Env vars needed: VERTEX_PROJECT_ID, VERTEX_LOCATION, VERTEX_API_KEY (Bearer token)

// -- Gemini on Vertex (publishers/google)
const MODEL_GEMINI: &str = "vertex::gemini-2.5-flash";

// -- Claude on Vertex / Model Garden (publishers/anthropic)
const MODEL_CLAUDE: &str = "vertex::claude-sonnet-4-6";

// region:    --- Chat (Gemini)

#[tokio::test]
#[serial(vertex)]
async fn test_chat_gemini_simple_ok() -> TestResult<()> {
	common_tests::common_test_chat_simple_ok(MODEL_GEMINI, None).await
}

#[tokio::test]
#[serial(vertex)]
async fn test_chat_gemini_multi_system_ok() -> TestResult<()> {
	common_tests::common_test_chat_multi_system_ok(MODEL_GEMINI).await
}

#[tokio::test]
#[serial(vertex)]
async fn test_chat_gemini_json_structured_ok() -> TestResult<()> {
	common_tests::common_test_chat_json_structured_ok(MODEL_GEMINI, Some(Check::USAGE)).await
}

#[tokio::test]
#[serial(vertex)]
async fn test_chat_gemini_temperature_ok() -> TestResult<()> {
	common_tests::common_test_chat_temperature_ok(MODEL_GEMINI).await
}

#[tokio::test]
#[serial(vertex)]
async fn test_chat_gemini_stop_sequences_ok() -> TestResult<()> {
	common_tests::common_test_chat_stop_sequences_ok(MODEL_GEMINI).await
}

// endregion: --- Chat (Gemini)

// region:    --- Chat (Claude)

#[tokio::test]
#[serial(vertex)]
async fn test_chat_claude_simple_ok() -> TestResult<()> {
	common_tests::common_test_chat_simple_ok(MODEL_CLAUDE, None).await
}

#[tokio::test]
#[serial(vertex)]
async fn test_chat_claude_multi_system_ok() -> TestResult<()> {
	common_tests::common_test_chat_multi_system_ok(MODEL_CLAUDE).await
}

#[tokio::test]
#[serial(vertex)]
async fn test_chat_claude_temperature_ok() -> TestResult<()> {
	common_tests::common_test_chat_temperature_ok(MODEL_CLAUDE).await
}

#[tokio::test]
#[serial(vertex)]
async fn test_chat_claude_stop_sequences_ok() -> TestResult<()> {
	common_tests::common_test_chat_stop_sequences_ok(MODEL_CLAUDE).await
}

// endregion: --- Chat (Claude)

// region:    --- Chat Stream (Gemini)

#[tokio::test]
#[serial(vertex)]
async fn test_chat_stream_gemini_simple_ok() -> TestResult<()> {
	common_tests::common_test_chat_stream_simple_ok(MODEL_GEMINI, None).await
}

#[tokio::test]
#[serial(vertex)]
async fn test_chat_stream_gemini_capture_content_ok() -> TestResult<()> {
	common_tests::common_test_chat_stream_capture_content_ok(MODEL_GEMINI).await
}

#[tokio::test]
#[serial(vertex)]
async fn test_chat_stream_gemini_capture_all_ok() -> TestResult<()> {
	common_tests::common_test_chat_stream_capture_all_ok(MODEL_GEMINI, None).await
}

// endregion: --- Chat Stream (Gemini)

// region:    --- Chat Stream (Claude)

#[tokio::test]
#[serial(vertex)]
async fn test_chat_stream_claude_simple_ok() -> TestResult<()> {
	common_tests::common_test_chat_stream_simple_ok(MODEL_CLAUDE, None).await
}

#[tokio::test]
#[serial(vertex)]
async fn test_chat_stream_claude_capture_content_ok() -> TestResult<()> {
	common_tests::common_test_chat_stream_capture_content_ok(MODEL_CLAUDE).await
}

#[tokio::test]
#[serial(vertex)]
async fn test_chat_stream_claude_capture_all_ok() -> TestResult<()> {
	common_tests::common_test_chat_stream_capture_all_ok(MODEL_CLAUDE, None).await
}

// endregion: --- Chat Stream (Claude)

// region:    --- Tool Tests (Gemini)

#[tokio::test]
#[serial(vertex)]
async fn test_tool_gemini_simple_ok() -> TestResult<()> {
	common_tests::common_test_tool_simple_ok(MODEL_GEMINI).await
}

#[tokio::test]
#[serial(vertex)]
async fn test_tool_gemini_full_flow_ok() -> TestResult<()> {
	common_tests::common_test_tool_full_flow_ok(MODEL_GEMINI).await
}

// endregion: --- Tool Tests (Gemini)

// region:    --- Tool Tests (Claude)

#[tokio::test]
#[serial(vertex)]
async fn test_tool_claude_simple_ok() -> TestResult<()> {
	common_tests::common_test_tool_simple_ok(MODEL_CLAUDE).await
}

#[tokio::test]
#[serial(vertex)]
async fn test_tool_claude_full_flow_ok() -> TestResult<()> {
	common_tests::common_test_tool_full_flow_ok(MODEL_CLAUDE).await
}

// endregion: --- Tool Tests (Claude)

// region:    --- Resolver Tests

#[tokio::test]
#[serial(vertex)]
async fn test_resolver_auth_ok() -> TestResult<()> {
	common_tests::common_test_resolver_auth_ok(MODEL_GEMINI, AuthData::from_env("VERTEX_API_KEY")).await
}

// endregion: --- Resolver Tests

// region:    --- List

#[tokio::test]
async fn test_list_models() -> TestResult<()> {
	common_tests::common_test_list_models(AdapterKind::Vertex, "gemini-2.5-flash").await
}

// endregion: --- List

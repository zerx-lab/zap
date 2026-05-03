mod support;

use crate::support::{TestResult, common_tests};
use genai::adapter::AdapterKind;
use genai::resolver::AuthData;
use serial_test::serial;

const MODEL: &str = "command-r7b-12-2024";
const MODEL_NS: &str = "cohere::command-r7b-12-2024";

// region:    --- Chat

#[tokio::test]
#[serial(cohere)]
async fn test_chat_simple_ok() -> TestResult<()> {
	common_tests::common_test_chat_simple_ok(MODEL, None).await
}

#[tokio::test]
#[serial(cohere)]
async fn test_chat_namespaced_ok() -> TestResult<()> {
	common_tests::common_test_chat_simple_ok(MODEL_NS, None).await
}

#[tokio::test]
#[serial(cohere)]
async fn test_chat_multi_system_ok() -> TestResult<()> {
	common_tests::common_test_chat_multi_system_ok(MODEL).await
}

#[tokio::test]
#[serial(cohere)]
async fn test_chat_stop_sequences_ok() -> TestResult<()> {
	common_tests::common_test_chat_stop_sequences_ok(MODEL).await
}

// endregion: --- Chat

// region:    --- Chat Stream Tests

#[tokio::test]
#[serial(cohere)]
async fn test_chat_stream_simple_ok() -> TestResult<()> {
	common_tests::common_test_chat_stream_simple_ok(MODEL, None).await
}

// NOTE 2024-06-23 - Occasionally, the last stream message sent by Cohere is malformed and cannot be parsed.
//                   Will investigate further if requested.
// #[tokio::test]
#[serial(cohere)]
// async fn test_chat_stream_capture_content_ok() -> TestResult<()> {
// 	common_tests::common_test_chat_stream_capture_content_ok(MODEL).await
// }
#[tokio::test]
#[serial(cohere)]
async fn test_chat_stream_capture_all_ok() -> TestResult<()> {
	common_tests::common_test_chat_stream_capture_all_ok(MODEL, None).await
}

#[tokio::test]
#[serial(cohere)]
async fn test_chat_temperature_ok() -> TestResult<()> {
	common_tests::common_test_chat_temperature_ok(MODEL).await
}

// endregion: --- Chat Stream Tests

// region:    --- Resolver Tests

#[tokio::test]
#[serial(cohere)]
async fn test_resolver_auth_ok() -> TestResult<()> {
	common_tests::common_test_resolver_auth_ok(MODEL, AuthData::from_env("COHERE_API_KEY")).await
}

// endregion: --- Resolver Tests

// region:    --- List

#[tokio::test]
#[serial(cohere)]
async fn test_list_models() -> TestResult<()> {
	common_tests::common_test_list_models(AdapterKind::Cohere, "command-r-plus").await
}

// endregion: --- List

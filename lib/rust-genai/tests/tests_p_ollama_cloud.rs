mod support;

use crate::support::{TestResult, common_tests};
use genai::adapter::AdapterKind;
use genai::resolver::AuthData;

const MODEL: &str = "ollama_cloud::gpt-oss:120b";

#[tokio::test]
async fn test_chat_simple_ok() -> TestResult<()> {
	if std::env::var("OLLAMA_API_KEY").is_err() {
		println!("Skipping test_chat_simple_ok: OLLAMA_API_KEY not set");
		return Ok(());
	}
	common_tests::common_test_chat_simple_ok(MODEL, None).await
}

#[tokio::test]
async fn test_chat_stream_simple_ok() -> TestResult<()> {
	if std::env::var("OLLAMA_API_KEY").is_err() {
		println!("Skipping test_chat_stream_simple_ok: OLLAMA_API_KEY not set");
		return Ok(());
	}
	common_tests::common_test_chat_stream_simple_ok(MODEL, None).await
}

#[tokio::test]
async fn test_chat_stream_capture_content_ok() -> TestResult<()> {
	if std::env::var("OLLAMA_API_KEY").is_err() {
		println!("Skipping test_chat_stream_capture_content_ok: OLLAMA_API_KEY not set");
		return Ok(());
	}
	common_tests::common_test_chat_stream_capture_content_ok(MODEL).await
}

#[tokio::test]
async fn test_resolver_auth_ok() -> TestResult<()> {
	if std::env::var("OLLAMA_API_KEY").is_err() {
		println!("Skipping test_resolver_auth_ok: OLLAMA_API_KEY not set");
		return Ok(());
	}
	common_tests::common_test_resolver_auth_ok(MODEL, AuthData::from_env("OLLAMA_API_KEY")).await
}

#[tokio::test]
async fn test_list_models() -> TestResult<()> {
	if std::env::var("OLLAMA_API_KEY").is_err() {
		println!("Skipping test_list_models: OLLAMA_API_KEY not set");
		return Ok(());
	}
	common_tests::common_test_list_models(AdapterKind::OllamaCloud, "gpt-oss:120b").await
}

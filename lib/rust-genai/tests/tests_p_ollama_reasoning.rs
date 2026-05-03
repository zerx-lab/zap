mod support;

use crate::support::{TestResult, common_tests, seed_chat_req_simple};
use genai::Client;
use genai::adapter::AdapterKind;
use genai::chat::ChatStreamEvent;
use genai::resolver::AuthData;
use serial_test::serial;
use tokio_stream::StreamExt;

// NOTE: Sometimes the 1.5b model does not provide the reasoning or has some issues.
//       Rerunning the test or switching to the 8b model would generally solve the issues.

// NOTE: Also, #[serial(ollama)] seems more reliable when using it.

const MODEL: &str = "deepseek-r1:1.5b"; // "deepseek-r1:8b" "deepseek-r1:1.5b"
const MODEL_QWEN3: &str = "qwen3:4b";

// region:    --- Chat

#[tokio::test]
#[serial(ollama)]
async fn test_chat_simple_ok() -> TestResult<()> {
	// NOTE: For now, the Ollama Deepseek Distilled model does not add .reasoning_content,
	//       but has a <think> tag which is tested in test_chat_reasoning_normalize_ok.
	common_tests::common_test_chat_simple_ok(MODEL, None).await
}

#[tokio::test]
#[serial(ollama)]
async fn test_chat_multi_system_ok() -> TestResult<()> {
	common_tests::common_test_chat_multi_system_ok(MODEL).await
}

#[tokio::test]
#[serial(ollama)]
async fn test_chat_json_mode_ok() -> TestResult<()> {
	// Note: Ollama does not capture usage on json mode (TODO: need to check now (2025-02-02))
	common_tests::common_test_chat_json_mode_ok(MODEL, None).await
}

#[tokio::test]
#[serial(ollama)]
async fn test_chat_temperature_ok() -> TestResult<()> {
	common_tests::common_test_chat_temperature_ok(MODEL).await
}

#[tokio::test]
#[serial(ollama)]
async fn test_chat_stop_sequences_ok() -> TestResult<()> {
	common_tests::common_test_chat_stop_sequences_ok(MODEL).await
}

/// Note: Unfortunately, sometime, the 1.5b does not provide reasoning.
#[tokio::test]
#[serial(ollama)]
async fn test_chat_reasoning_normalize_ok() -> TestResult<()> {
	common_tests::common_test_chat_reasoning_normalize_ok(MODEL).await
}
// endregion: --- Chat

// region:    --- Chat Stream Tests

#[tokio::test]
#[serial(ollama)]
async fn test_chat_stream_simple_ok() -> TestResult<()> {
	common_tests::common_test_chat_stream_simple_ok(MODEL, None).await
}

#[tokio::test]
#[serial(ollama)]
async fn test_chat_stream_capture_content_ok() -> TestResult<()> {
	common_tests::common_test_chat_stream_capture_content_ok(MODEL).await
}

#[tokio::test]
#[serial(ollama)]
async fn test_chat_stream_reasoning_chunk_ok() -> TestResult<()> {
	let client = Client::default();
	let chat_req = seed_chat_req_simple();

	let chat_res = client.exec_chat_stream(MODEL_QWEN3, chat_req, None).await?;
	let mut stream = chat_res.stream;
	let mut reasoning_content = String::new();

	while let Some(result) = stream.next().await {
		match result? {
			ChatStreamEvent::ReasoningChunk(chunk) => {
				reasoning_content.push_str(&chunk.content);
				break;
			}
			ChatStreamEvent::End(_) => break,
			_ => {}
		}
	}
	assert!(!reasoning_content.is_empty(), "reasoning_content should not be empty");

	Ok(())
}

#[tokio::test]
#[serial(ollama)]
async fn test_chat_stream_non_empty_chunk_deepseek_ok() -> TestResult<()> {
	let client = Client::default();
	let chat_req = seed_chat_req_simple();

	let chat_res = client.exec_chat_stream(MODEL, chat_req, None).await?;
	let mut stream = chat_res.stream;
	let mut found_non_empty = false;

	while let Some(result) = stream.next().await {
		match result? {
			ChatStreamEvent::Chunk(chunk) if !chunk.content.is_empty() => {
				found_non_empty = true;
				break;
			}
			ChatStreamEvent::ReasoningChunk(chunk) if !chunk.content.is_empty() => {
				found_non_empty = true;
				break;
			}
			ChatStreamEvent::End(_) => break,
			_ => {}
		}
	}

	assert!(
		found_non_empty,
		"stream should yield non-empty content or reasoning chunks"
	);

	Ok(())
}

// /// COMMENTED FOR NOW AS OLLAMA OpenAI Compatibility Layer does not support
// /// usage tokens when streaming. See https://github.com/ollama/ollama/issues/4448
// #[tokio::test]
// async fn test_chat_stream_capture_all_ok() -> TestResult<()> {
// 	common_tests::common_test_chat_stream_capture_all_ok(MODEL).await
// }

// endregion: --- Chat Stream Tests

// region:    --- Resolver Tests

#[tokio::test]
#[serial(ollama)]
async fn test_resolver_auth_ok() -> TestResult<()> {
	common_tests::common_test_resolver_auth_ok(MODEL, AuthData::from_single("ollama")).await
}

// endregion: --- Resolver Tests

// region:    --- List

/// NOTE this test assume the "gemma3:4b" is present.
#[tokio::test]
#[serial(ollama)]
async fn test_list_models() -> TestResult<()> {
	common_tests::common_test_list_models(AdapterKind::Ollama, "gemma3:4b").await
}

// endregion: --- List

//! Replay integration tests for the Ollama Cloud adapter.
//!
//! These tests use pre-recorded cassettes from `tests/data/yakbak/ollama_cloud/`
//! and assert that Ollama-native streaming content and stop reasons replay
//! correctly for the hosted Ollama Cloud backend.

mod support;

use genai::chat::*;
use support::yakbak::replay_client;
use support::{TestResult, extract_stream_end};

#[tokio::test]
async fn test_yakbak_ollama_cloud_simple_stream() -> TestResult<()> {
	let (client, _server) = replay_client("ollama_cloud", "simple_stream").await?;

	let chat_req = ChatRequest::new(vec![
		ChatMessage::system("Answer in one sentence."),
		ChatMessage::user("Why is the sky blue?"),
	]);
	let options = ChatOptions::default().with_capture_content(true).with_capture_usage(true);

	let stream_res = client
		.exec_chat_stream("ollama_cloud::gemma3:4b", chat_req, Some(&options))
		.await?;
	let extract = extract_stream_end(stream_res.stream).await?;

	assert_eq!(
		extract.content.as_deref(),
		Some(
			"The sky appears blue due to a phenomenon called Rayleigh scattering, where shorter wavelengths of sunlight (blue and violet) are scattered more by the Earth’s atmosphere than longer wavelengths like red and orange."
		),
		"Text should match recorded response exactly"
	);

	assert_eq!(
		extract.stream_end.captured_stop_reason,
		Some(StopReason::Completed("stop".to_string()))
	);

	Ok(())
}

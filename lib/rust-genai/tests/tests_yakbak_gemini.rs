//! Replay integration tests for the Gemini adapter.
//!
//! These tests use pre-recorded cassettes from `tests/data/yakbak/gemini/`
//! and assert that thinking content, tool calls, and usage flow through correctly.

mod support;

use genai::chat::*;
use support::yakbak::replay_client;
use support::{TestResult, extract_stream_end};

#[tokio::test]
async fn test_yakbak_gemini_thinking_stream() -> TestResult<()> {
	let (client, _server) = replay_client("gemini", "thinking_stream").await?;

	let chat_req = ChatRequest::new(vec![
		ChatMessage::system("Answer in one sentence."),
		ChatMessage::user("Why is the sky blue?"),
	]);
	let options = ChatOptions::default()
		.with_reasoning_effort(ReasoningEffort::Low)
		.with_capture_content(true)
		.with_capture_reasoning_content(true)
		.with_capture_usage(true);

	let stream_res = client.exec_chat_stream("gemini-2.5-flash", chat_req, Some(&options)).await?;
	let extract = extract_stream_end(stream_res.stream).await?;

	// Exact text
	assert_eq!(
		extract.content.as_deref(),
		Some(
			"The sky is blue because molecules in Earth's atmosphere scatter shorter-wavelength blue light from the sun more efficiently than longer-wavelength red light."
		),
	);

	// Reasoning should be substantial (862 chars from recorded)
	let reasoning = extract.reasoning_content.as_deref().ok_or("Should have reasoning")?;
	assert_eq!(reasoning.len(), 862, "reasoning length should be exactly 862 chars");
	assert!(reasoning.starts_with("**Defining Sky Color**"));

	// Exact usage
	let usage = extract.stream_end.captured_usage.as_ref().ok_or("Should have usage")?;
	assert_eq!(usage.prompt_tokens, Some(12));
	assert_eq!(usage.completion_tokens, Some(230));

	Ok(())
}

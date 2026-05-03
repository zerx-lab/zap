//! Replay integration tests for the GitHub Copilot adapter.
//!
//! These tests use pre-recorded cassettes from `tests/data/yakbak/github_copilot/`
//! and assert that content and tool calls flow through correctly. GitHub Copilot
//! supports multiple publishers, but these cassettes stick to `openai/gpt-4.1-mini`
//! for deterministic replay coverage.
//!
//! GitHub Copilot uses the OpenAI Chat Completions protocol via the GitHub
//! Models inference API, so cassettes are in standard `data: {...}` SSE format.
//! The GitHub Models API does NOT return actual usage tokens in streaming
//! responses, but may emit a `prompt_filter_results` message with empty choices
//! that causes the streamer to capture a default (all-None) Usage struct. Other
//! publishers are supported by the adapter too, but are not recorded here.

mod support;

use genai::chat::*;
use serde_json::json;
use support::yakbak::replay_client;
use support::{TestResult, extract_stream_end};

#[tokio::test]
async fn test_yakbak_github_copilot_simple_stream() -> TestResult<()> {
	let (client, _server) = replay_client("github_copilot", "simple_stream").await?;

	let chat_req = ChatRequest::new(vec![
		ChatMessage::system("Answer in one sentence"),
		ChatMessage::user("Why is the sky blue?"),
	]);
	let options = ChatOptions::default().with_capture_content(true).with_capture_usage(true);

	let stream_res = client
		.exec_chat_stream("github_copilot::openai/gpt-4.1-mini", chat_req, Some(&options))
		.await?;
	let extract = extract_stream_end(stream_res.stream).await?;

	// Exact text content
	assert_eq!(
		extract.content.as_deref(),
		Some(
			"The sky is blue because molecules in the Earth's atmosphere scatter shorter blue wavelengths of sunlight more than longer red wavelengths."
		),
		"Text should match recorded response exactly"
	);

	let usage = extract.stream_end.captured_usage.as_ref();
	if let Some(usage) = usage {
		assert!(usage.prompt_tokens.is_none());
		assert!(usage.completion_tokens.is_none());
		assert!(usage.total_tokens.is_none());
	}

	Ok(())
}

#[tokio::test]
async fn test_yakbak_github_copilot_tool_stream() -> TestResult<()> {
	let (client, _server) = replay_client("github_copilot", "tool_stream").await?;

	let chat_req = ChatRequest::new(vec![
		ChatMessage::system("You are a helpful assistant. Use tools when needed."),
		ChatMessage::user("What is the temperature in C and weather, in Paris, France"),
	])
	.append_tool(Tool::new("get_weather").with_schema(json!({
		"type": "object",
		"properties": {
			"city": { "type": "string", "description": "The city name" },
			"country": { "type": "string", "description": "The most likely country of this city name" },
			"unit": { "type": "string", "enum": ["C", "F"], "description": "Temperature unit" }
		},
		"required": ["city", "country", "unit"],
	})));

	let options = ChatOptions::default()
		.with_capture_content(true)
		.with_capture_tool_calls(true)
		.with_capture_usage(true);

	let stream_res = client
		.exec_chat_stream("github_copilot::openai/gpt-4.1-mini", chat_req, Some(&options))
		.await?;
	let extract = extract_stream_end(stream_res.stream).await?;

	// Verify incremental ToolCallChunk events from the OpenAI-compatible stream.
	let chunks = &extract.tool_call_chunks;
	assert!(
		chunks.len() >= 2,
		"Should have at least 2 tool call chunks (start + deltas), got {}",
		chunks.len()
	);

	let first = &chunks[0];
	assert_eq!(first.fn_name, "get_weather", "First chunk should have tool name");
	assert_eq!(
		first.fn_arguments.as_str(),
		Some(""),
		"First chunk should have empty string args"
	);
	assert!(
		!first.call_id.is_empty(),
		"First chunk should include a non-empty provider call id"
	);

	let last = chunks.last().ok_or("Should have a final tool call chunk")?;
	assert_eq!(last.fn_name, "get_weather");
	let last_args_str = last.fn_arguments.as_str().ok_or("Args should stream as strings")?;
	assert!(
		last_args_str.contains("Paris"),
		"Final accumulated args should contain 'Paris', got: {last_args_str}"
	);
	assert!(
		last_args_str.contains("France"),
		"Final accumulated args should contain 'France', got: {last_args_str}"
	);
	assert!(
		last_args_str.contains("C"),
		"Final accumulated args should contain 'C', got: {last_args_str}"
	);

	// Exactly one tool call
	let tool_calls = extract.stream_end.captured_tool_calls().ok_or("Should have tool calls")?;
	assert_eq!(tool_calls.len(), 1);

	// Exact tool call details
	let tc = &tool_calls[0];
	assert_eq!(tc.fn_name, "get_weather");
	assert_eq!(
		tc.fn_arguments,
		json!({"city": "Paris", "country": "France", "unit": "C"})
	);
	assert!(!tc.call_id.is_empty(), "Tool call id should be non-empty");
	assert_eq!(
		tc.call_id, first.call_id,
		"StreamEnd should preserve the streamed call id"
	);

	let usage = extract.stream_end.captured_usage.as_ref();
	if let Some(usage) = usage {
		assert!(usage.prompt_tokens.is_none());
		assert!(usage.completion_tokens.is_none());
		assert!(usage.total_tokens.is_none());
	}

	Ok(())
}

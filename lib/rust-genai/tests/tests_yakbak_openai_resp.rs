//! Replay integration tests for the OpenAI Responses API adapter.
//!
//! These tests use pre-recorded cassettes from `tests/data/yakbak/openai_resp/`
//! and assert that content and tool calls flow through correctly.

mod support;

use genai::chat::*;
use serde_json::json;
use support::yakbak::replay_client;
use support::{TestResult, extract_stream_end};

#[tokio::test]
async fn test_yakbak_openai_resp_reasoning_summary_capture() -> TestResult<()> {
	// Regression for the two-part fix applied in
	// `adapter_impl.rs` (request side inserts
	// `reasoning.summary="detailed"` when
	// `capture_reasoning_content=true`) and `streamer.rs` (parses
	// `response.reasoning_summary_text.delta` events emitted by the
	// Responses API). The cassette was recorded against the real
	// API with both halves present — if either regresses, this
	// test's `reasoning_content` assertion starts failing.
	let (client, _server) = replay_client("openai_resp", "reasoning_summary_capture").await?;

	let chat_req = ChatRequest::new(vec![
		ChatMessage::system("Answer concisely."),
		ChatMessage::user("Why is 47 * 23 = 1081? Reason step by step."),
	]);
	let options = ChatOptions::default()
		.with_reasoning_effort(ReasoningEffort::Low)
		.with_capture_content(true)
		.with_capture_reasoning_content(true);

	let stream_res = client
		.exec_chat_stream("openai_resp::gpt-5.4-mini", chat_req, Some(&options))
		.await?;
	let extract = extract_stream_end(stream_res.stream).await?;

	let reasoning = extract
		.reasoning_content
		.as_deref()
		.ok_or("reasoning_content should be populated")?;
	assert!(
		!reasoning.is_empty(),
		"reasoning_content populated but empty"
	);
	// First summary chunk for the recorded prompt starts with a
	// header line we can pin — the API formats summaries as
	// "**Topic**\n\nFirst line..." and the exact header is stable
	// for a fixed prompt.
	assert!(
		reasoning.starts_with("**"),
		"reasoning_content should start with a summary header, got: {:?}",
		&reasoning[..reasoning.len().min(60)]
	);

	Ok(())
}

#[tokio::test]
async fn test_yakbak_openai_resp_reasoning_stream() -> TestResult<()> {
	let (client, _server) = replay_client("openai_resp", "reasoning_stream").await?;

	let chat_req = ChatRequest::new(vec![
		ChatMessage::system("Answer in one sentence."),
		ChatMessage::user("Why is the sky blue?"),
	]);
	let options = ChatOptions::default()
		.with_reasoning_effort(ReasoningEffort::Low)
		.with_capture_content(true)
		.with_capture_reasoning_content(true)
		.with_capture_usage(true);

	let stream_res = client
		.exec_chat_stream("openai_resp::gpt-5.4-mini", chat_req, Some(&options))
		.await?;
	let extract = extract_stream_end(stream_res.stream).await?;

	// Exact text content
	assert_eq!(
		extract.content.as_deref(),
		Some(
			"The sky looks blue because molecules in Earth\u{2019}s atmosphere scatter shorter blue wavelengths of sunlight more strongly than longer red wavelengths, sending more blue light to our eyes."
		),
		"Text should match recorded response exactly"
	);

	// Exact usage
	let usage = extract.stream_end.captured_usage.as_ref().ok_or("Should have usage")?;
	assert_eq!(usage.prompt_tokens, Some(21));
	assert_eq!(usage.completion_tokens, Some(45));
	assert_eq!(usage.total_tokens, Some(66));

	// Encrypted reasoning content should be captured as thought signatures
	let thought_sigs = extract
		.stream_end
		.captured_thought_signatures()
		.ok_or("Should have thought signatures (encrypted reasoning content)")?;
	assert_eq!(thought_sigs.len(), 1, "Should have exactly one thought signature");
	assert!(
		thought_sigs[0].len() > 100,
		"Encrypted content should be substantial, got {} bytes",
		thought_sigs[0].len()
	);

	Ok(())
}

#[tokio::test]
async fn test_yakbak_openai_resp_stream_tools() -> TestResult<()> {
	let (client, _server) = replay_client("openai_resp", "reasoning_stream_tools").await?;

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
		.with_reasoning_effort(ReasoningEffort::Low)
		.with_capture_content(true)
		.with_capture_reasoning_content(true)
		.with_capture_tool_calls(true)
		.with_capture_usage(true);

	let stream_res = client
		.exec_chat_stream("openai_resp::gpt-5.4-mini", chat_req, Some(&options))
		.await?;
	let extract = extract_stream_end(stream_res.stream).await?;

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
	assert!(!tc.call_id.is_empty());

	// Exact usage
	let usage = extract.stream_end.captured_usage.as_ref().ok_or("Should have usage")?;
	assert_eq!(usage.prompt_tokens, Some(106));
	assert_eq!(usage.completion_tokens, Some(45));
	assert_eq!(usage.total_tokens, Some(151));

	Ok(())
}

/// Demonstrates and verifies the UTF-8 chunking fix in WebStream::poll_next().
///
/// When a multi-byte UTF-8 character (e.g. Japanese 日 = 3 bytes) straddles an HTTP
/// chunk boundary, the old code's `String::from_utf8()` would fail. The fix buffers
/// incomplete trailing bytes across chunks.
///
/// This tape has Japanese text positioned at byte offset 8191 (8KB boundary).
#[tokio::test]
async fn test_yakbak_openai_resp_utf8_chunking_bug() -> TestResult<()> {
	let (client, _server) = replay_client("openai_resp", "utf8_chunking_bug").await?;

	let chat_req = ChatRequest::new(vec![ChatMessage::user("Say something in Japanese.")]);
	let options = ChatOptions::default().with_capture_content(true).with_capture_usage(true);

	let stream_res = client
		.exec_chat_stream("openai_resp::o3-mini", chat_req, Some(&options))
		.await?;
	let extract = extract_stream_end(stream_res.stream).await?;

	// The content should contain Japanese characters intact
	let content = extract.content.ok_or("Should have streamed content")?;
	assert!(
		content.contains("日本語のテスト"),
		"Content should contain Japanese text, got: {}...",
		&content[content.len().saturating_sub(100)..]
	);

	// Usage should be captured
	let usage = extract.stream_end.captured_usage.as_ref().ok_or("Should have usage")?;
	assert_eq!(usage.prompt_tokens, Some(10));
	assert_eq!(usage.completion_tokens, Some(50));
	assert_eq!(usage.total_tokens, Some(60));

	Ok(())
}

/// Regression test: tool calls from a streamed response must be returned in the
/// same order as their `output_index` from the SSE stream.
///
/// Before the fix, `HashMap::drain()` yielded entries in arbitrary hash-bucket
/// order, so the final `Vec<ToolCall>` could be shuffled across runs even though
/// the input stream was identical.  Replacing `HashMap` with `BTreeMap` (keyed by
/// `output_index`) guarantees sorted iteration order.
///
/// This cassette contains 3 parallel tool calls at output indices 1, 2, 3:
///   1 → get_weather(city=Paris, country=France, unit=C)
///   2 → get_time(timezone=Europe/Paris)
///   3 → get_currency(from=USD, to=EUR, amount=100)
#[tokio::test]
async fn test_yakbak_openai_resp_multi_tool_ordering() -> TestResult<()> {
	let (client, _server) = replay_client("openai_resp", "multi_tool_ordering").await?;

	let chat_req = ChatRequest::new(vec![
		ChatMessage::system("You are a helpful assistant. Use tools when needed."),
		ChatMessage::user("What's the weather, time, and USD→EUR rate for Paris?"),
	])
	.append_tool(Tool::new("get_weather").with_schema(json!({
		"type": "object",
		"properties": {
			"city":    { "type": "string" },
			"country": { "type": "string" },
			"unit":    { "type": "string", "enum": ["C", "F"] }
		},
		"required": ["city", "country", "unit"]
	})))
	.append_tool(Tool::new("get_time").with_schema(json!({
		"type": "object",
		"properties": {
			"timezone": { "type": "string" }
		},
		"required": ["timezone"]
	})))
	.append_tool(Tool::new("get_currency").with_schema(json!({
		"type": "object",
		"properties": {
			"from":   { "type": "string" },
			"to":     { "type": "string" },
			"amount": { "type": "number" }
		},
		"required": ["from", "to", "amount"]
	})));

	let options = ChatOptions::default()
		.with_reasoning_effort(ReasoningEffort::Low)
		.with_capture_tool_calls(true)
		.with_capture_usage(true);

	let stream_res = client
		.exec_chat_stream("openai_resp::gpt-5.4-mini", chat_req, Some(&options))
		.await?;
	let extract = extract_stream_end(stream_res.stream).await?;

	// -- Exactly 3 tool calls
	let tool_calls = extract.stream_end.captured_tool_calls().ok_or("Should have tool calls")?;
	assert_eq!(tool_calls.len(), 3, "Expected exactly 3 tool calls");

	// -- Tool call 0: get_weather (output_index 1)
	assert_eq!(tool_calls[0].fn_name, "get_weather");
	assert_eq!(tool_calls[0].call_id, "call_weather_001");
	assert_eq!(
		tool_calls[0].fn_arguments,
		json!({"city": "Paris", "country": "France", "unit": "C"})
	);

	// -- Tool call 1: get_time (output_index 2)
	assert_eq!(tool_calls[1].fn_name, "get_time");
	assert_eq!(tool_calls[1].call_id, "call_time_002");
	assert_eq!(tool_calls[1].fn_arguments, json!({"timezone": "Europe/Paris"}));

	// -- Tool call 2: get_currency (output_index 3)
	assert_eq!(tool_calls[2].fn_name, "get_currency");
	assert_eq!(tool_calls[2].call_id, "call_currency_003");
	assert_eq!(
		tool_calls[2].fn_arguments,
		json!({"from": "USD", "to": "EUR", "amount": 100})
	);

	// -- Usage
	let usage = extract.stream_end.captured_usage.as_ref().ok_or("Should have usage")?;
	assert_eq!(usage.prompt_tokens, Some(150));
	assert_eq!(usage.completion_tokens, Some(80));
	assert_eq!(usage.total_tokens, Some(230));

	Ok(())
}

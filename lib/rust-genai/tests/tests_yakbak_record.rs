//! Recording scripts for yakbak cassettes.
//!
//! These are `#[ignore]` tests — run manually with real API keys.
//! Each provider's keys and base URLs are independent; you only need
//! the credentials for the provider(s) you want to record.
//!
//! ```sh
//! # Record all providers (need all keys):
//! OPENAI_API_KEY=... GEMINI_API_KEY=... GITHUB_TOKEN=... OLLAMA_API_KEY=... cargo test --test tests_yakbak_record -- --ignored
//!
//! # Record only Gemini scenarios:
//! GEMINI_API_KEY=... cargo test --test tests_yakbak_record -- --ignored record_gemini
//!
//! # Record only OpenAI scenarios:
//! OPENAI_API_KEY=... cargo test --test tests_yakbak_record -- --ignored record_openai
//!
//! # Record only GitHub Copilot scenarios:
//! GITHUB_TOKEN=... cargo test --test tests_yakbak_record -- --ignored record_github_copilot
//!
//! # Record only Ollama Cloud scenarios:
//! OLLAMA_API_KEY=... cargo test --test tests_yakbak_record -- --ignored record_ollama_cloud
//!
//! # Record a single scenario by name:
//! GEMINI_API_KEY=... cargo test --test tests_yakbak_record -- --ignored record_gemini_thinking_stream
//! ```
//!
//! Optional env vars for custom endpoints: `OPENAI_BASE_URL`, `GEMINI_BASE_URL`, `GITHUB_COPILOT_BASE_URL`, `OLLAMA_CLOUD_BASE_URL`.
//!
//! Each test records a response cassette to `tests/data/yakbak/{provider}/{scenario}/`.

mod support;

use genai::chat::*;
use serde_json::json;
use support::yakbak::record_client;
use support::{TestResult, extract_stream_end};

fn openai_backend() -> String {
	std::env::var("OPENAI_BASE_URL").unwrap_or_else(|_| "https://api.openai.com/v1/".to_string())
}

const OPENAI_MODEL: &str = "openai_resp::gpt-5.4-mini";

#[tokio::test]
#[ignore]
async fn record_openai_resp_reasoning_stream() -> TestResult<()> {
	let (client, mut server) = record_client("openai_resp", "reasoning_stream", &openai_backend()).await?;

	let chat_req = ChatRequest::new(vec![
		ChatMessage::system("Answer in one sentence."),
		ChatMessage::user("Why is the sky blue?"),
	]);
	let options = ChatOptions::default()
		.with_reasoning_effort(ReasoningEffort::Low)
		.with_capture_content(true)
		.with_capture_reasoning_content(true);

	let stream_res = client.exec_chat_stream(OPENAI_MODEL, chat_req, Some(&options)).await?;
	let extract = extract_stream_end(stream_res.stream).await?;
	eprintln!(
		"[record] Stream content: {:?}",
		extract.content.as_deref().map(|s| &s[..s.len().min(80)])
	);
	eprintln!(
		"[record] Stream reasoning: {:?}",
		extract.reasoning_content.as_deref().map(|s| &s[..s.len().min(80)])
	);

	server.shutdown().await;
	Ok(())
}

#[tokio::test]
#[ignore]
async fn record_openai_resp_reasoning_summary_capture() -> TestResult<()> {
	// Regression for the two-part fix. Pairs effort=Low with
	// capture_reasoning_content=true so the API actually emits
	// summary deltas (effort alone with no capture gets no summary;
	// capture alone gets effort="none" server-default → no reasoning
	// at all; both required IN PRACTICE on current models). Once
	// emitted, the `response.reasoning_summary_text.delta` events
	// must land in `captured_reasoning_content` — previously the
	// streamer only parsed the `response.reasoning_text.delta`
	// family and silently dropped summaries.
	let (client, mut server) =
		record_client("openai_resp", "reasoning_summary_capture", &openai_backend()).await?;

	let chat_req = ChatRequest::new(vec![
		ChatMessage::system("Answer concisely."),
		ChatMessage::user("Why is 47 * 23 = 1081? Reason step by step."),
	]);
	let options = ChatOptions::default()
		.with_reasoning_effort(ReasoningEffort::Low)
		.with_capture_content(true)
		.with_capture_reasoning_content(true)
		.with_capture_usage(true);

	let stream_res = client.exec_chat_stream(OPENAI_MODEL, chat_req, Some(&options)).await?;
	let extract = extract_stream_end(stream_res.stream).await?;
	eprintln!(
		"[record] reasoning_summary_capture reasoning_content: {:?}",
		extract.reasoning_content.as_deref().map(|s| &s[..s.len().min(200)])
	);

	server.shutdown().await;
	Ok(())
}

#[tokio::test]
#[ignore]
async fn record_openai_resp_reasoning_stream_tools() -> TestResult<()> {
	let (client, mut server) = record_client("openai_resp", "reasoning_stream_tools", &openai_backend()).await?;

	let chat_req = seed_tool_request();
	let options = ChatOptions::default()
		.with_reasoning_effort(ReasoningEffort::Low)
		.with_capture_content(true)
		.with_capture_reasoning_content(true);

	let stream_res = client.exec_chat_stream(OPENAI_MODEL, chat_req, Some(&options)).await?;
	let extract = extract_stream_end(stream_res.stream).await?;
	eprintln!("[record] Stream reasoning: {:?}", extract.reasoning_content.is_some());
	let tool_calls = &extract.stream_end.captured_tool_calls();
	eprintln!("[record] Tool calls: {:?}", tool_calls.as_ref().map(|tc| tc.len()));

	server.shutdown().await;
	Ok(())
}

fn gemini_backend() -> String {
	std::env::var("GEMINI_BASE_URL").unwrap_or_else(|_| "https://generativelanguage.googleapis.com/v1beta/".to_string())
}

const GEMINI_MODEL: &str = "gemini-2.5-flash";

#[tokio::test]
#[ignore]
async fn record_gemini_thinking_stream() -> TestResult<()> {
	let (client, mut server) = record_client("gemini", "thinking_stream", &gemini_backend()).await?;

	let chat_req = ChatRequest::new(vec![
		ChatMessage::system("Answer in one sentence."),
		ChatMessage::user("Why is the sky blue?"),
	]);
	let options = ChatOptions::default()
		.with_reasoning_effort(ReasoningEffort::Low)
		.with_capture_content(true)
		.with_capture_reasoning_content(true);

	let stream_res = client.exec_chat_stream(GEMINI_MODEL, chat_req, Some(&options)).await?;
	let extract = extract_stream_end(stream_res.stream).await?;
	eprintln!(
		"[record] Stream content: {:?}",
		extract.content.as_deref().map(|s| &s[..s.len().min(80)])
	);
	eprintln!(
		"[record] Stream reasoning: {:?}",
		extract.reasoning_content.as_deref().map(|s| &s[..s.len().min(80)])
	);

	server.shutdown().await;
	Ok(())
}

fn github_copilot_backend() -> String {
	std::env::var("GITHUB_COPILOT_BASE_URL").unwrap_or_else(|_| "https://models.github.ai/inference/".to_string())
}

const GITHUB_COPILOT_MODEL: &str = "github_copilot::openai/gpt-4.1-mini";

#[tokio::test]
#[ignore]
async fn record_github_copilot_simple_stream() -> TestResult<()> {
	let (client, mut server) = record_client("github_copilot", "simple_stream", &github_copilot_backend()).await?;

	let chat_req = ChatRequest::new(vec![
		ChatMessage::system("Answer in one sentence"),
		ChatMessage::user("Why is the sky blue?"),
	]);
	let options = ChatOptions::default().with_capture_content(true);

	let stream_res = client.exec_chat_stream(GITHUB_COPILOT_MODEL, chat_req, Some(&options)).await?;
	let extract = extract_stream_end(stream_res.stream).await?;
	eprintln!(
		"[record] Stream content: {:?}",
		extract.content.as_deref().map(|s| &s[..s.len().min(80)])
	);

	server.shutdown().await;
	Ok(())
}

#[tokio::test]
#[ignore]
async fn record_github_copilot_tool_stream() -> TestResult<()> {
	let (client, mut server) = record_client("github_copilot", "tool_stream", &github_copilot_backend()).await?;

	let chat_req = seed_tool_request();
	let options = ChatOptions::default().with_capture_content(true).with_capture_tool_calls(true);

	let stream_res = client.exec_chat_stream(GITHUB_COPILOT_MODEL, chat_req, Some(&options)).await?;
	let extract = extract_stream_end(stream_res.stream).await?;
	let tool_calls = &extract.stream_end.captured_tool_calls();
	eprintln!("[record] Tool calls: {:?}", tool_calls.as_ref().map(|tc| tc.len()));

	server.shutdown().await;
	Ok(())
}

fn seed_tool_request() -> ChatRequest {
	ChatRequest::new(vec![
		ChatMessage::system("You are a helpful assistant. Use tools when needed."),
		ChatMessage::user("What is the temperature in C and weather, in Paris, France"),
	])
	.append_tool(Tool::new("get_weather").with_schema(json!({
		"type": "object",
		"properties": {
			"city": {
				"type": "string",
				"description": "The city name"
			},
			"country": {
				"type": "string",
				"description": "The most likely country of this city name"
			},
			"unit": {
				"type": "string",
				"enum": ["C", "F"],
				"description": "The temperature unit of the country. C for Celsius, and F for Fahrenheit"
			}
		},
		"required": ["city", "country", "unit"],
	})))
}

fn ollama_cloud_backend() -> String {
	std::env::var("OLLAMA_CLOUD_BASE_URL").unwrap_or_else(|_| "https://ollama.com/".to_string())
}

const OLLAMA_CLOUD_MODEL: &str = "ollama_cloud::gemma3:4b";

#[tokio::test]
#[ignore]
async fn record_ollama_cloud_simple_stream() -> TestResult<()> {
	let (client, mut server) = record_client("ollama_cloud", "simple_stream", &ollama_cloud_backend()).await?;

	let chat_req = ChatRequest::new(vec![
		ChatMessage::system("Answer in one sentence."),
		ChatMessage::user("Why is the sky blue?"),
	]);
	let options = ChatOptions::default().with_capture_content(true);

	let stream_res = client.exec_chat_stream(OLLAMA_CLOUD_MODEL, chat_req, Some(&options)).await?;
	let extract = extract_stream_end(stream_res.stream).await?;
	eprintln!(
		"[record] Stream content: {:?}",
		extract.content.as_deref().map(|s| &s[..s.len().min(80)])
	);

	server.shutdown().await;
	Ok(())
}

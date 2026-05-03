use crate::get_option_value;
use crate::support::data::{
	AUDIO_TEST_FILE_PATH, IMAGE_URL_JPG_DUCK, TEST_IMAGE_FILE_PATH, get_b64_audio, get_b64_duck, get_b64_pdf,
	has_audio_file,
};
use crate::support::{
	Check, StreamExtract, TestResult, assert_contains, assert_reasoning_content, assert_reasoning_usage,
	contains_checks, extract_stream_end, get_big_content, seed_chat_req_simple, seed_chat_req_tool_simple,
	validate_checks,
};
use genai::adapter::AdapterKind;
use genai::chat::{
	BinarySource, CacheControl, ChatMessage, ChatOptions, ChatRequest, ChatResponseFormat, ContentPart, JsonSpec,
	ReasoningEffort, Tool, ToolResponse, Verbosity,
};
use genai::embed::EmbedOptions;
use genai::resolver::{AuthData, AuthResolver, AuthResolverFn, IntoAuthResolverFn};
use genai::{Client, ClientConfig, ModelIden};
use serde_json::{Value, json};
use std::sync::Arc;
use value_ext::JsonValueExt;

// region:    --- Chat

pub async fn common_test_chat_simple_ok(model: &str, checks: Option<Check>) -> TestResult<()> {
	validate_checks(checks.clone(), Check::REASONING_CONTENT | Check::REASONING_USAGE)?;

	// -- Setup & Fixtures
	let client = Client::default();
	let chat_req = seed_chat_req_simple();

	// -- Exec
	let chat_res = client.exec_chat(model, chat_req, None).await?;

	// -- Check Content
	let content = chat_res.first_text().ok_or("Should have content")?;
	assert!(!content.trim().is_empty(), "Content should not be empty");

	// -- Check Usage
	let usage = &chat_res.usage;
	let prompt_tokens = get_option_value!(usage.prompt_tokens);
	let completion_tokens = get_option_value!(usage.completion_tokens);
	let total_tokens = get_option_value!(usage.total_tokens);
	assert!(total_tokens > 0, "total_tokens should be > 0");
	assert!(
		total_tokens == prompt_tokens + completion_tokens,
		"total_tokens should be equal to prompt_token + comletion_token"
	);

	// -- Check Reasoning Usage
	if contains_checks(checks.clone(), Check::REASONING_USAGE) {
		assert_reasoning_usage(usage)?;
	}

	// -- Check Reasoning Content
	if contains_checks(checks, Check::REASONING_CONTENT) {
		assert_reasoning_content(&chat_res)?;
	}

	Ok(())
}

// NOTE: here we still have the options about checking REASONING_USAGE, because Anthropic does not have reasoning token.
pub async fn common_test_chat_reasoning_ok(
	model: &str,
	reasoning_effort: ReasoningEffort,
	checks: Option<Check>,
) -> TestResult<()> {
	// -- Setup & Fixtures
	let client = Client::default();
	let chat_req = ChatRequest::new(vec![
		// -- Messages (deactivate to see the differences)
		ChatMessage::system("Answer in one sentence. But make think hard to make sure it is not a trick question."),
		ChatMessage::user("Why is the sky red?"),
	]);
	let options = ChatOptions::default().with_reasoning_effort(reasoning_effort);

	// -- Exec
	let chat_res = client.exec_chat(model, chat_req, Some(&options)).await?;

	// -- Check Content
	let content = chat_res.first_text().ok_or("Should have content")?;
	assert!(!content.trim().is_empty(), "Content should not be empty");

	// -- Check Usage
	let usage = &chat_res.usage;
	let prompt_tokens = get_option_value!(usage.prompt_tokens);
	let completion_tokens = get_option_value!(usage.completion_tokens);
	let total_tokens = get_option_value!(usage.total_tokens);
	assert!(total_tokens > 0, "total_tokens should be > 0");
	assert!(
		total_tokens == prompt_tokens + completion_tokens,
		"total_tokens should be equal to prompt_token + comletion_token"
	);

	// -- Check Reasoning Usage
	if contains_checks(checks.clone(), Check::REASONING_USAGE) {
		let reasoning_tokens = usage
			.completion_tokens_details
			.as_ref()
			.and_then(|v| v.reasoning_tokens)
			.ok_or("should have reasoning_tokens")?;
		assert!(reasoning_tokens > 0, "reasoning_usage should be > 0");
	}

	// -- Check Reasoning Content
	if contains_checks(checks, Check::REASONING_CONTENT) {
		let reasoning_content = chat_res
			.reasoning_content
			.as_deref()
			.ok_or("SHOULD have extracted some reasoning_content")?;
		assert!(!reasoning_content.is_empty(), "reasoning_content should not be empty");
	}

	Ok(())
}

pub async fn common_test_chat_verbosity_ok(model: &str) -> TestResult<()> {
	// -- Setup & Fixtures
	let chat_client_options = ChatOptions::default().with_reasoning_effort(ReasoningEffort::Low);
	let client = Client::builder().with_chat_options(chat_client_options).build();
	let chat_req = ChatRequest::new(vec![
		//
		ChatMessage::user("Why is the sky blue?"),
	]);

	// -- Exec
	// content low
	let chat_options = ChatOptions::default().with_verbosity(Verbosity::Low);
	let chat_res = client.exec_chat(model, chat_req.clone(), Some(&chat_options)).await?;
	let content_low = chat_res.first_text().ok_or("Should have content")?;
	// content high
	let chat_options = ChatOptions::default().with_verbosity(Verbosity::High);
	let chat_res = client.exec_chat(model, chat_req, Some(&chat_options)).await?;
	let content_high = chat_res.first_text().ok_or("Should have content")?;

	// -- Check Content
	let ratio = content_high.len() as f64 / content_low.len() as f64;
	assert!(
		ratio >= 1.5,
		"The verbosity high was not high enough compared to the low. Ratio {ratio}"
	);

	Ok(())
}

pub async fn common_test_chat_top_system_ok(model: &str) -> TestResult<()> {
	// -- Setup & Fixtures
	let client = Client::default();
	let chat_req = ChatRequest::new(vec![
		// -- Messages (deactivate to see the differences)
		ChatMessage::user("Why is the sky blue?"),
	])
	.with_system("Be very concise, and at end with 'Thank you'");

	// -- Exec
	let chat_res = client.exec_chat(model, chat_req, None).await?;

	// -- Check
	let content_txt = chat_res.content.joined_texts().ok_or("Should have texts")?;
	assert_contains(&content_txt, "Thank you");

	Ok(())
}

pub async fn common_test_chat_multi_system_ok(model: &str) -> TestResult<()> {
	// -- Setup & Fixtures
	let client = Client::default();
	let chat_req = ChatRequest::new(vec![
		// -- Messages (deactivate to see the differences)
		ChatMessage::system("Be very concise"),
		ChatMessage::system("Explain with bullet points"),
		ChatMessage::user("Why is the sky blue?"),
	])
	.with_system("And end with 'Thank you'");

	// -- Exec
	let chat_res = client.exec_chat(model, chat_req, None).await?;

	// -- Check
	assert!(!chat_res.content.is_empty(), "Content should not be empty");
	let usage = chat_res.usage;
	let prompt_tokens = get_option_value!(usage.prompt_tokens);
	let completion_tokens = get_option_value!(usage.completion_tokens);
	let total_tokens = get_option_value!(usage.total_tokens);

	assert!(total_tokens > 0, "total_tokens should be > 0");
	assert!(
		total_tokens == prompt_tokens + completion_tokens,
		"total_tokens should be equal to prompt_tokens + completion_tokens"
	);

	Ok(())
}

/// Test with JSON mode enabled. This is not a structured output test.
/// - test_token: This is to avoid checking the token (due to an Ollama bug when in JSON mode, no token is returned)
pub async fn common_test_chat_json_mode_ok(model: &str, checks: Option<Check>) -> TestResult<()> {
	validate_checks(checks.clone(), Check::USAGE)?;

	// -- Setup & Fixtures
	let client = Client::default();
	let chat_req = ChatRequest::new(vec![
		// -- Messages (de/activate to see the differences)
		ChatMessage::system(
			r#"Turn the user content into the most probable JSON content.
Reply in a JSON format."#,
		),
		ChatMessage::user(
			r#"
| Model          | Maker
| gpt-4o	       | OpenAI
| gpt-4o-mini	   | OpenAI
| llama-3.1-70B  | Meta
		"#,
		),
	]);
	let chat_options = ChatOptions::default().with_response_format(ChatResponseFormat::JsonMode);

	// -- Exec
	let chat_res = client.exec_chat(model, chat_req, Some(&chat_options)).await?;

	// -- Check
	// Ensure tokens are still counted
	if contains_checks(checks, Check::USAGE) {
		// Ollama does not send back token usage when in JSON mode
		let usage = &chat_res.usage;
		let total_tokens = get_option_value!(usage.total_tokens);
		assert!(total_tokens > 0, "total_tokens should be > 0");
	}

	// Check content
	let content = chat_res.into_first_text().ok_or("SHOULD HAVE CONTENT")?;
	// Parse content as JSON
	let json: serde_json::Value = serde_json::from_str(&content).map_err(|err| format!("Was not valid JSON: {err}"))?;
	// Pretty print JSON
	let pretty_json = serde_json::to_string_pretty(&json).map_err(|err| format!("Was not valid JSON: {err}"))?;

	Ok(())
}

/// Test with JSON mode enabled. This is not a structured output test.
/// - test_token: This is to avoid checking the token (due to an Ollama bug when in JSON mode, no token is returned)
pub async fn common_test_chat_json_structured_ok(model: &str, checks: Option<Check>) -> TestResult<()> {
	validate_checks(checks.clone(), Check::USAGE)?;

	// -- Setup & Fixtures
	let client = Client::default();
	let chat_req = ChatRequest::new(vec![
		// -- Messages (de/activate to see the differences)
		ChatMessage::system(
			r#"Turn the user content into the most probable JSON content.
Reply in a JSON format."#,
		),
		ChatMessage::user(
			r#"
| Model          | Maker
| gpt-4o	       | OpenAI
| gpt-4o-mini	   | OpenAI
| llama-3.1-70B  | Meta
		"#,
		),
	]);

	let json_schema = json!({
	  "type": "object",
		// "additionalProperties": false,
	  "properties": {
			"all_models": {
				"type": "array",
				"items": {
					"type": "object",
					// "additionalProperties": false,
					"properties": {
						"maker": { "type": "string" },
						"model_name": { "type": "string" }
					},
					"required": ["maker", "model_name"]
				}
			}
	  },
	  "required": ["all_models"]
	});

	let chat_options = ChatOptions::default().with_response_format(JsonSpec::new("some-schema", json_schema));

	// -- Exec
	let chat_res = client.exec_chat(model, chat_req, Some(&chat_options)).await?;

	// -- Check
	// Ensure tokens are still counted
	if contains_checks(checks, Check::USAGE) {
		// Ollama does not send back token usage when in JSON mode
		let usage = &chat_res.usage;
		let total_tokens = get_option_value!(usage.total_tokens);
		assert!(total_tokens > 0, "total_tokens should be > 0");
	}

	// Check content
	let content = chat_res.into_first_text().ok_or("SHOULD HAVE CONTENT")?;
	// Parse content as JSON
	let json_response: serde_json::Value =
		serde_json::from_str(&content).map_err(|err| format!("Was not valid JSON: {err}"))?;
	// Check models count
	let models: Vec<Value> = json_response.x_get("all_models")?;
	assert_eq!(3, models.len(), "Number of models");
	let first_maker: String = models.first().ok_or("No models")?.x_get("maker")?;
	assert_eq!("OpenAI", first_maker, "First maker");

	Ok(())
}

pub async fn common_test_chat_temperature_ok(model: &str) -> TestResult<()> {
	// -- Setup & Fixtures
	let client = Client::default();
	let chat_req = seed_chat_req_simple();
	let chat_options = ChatOptions::default().with_temperature(0.);

	// -- Exec
	let chat_res = client.exec_chat(model, chat_req, Some(&chat_options)).await?;

	// -- Check
	assert!(
		!chat_res.first_text().unwrap_or("").is_empty(),
		"Content should not be empty"
	);

	Ok(())
}

pub async fn common_test_chat_stop_sequences_ok(model: &str) -> TestResult<()> {
	// -- Setup & Fixtures
	let client = Client::default();
	let chat_req = ChatRequest::from_user("What is the capital of England?");
	let chat_options = ChatOptions::default().with_stop_sequences(vec!["London".to_string()]);

	// -- Exec
	let chat_res = client.exec_chat(model, chat_req, Some(&chat_options)).await?;

	// -- Check
	// Note: if there is no content, that's ok, it means "London" was the first answer.
	if let Some(ai_content) = chat_res.content.into_joined_texts().map(|s| s.to_lowercase()) {
		assert!(!ai_content.contains("london"), "Content should not contain 'London'");
	}

	Ok(())
}

pub async fn common_test_chat_reasoning_normalize_ok(model: &str) -> TestResult<()> {
	// -- Setup & Fixtures
	let client = Client::builder()
		.with_chat_options(ChatOptions::default().with_normalize_reasoning_content(true))
		.build();
	let chat_req = seed_chat_req_simple();

	// -- Exec
	let chat_res = client.exec_chat(model, chat_req, None).await?;

	// -- Check Content
	chat_res.first_text();
	let content = chat_res.first_text().ok_or("Should have content")?;
	assert!(!content.trim().is_empty(), "Content should not be empty");

	// -- Check Reasoning Content
	let reasoning_content = chat_res.reasoning_content.as_deref().ok_or("Should have reasoning_content")?;
	// Because the test is so simple, sometime ollama for example, will have `<think>\n\n</think>`
	// So we are ok if is_empty() for now
	// assert!(!reasoning_content.is_empty(), "reasoning_content should not be empty");
	assert!(
		reasoning_content.len() > content.len(),
		"reasoning_content should be > than the content"
	);

	// -- Check Usage
	let usage = chat_res.usage;
	let prompt_tokens = get_option_value!(usage.prompt_tokens);
	let completion_tokens = get_option_value!(usage.completion_tokens);
	let total_tokens = get_option_value!(usage.total_tokens);
	assert!(total_tokens > 0, "total_tokens should be > 0");
	assert!(
		total_tokens == prompt_tokens + completion_tokens,
		"total_tokens should be equal to prompt_token + completion_tokens"
	);

	Ok(())
}

// endregion: --- Chat

// region:    --- Chat Implicit Cache

pub async fn common_test_chat_cache_implicit_simple_ok(model: &str) -> TestResult<()> {
	// -- Setup & Fixtures
	let client = Client::default();
	let big_content = get_big_content()?;
	let chat_req = ChatRequest::new(vec![
		// -- Messages (deactivate to see the differences)
		ChatMessage::user(big_content),
		ChatMessage::user("Give a very short summary of what each of those files are about."),
	]);

	// -- Exec
	// Execute three times
	let chat_res = client.exec_chat(model, chat_req.clone(), None).await?;
	// sleep 500ms
	tokio::time::sleep(std::time::Duration::from_millis(200)).await;
	let chat_res = client.exec_chat(model, chat_req.clone(), None).await?;
	tokio::time::sleep(std::time::Duration::from_millis(200)).await;
	let chat_res = client.exec_chat(model, chat_req, None).await?;

	// -- Check Content
	let content = chat_res.first_text().ok_or("Should have content")?;
	assert!(!content.trim().is_empty(), "Content should not be empty");

	// -- Check Usage
	let usage = &chat_res.usage;

	let prompt_tokens = get_option_value!(usage.prompt_tokens);
	let completion_tokens = get_option_value!(usage.completion_tokens);
	let total_tokens = get_option_value!(usage.total_tokens);
	let prompt_tokens_details = usage
		.prompt_tokens_details
		.as_ref()
		.ok_or("Should have prompt_tokens_details")?;
	let cached_tokens = get_option_value!(prompt_tokens_details.cached_tokens);

	assert!(cached_tokens > 0, "cached_tokens should be greater than 0");
	assert!(total_tokens > 0, "total_tokens should be > 0");

	Ok(())
}

// endregion: --- Chat Implicit Cache

// region:    --- Chat Explicit Cache

pub async fn common_test_chat_cache_explicit_user_ok(model: &str) -> TestResult<()> {
	// -- Setup & Fixtures
	let client = Client::default();
	let big_content = get_big_content()?;
	let chat_req = ChatRequest::new(vec![
		// -- Messages (deactivate to see the differences)
		ChatMessage::system("Give a very short summary of what each of those files are about"),
		ChatMessage::user(big_content).with_options(CacheControl::Ephemeral),
	]);

	// -- Exec
	let chat_res = client.exec_chat(model, chat_req, None).await?;

	// -- Check Content
	let content = chat_res.first_text().ok_or("Should have content")?;
	assert!(!content.trim().is_empty(), "Content should not be empty");

	// -- Check Usage
	let usage = &chat_res.usage;

	let prompt_tokens = get_option_value!(usage.prompt_tokens);
	let completion_tokens = get_option_value!(usage.completion_tokens);
	let total_tokens = get_option_value!(usage.total_tokens);
	let prompt_tokens_details = usage
		.prompt_tokens_details
		.as_ref()
		.ok_or("Should have prompt_tokens_details")?;
	let cache_creation_tokens = get_option_value!(prompt_tokens_details.cache_creation_tokens);
	let cached_tokens = get_option_value!(prompt_tokens_details.cached_tokens);

	assert!(
		cache_creation_tokens > 0 || cached_tokens > 0,
		"one of cache_creation_tokens or cached_tokens should be greater than 0"
	);
	assert!(total_tokens > 0, "total_tokens should be > 0");

	Ok(())
}

pub async fn common_test_chat_cache_explicit_system_ok(model: &str) -> TestResult<()> {
	// -- Setup & Fixtures
	let client = Client::default();
	let big_content = get_big_content()?;
	let chat_req = ChatRequest::new(vec![
		// -- Messages (deactivate to see the differences)
		ChatMessage::system("You are a senior developer which has the following code base:"),
		ChatMessage::system(big_content).with_options(CacheControl::Ephemeral),
		ChatMessage::user("can you give a summary of each file (very concise)"),
	]);

	// -- Exec
	let chat_res = client.exec_chat(model, chat_req, None).await?;

	// -- Check Content
	let content = chat_res.first_text().ok_or("Should have content")?;
	assert!(!content.trim().is_empty(), "Content should not be empty");

	// -- Check Usage
	let usage = &chat_res.usage;

	let prompt_tokens = get_option_value!(usage.prompt_tokens);
	let completion_tokens = get_option_value!(usage.completion_tokens);
	let total_tokens = get_option_value!(usage.total_tokens);
	let prompt_tokens_details = usage
		.prompt_tokens_details
		.as_ref()
		.ok_or("Should have prompt_tokens_details")?;
	let cache_creation_tokens = get_option_value!(prompt_tokens_details.cache_creation_tokens);
	let cached_tokens = get_option_value!(prompt_tokens_details.cached_tokens);

	assert!(
		cache_creation_tokens > 0 || cached_tokens > 0,
		"one of cache_creation_tokens or cached_tokens should be greater than 0"
	);
	assert!(total_tokens > 0, "total_tokens should be > 0");

	Ok(())
}

/// Test for 1-hour TTL cache (Ephemeral1h).
/// Note: 1h TTL is only supported on Claude 4.5 models (Opus 4.5, Sonnet 4.5, Haiku 4.5).
pub async fn common_test_chat_cache_explicit_1h_ttl_ok(model: &str) -> TestResult<()> {
	// -- Setup & Fixtures
	let client = Client::default();
	let big_content = get_big_content()?;
	let chat_req = ChatRequest::new(vec![
		// -- Messages
		ChatMessage::system("Give a very short summary of what each of those files are about"),
		ChatMessage::user(big_content).with_options(CacheControl::Ephemeral1h),
	]);

	// -- Exec
	let chat_res = client.exec_chat(model, chat_req, None).await?;

	// -- Check Content
	let content = chat_res.first_text().ok_or("Should have content")?;
	assert!(!content.trim().is_empty(), "Content should not be empty");

	// -- Check Usage
	let usage = &chat_res.usage;

	let total_tokens = get_option_value!(usage.total_tokens);
	let prompt_tokens_details = usage
		.prompt_tokens_details
		.as_ref()
		.ok_or("Should have prompt_tokens_details")?;
	let cache_creation_tokens = get_option_value!(prompt_tokens_details.cache_creation_tokens);
	let cached_tokens = get_option_value!(prompt_tokens_details.cached_tokens);

	assert!(
		cache_creation_tokens > 0 || cached_tokens > 0,
		"one of cache_creation_tokens or cached_tokens should be greater than 0"
	);
	assert!(total_tokens > 0, "total_tokens should be > 0");

	// Note: cache_creation_details may or may not be present depending on provider response format
	// The API may return TTL-specific breakdown in cache_creation_details when using different TTLs

	Ok(())
}

/// Streaming test for 1-hour TTL cache (Ephemeral1h).
/// Note: 1h TTL is only supported on Claude 4.5 models (Opus 4.5, Sonnet 4.5, Haiku 4.5).
pub async fn common_test_chat_stream_cache_explicit_1h_ttl_ok(model: &str) -> TestResult<()> {
	// -- Setup & Fixtures
	let client = Client::builder()
		.with_chat_options(ChatOptions::default().with_capture_usage(true))
		.build();
	let big_content = get_big_content()?;
	let chat_req = ChatRequest::new(vec![
		// -- Messages
		ChatMessage::system("Give a very short summary of what each of those files are about"),
		ChatMessage::user(big_content).with_options(CacheControl::Ephemeral1h),
	]);

	// -- Exec
	let chat_res = client.exec_chat_stream(model, chat_req, None).await?;

	// -- Extract Stream content
	let StreamExtract {
		stream_end,
		content,
		reasoning_content: _,
		..
	} = extract_stream_end(chat_res.stream).await?;
	let content = content.ok_or("extract_stream_end SHOULD have extracted some content")?;

	// -- Check Content
	assert!(!content.trim().is_empty(), "Content should not be empty");

	// -- Check Usage
	let usage = stream_end.captured_usage.as_ref().ok_or("Should have captured_usage")?;

	let total_tokens = get_option_value!(usage.total_tokens);
	let prompt_tokens_details = usage
		.prompt_tokens_details
		.as_ref()
		.ok_or("Should have prompt_tokens_details")?;
	let cache_creation_tokens = get_option_value!(prompt_tokens_details.cache_creation_tokens);
	let cached_tokens = get_option_value!(prompt_tokens_details.cached_tokens);

	assert!(
		cache_creation_tokens > 0 || cached_tokens > 0,
		"one of cache_creation_tokens or cached_tokens should be greater than 0"
	);
	assert!(total_tokens > 0, "total_tokens should be > 0");

	Ok(())
}

// endregion: --- Chat Explicit Cache

// region:    --- Chat Stream Tests

pub async fn common_test_chat_stream_simple_ok(model: &str, checks: Option<Check>) -> TestResult<()> {
	validate_checks(checks.clone(), Check::REASONING_CONTENT)?;

	// -- Setup & Fixtures
	let client = Client::default();
	let chat_req = seed_chat_req_simple();

	// -- Exec
	let chat_res = client.exec_chat_stream(model, chat_req.clone(), None).await?;

	// -- Extract Stream content
	let StreamExtract {
		stream_end,
		content,
		reasoning_content,
		..
	} = extract_stream_end(chat_res.stream).await?;
	let content = content.ok_or("extract_stream_end SHOULD have extracted some content")?;

	// -- Check no meta_usage and captured_content
	assert!(!content.is_empty(), "Content streamed should not be empty");
	assert!(
		stream_end.captured_usage.is_none(),
		"StreamEnd should not have any meta_usage"
	);

	// NOTE: with gemini 3, signature is always sent back in the captured message content, even if
	//       the captured_content is not set (because it meant capture message content)
	//
	let no_captured_text = match stream_end.captured_content.as_ref() {
		None => true,
		Some(cc) => cc.texts().is_empty(),
	};
	assert!(no_captured_text, "StreamEnd should not have any text captured_content");

	// -- Check Reasoning Content
	if contains_checks(checks, Check::REASONING_CONTENT) {
		let reasoning_content =
			reasoning_content.ok_or("extract_stream_end SHOULD have extracted some reasoning_content")?;
		assert!(!reasoning_content.is_empty(), "reasoning_content should not be empty");
		// We can assume that the reasoning content should be bigger than the content given the prompt to keep content very concise.
		assert!(
			reasoning_content.len() > content.len(),
			"Reasoning content should be > than the content"
		);
	}

	Ok(())
}

/// Check that the capture content flag does the capture
/// NOTE: When checking for reasoning, the captured_reasoning_content should be None in this function
pub async fn common_test_chat_stream_capture_content_ok(model: &str) -> TestResult<()> {
	// -- Setup & Fixtures
	let client = Client::builder()
		.with_chat_options(ChatOptions::default().with_capture_content(true))
		.build();
	let chat_req = seed_chat_req_simple();

	// -- Exec
	let chat_res = client.exec_chat_stream(model, chat_req.clone(), None).await?;

	// -- Extract Stream content
	let StreamExtract {
		stream_end,
		content,
		reasoning_content,
		..
	} = extract_stream_end(chat_res.stream).await?;

	// -- Check meta_usage
	// Should be None as not captured
	assert!(
		stream_end.captured_usage.is_none(),
		"StreamEnd should not have any meta_usage"
	);

	// -- Check Reasoning Content
	// Should always be none, as it was not instructed to be captured.
	assert!(
		stream_end.captured_reasoning_content.is_none(),
		"The captured_reasoning_content should be None"
	);

	// -- Check captured_content
	let captured_content = get_option_value!(stream_end.captured_content);
	assert!(!captured_content.is_empty(), "captured_content.length should be > 0");

	Ok(())
}

pub async fn common_test_chat_stream_capture_all_ok(model: &str, checks: Option<Check>) -> TestResult<()> {
	validate_checks(checks.clone(), Check::REASONING_CONTENT | Check::REASONING_USAGE)?;

	// -- Setup & Fixtures
	let mut chat_options = ChatOptions::default()
		.with_capture_usage(true)
		.with_capture_content(true)
		.with_capture_reasoning_content(true);

	if contains_checks(checks.clone(), Check::REASONING_CONTENT | Check::REASONING_USAGE) {
		chat_options = chat_options.with_reasoning_effort(ReasoningEffort::Medium);
	}

	let client = Client::builder().with_chat_options(chat_options).build();
	let chat_req = seed_chat_req_simple();

	// -- Exec
	let chat_res = client.exec_chat_stream(model, chat_req.clone(), None).await?;

	// -- Extract Stream content
	let StreamExtract {
		stream_end,
		content,
		reasoning_content,
		..
	} = extract_stream_end(chat_res.stream).await?;

	// -- Check meta_usage
	let meta_usage = stream_end.captured_usage.as_ref().ok_or("Should have usage")?;

	assert!(
		get_option_value!(meta_usage.prompt_tokens) > 0,
		"prompt_token should be > 0"
	);
	assert!(
		get_option_value!(meta_usage.completion_tokens) > 0,
		"completion_tokens should be > 0"
	);
	assert!(
		get_option_value!(meta_usage.total_tokens) > 0,
		"total_tokens should be > 0"
	);

	// -- Check captured_content
	let captured_content = stream_end.captured_first_text();
	let captured_content = captured_content.ok_or("Captured content should have a text")?;
	assert!(!captured_content.is_empty(), "captured_content.length should be > 0");

	// -- Check Reasoning Usage
	if contains_checks(checks.clone(), Check::REASONING_USAGE) {
		assert_reasoning_usage(meta_usage)?;
	}

	// -- Check Reasoning Content
	if contains_checks(checks, Check::REASONING_CONTENT) {
		let _reasoning_content = reasoning_content.ok_or("Should have reasoning content")?;
	}

	Ok(())
}

/// Just making the tool request, and checking the tool call response
/// `complete_check` if for LLMs that are better at giving back the unit and weather.
pub async fn common_test_chat_stream_tool_capture_ok(model: &str) -> TestResult<()> {
	// -- Setup & Fixtures
	let client = Client::default();
	let chat_req = seed_chat_req_tool_simple();
	let mut chat_options = ChatOptions::default().with_capture_tool_calls(true);

	// -- Exec
	let chat_res = client.exec_chat_stream(model, chat_req, Some(&chat_options)).await?;

	// Extract Stream content
	let StreamExtract {
		stream_end,
		content,
		reasoning_content,
		..
	} = extract_stream_end(chat_res.stream).await?;

	// -- Check
	let mut tool_calls = stream_end.captured_tool_calls().ok_or("Should have captured tools")?;
	if tool_calls.is_empty() {
		return Err("Should have tool calls in chat_res".into());
	}
	let tool_call = tool_calls.pop().ok_or("Should have at least one tool call")?;
	assert_eq!(tool_call.fn_arguments.x_get_as::<&str>("city")?, "Paris");
	assert_eq!(tool_call.fn_arguments.x_get_as::<&str>("country")?, "France");
	assert_eq!(tool_call.fn_arguments.x_get_as::<&str>("unit")?, "C");

	Ok(())
}

// endregion: --- Chat Stream Tests

// region:    --- Binaries

pub async fn common_test_chat_image_url_ok(model: &str) -> TestResult<()> {
	// -- Setup
	let client = Client::default();

	// -- Build & Exec
	let mut chat_req = ChatRequest::default().with_system("Answer in one sentence");
	// This is similar to sending initial system chat messages (which will be cumulative with system chat messages)
	chat_req = chat_req.append_message(ChatMessage::user(vec![
		ContentPart::from_text("What is in this picture?"),
		ContentPart::from_binary_url("image/jpeg", IMAGE_URL_JPG_DUCK, None),
	]));
	let chat_res = client.exec_chat(model, chat_req, None).await?;

	// -- Check
	let res = chat_res.first_text().ok_or("Should have text result")?;
	assert_contains(res, "duck");

	Ok(())
}

pub async fn common_test_chat_image_b64_ok(model: &str) -> TestResult<()> {
	// -- Setup
	let client = Client::default();

	// -- Build & Exec
	let mut chat_req = ChatRequest::default().with_system("Answer in one sentence");
	// This is similar to sending initial system chat messages (which will be cumulative with system chat messages)
	chat_req = chat_req.append_message(ChatMessage::user(vec![
		ContentPart::from_text("What is in this picture?"),
		ContentPart::from_binary_base64("image/jpeg", get_b64_duck()?, None),
	]));

	let chat_res = client.exec_chat(model, chat_req, None).await?;

	// -- Check
	let res = chat_res.first_text().ok_or("Should have text result")?;
	assert_contains(res, "duck");

	Ok(())
}

pub async fn common_test_chat_image_file_ok(model: &str) -> TestResult<()> {
	// -- Setup
	let client = Client::default();

	// -- Build & Exec
	let mut chat_req = ChatRequest::default().with_system("Answer in one sentence");
	// This is similar to sending initial system chat messages (which will be cumulative with system chat messages)
	chat_req = chat_req.append_message(ChatMessage::user(vec![
		ContentPart::from_text("What is in this picture?"),
		ContentPart::from_binary_file(TEST_IMAGE_FILE_PATH)?,
	]));

	let chat_res = client.exec_chat(model, chat_req, None).await?;

	// -- Check
	let res = chat_res.first_text().ok_or("Should have text result")?;
	assert_contains(res, "duck");

	Ok(())
}

pub async fn common_test_chat_audio_b64_ok(model: &str) -> TestResult<()> {
	if !has_audio_file() {
		println!("No test audio file. Skipping this test.");
		return Ok(());
	}

	// -- Setup
	let client = Client::default();

	// -- Build & Exec
	let mut chat_req = ChatRequest::default().with_system("Transcribe the audio");
	let cp_audio = ContentPart::from_binary_file(AUDIO_TEST_FILE_PATH)?;
	// similar as the from_binary_file but manual
	// let cp_audio = ContentPart::from_binary_base64("audio/wav", get_b64_audio()?, None);

	chat_req = chat_req.append_message(ChatMessage::user(vec![cp_audio]));

	let chat_res = client.exec_chat(model, chat_req, None).await?;

	// -- Check
	let res = chat_res.first_text().ok_or("Should have text result")?;
	// NOTE: here we make the test a little loose as the point of this test is not to test the model accuracy
	assert_contains(res, "one small step");
	assert_contains(res, "one giant leap");

	Ok(())
}

pub async fn common_test_chat_pdf_b64_ok(model: &str) -> TestResult<()> {
	// -- Setup
	let client = Client::default();

	// -- Build & Exec
	let mut chat_req = ChatRequest::default().with_system("Answer in one sentence");
	// This is similar to sending initial system chat messages (which will be cumulative with system chat messages)
	chat_req = chat_req.append_message(ChatMessage::user(vec![
		ContentPart::from_text("What does this document talk about?"),
		ContentPart::from_binary_base64("application/pdf", get_b64_pdf()?, Some("small.pdf".to_string())),
	]));

	let chat_res = client.exec_chat(model, chat_req, None).await?;

	// -- Check
	let res = chat_res.first_text().ok_or("Should have text result")?;
	assert_contains(res, "quantum");

	Ok(())
}

pub async fn common_test_chat_multi_binary_b64_ok(model: &str) -> TestResult<()> {
	// -- Setup
	let client = Client::default();

	// -- Build & Exec
	let mut chat_req = ChatRequest::default().with_system("Answer in one sentence");
	// This is similar to sending initial system chat messages (which will be cumulative with system chat messages)
	chat_req = chat_req.append_message(ChatMessage::user(vec![
		ContentPart::from_binary_base64("image/jpeg", get_b64_duck()?, None),
		ContentPart::from_binary_base64("application/pdf", get_b64_pdf()?, Some("small.pdf".to_string())),
	]));
	chat_req = chat_req.append_message(ChatMessage::user(
		"
Can you tell me what those images and files are about.
	",
	));

	let chat_res = client.exec_chat(model, chat_req, None).await?;

	// -- Check
	let res = chat_res.first_text().ok_or("Should have text result")?;
	assert_contains(res, "quantum");
	assert_contains(res, "duck");

	Ok(())
}
// endregion: --- Binaries

// region:    --- Tools

/// Just making the tool request, and checking the tool call response
/// `complete_check` if for LLMs that are better at giving back the unit and weather.
pub async fn common_test_tool_simple_ok(model: &str) -> TestResult<()> {
	// -- Setup & Fixtures
	let client = Client::default();
	let chat_req = seed_chat_req_tool_simple();

	// -- Exec
	let chat_res = client.exec_chat(model, chat_req, None).await?;

	// -- Check
	let mut tool_calls = chat_res.tool_calls();
	if tool_calls.is_empty() {
		return Err("Should have tool calls in chat_res".into());
	}
	let tool_call = tool_calls.pop().ok_or("Should have at least one tool call")?;
	assert_eq!(tool_call.fn_arguments.x_get_as::<&str>("city")?, "Paris");
	assert_eq!(tool_call.fn_arguments.x_get_as::<&str>("country")?, "France");
	assert_eq!(tool_call.fn_arguments.x_get_as::<&str>("unit")?, "C");

	Ok(())
}

pub async fn common_test_tool_full_flow_ok(model: &str) -> TestResult<()> {
	// -- Setup & Fixtures
	let client = Client::default();
	let mut chat_req = seed_chat_req_tool_simple();

	// -- Exec first request to get the tool calls
	let chat_res = client.exec_chat(model, chat_req.clone(), None).await?;
	let tool_calls = chat_res.into_tool_calls();

	if tool_calls.is_empty() {
		return Err("Should have tool calls in chat_res".into());
	}

	// -- Exec the second request
	// get the tool call id (first one)
	let first_tool_call = tool_calls.first().ok_or("Should have at least one tool call")?;
	let first_tool_call_id = &first_tool_call.call_id;
	// simulate the response
	let tool_response = ToolResponse::new(first_tool_call_id, r#"{"weather": "Sunny", "temperature": "32C"}"#);

	// Add the tool_calls, tool_response
	let chat_req = chat_req.append_message(tool_calls).append_message(tool_response);

	let chat_res = client.exec_chat(model, chat_req.clone(), None).await?;

	// -- Check
	// lowercase because some models send "Sunny" and not "sunny"
	let content = chat_res.first_text().ok_or("Last response should be message")?.to_lowercase();

	assert!(content.contains("paris"), "Should contain 'Paris'");
	assert!(content.contains("32"), "Should contain '32'");
	assert!(content.contains("sunny"), "Should contain 'sunny'");

	Ok(())
}

// endregion: --- Tools

// region:    --- With Resolvers

pub async fn common_test_resolver_auth_ok(model: &str, auth_data: AuthData) -> TestResult<()> {
	// -- Setup & Fixtures
	let auth_resolver = AuthResolver::from_resolver_fn(move |model_iden: ModelIden| Ok(Some(auth_data)));
	let client = Client::builder().with_auth_resolver(auth_resolver).build();
	let chat_req = seed_chat_req_simple();

	// -- Exec
	let chat_res = client.exec_chat(model, chat_req, None).await?;

	// -- Check
	assert!(!chat_res.content.is_empty(), "Content should not be empty");
	let usage = chat_res.usage;
	let total_tokens = get_option_value!(usage.total_tokens);
	assert!(total_tokens > 0, "total_tokens should be > 0");

	Ok(())
}

// endregion: --- With Resolvers

// region:    --- List

pub async fn common_test_list_models(adapter_kind: AdapterKind, contains: &str) -> TestResult<()> {
	let client = Client::default();

	// -- Exec
	let models = client.all_model_names(adapter_kind).await?;

	// -- Check
	assert_contains(&models, contains);

	Ok(())
}

// endregion: --- List

// region:    --- Embeddings

pub async fn common_test_embed_single_simple_ok(model: &str) -> TestResult<()> {
	common_test_embed_single_simple_ok_with_usage_check(model, true).await
}

pub async fn common_test_embed_single_simple_ok_with_usage_check(model: &str, expect_usage: bool) -> TestResult<()> {
	// -- Setup & Fixtures
	let client = Client::default();
	let text = "Hello, world!";

	// -- Exec
	let response = client.embed(model, text, None).await?;

	// -- Check Basic Properties
	assert_eq!(response.embedding_count(), 1);
	assert!(response.is_single());
	assert!(!response.is_batch());

	// -- Check Embedding
	let embedding = response.first_embedding().ok_or("Should have embedding")?;
	assert_eq!(embedding.index(), 0);
	assert!(embedding.dimensions() > 0);
	assert_eq!(embedding.vector().len(), embedding.dimensions());

	// -- Check Usage (provider-dependent)
	assert!(response.usage.completion_tokens.is_none()); // Embeddings don't have completion tokens

	if expect_usage {
		assert!(response.usage.prompt_tokens.is_some());
		assert!(response.usage.prompt_tokens.ok_or("should have prompt token")? > 0);
		assert!(response.usage.total_tokens.is_some());
		println!(
			"✓ Single embedding: {} dimensions, {} tokens",
			embedding.dimensions(),
			response.usage.prompt_tokens.ok_or("Should have prompt_tokens")?
		);
	} else {
		// Some providers (like Gemini) don't provide usage information for embeddings
		assert!(response.usage.prompt_tokens.is_none());
		assert!(response.usage.total_tokens.is_none());
		println!(
			"✓ Single embedding: {} dimensions (no usage info)",
			embedding.dimensions()
		);
	}

	Ok(())
}

pub async fn common_test_embed_single_with_options_ok(model: &str) -> TestResult<()> {
	common_test_embed_single_with_options_ok_with_usage_check(model, true).await
}

pub async fn common_test_embed_single_with_options_ok_with_usage_check(
	model: &str,
	expect_usage: bool,
) -> TestResult<()> {
	// -- Setup & Fixtures
	let client = Client::default();
	let text = "Test with options";

	let options = EmbedOptions::new()
		.with_dimensions(512)
		.with_capture_usage(true)
		.with_user("test-user".to_string());

	// -- Exec
	let response = client.embed(model, text, Some(&options)).await?;

	// -- Check
	let embedding = response.first_embedding().ok_or("Should have embedding")?;
	assert!(embedding.dimensions() > 0);

	// Usage check (provider-dependent)
	if expect_usage {
		assert!(response.usage.prompt_tokens.is_some());
		println!(
			"✓ Embedding with options: {} dimensions (requested 512)",
			embedding.dimensions()
		);
	} else {
		// Some providers (like Gemini) don't provide usage information for embeddings
		assert!(response.usage.prompt_tokens.is_none());
		println!(
			"✓ Embedding with options: {} dimensions (requested 512, no usage info)",
			embedding.dimensions()
		);
	}

	Ok(())
}

pub async fn common_test_embed_batch_simple_ok(model: &str) -> TestResult<()> {
	common_test_embed_batch_simple_ok_with_usage_check(model, true).await
}

pub async fn common_test_embed_batch_simple_ok_with_usage_check(model: &str, expect_usage: bool) -> TestResult<()> {
	// -- Setup & Fixtures
	let client = Client::default();
	let texts = vec!["First text".to_string(), "Second text".to_string(), "Third text".to_string()];

	// -- Exec
	let response = client.embed_batch(model, texts.clone(), None).await?;

	// -- Check Basic Properties
	assert_eq!(response.embedding_count(), 3);
	assert!(!response.is_single());
	assert!(response.is_batch());

	// -- Check Each Embedding
	for (i, embedding) in response.embeddings.iter().enumerate() {
		assert_eq!(embedding.index(), i);
		assert!(embedding.dimensions() > 0);
		assert_eq!(embedding.vector().len(), embedding.dimensions());
	}

	// -- Check Consistent Dimensions
	let first_dims = response.embeddings[0].dimensions();
	for embedding in &response.embeddings {
		assert_eq!(embedding.dimensions(), first_dims);
	}

	// -- Check Usage (provider-dependent)
	if expect_usage {
		let prompt_tokens = response.usage.prompt_tokens.ok_or("Should have prompt tokens")?;
		assert!(prompt_tokens > 0);
		println!(
			"✓ Batch embedding: {} texts, {} dimensions each, {} tokens",
			response.embedding_count(),
			first_dims,
			prompt_tokens
		);
	} else {
		// Some providers (like Gemini) don't provide usage information for embeddings
		assert!(response.usage.prompt_tokens.is_none());
		println!(
			"✓ Batch embedding: {} texts, {} dimensions each (no usage info)",
			response.embedding_count(),
			first_dims
		);
	}

	Ok(())
}

pub async fn common_test_embed_provider_specific_options_ok(
	model: &str,
	embedding_type: &str,
	truncate: Option<&str>,
) -> TestResult<()> {
	common_test_embed_provider_specific_options_ok_with_usage_check(model, embedding_type, truncate, true).await
}

pub async fn common_test_embed_provider_specific_options_ok_with_usage_check(
	model: &str,
	embedding_type: &str,
	truncate: Option<&str>,
	expect_usage: bool,
) -> TestResult<()> {
	// -- Setup & Fixtures
	let client = Client::default();
	let text = "Test with provider-specific options";

	let mut options = EmbedOptions::new()
		.with_dimensions(512)
		.with_capture_usage(true)
		.with_embedding_type(embedding_type);

	if let Some(truncate_val) = truncate {
		options = options.with_truncate(truncate_val);
	}

	// -- Exec
	let response = client.embed(model, text, Some(&options)).await?;

	// -- Check
	let embedding = response.first_embedding().ok_or("Should have embedding")?;
	assert!(embedding.dimensions() > 0);

	// Usage check (provider-dependent)
	if expect_usage {
		assert!(response.usage.prompt_tokens.is_some());
		println!(
			"✓ Provider-specific options: {} dimensions, embedding_type='{}'",
			embedding.dimensions(),
			embedding_type
		);
	} else {
		// Some providers (like Gemini) don't provide usage information for embeddings
		assert!(response.usage.prompt_tokens.is_none());
		println!(
			"✓ Provider-specific options: {} dimensions, embedding_type='{}' (no usage info)",
			embedding.dimensions(),
			embedding_type
		);
	}

	Ok(())
}

pub async fn common_test_embed_empty_batch_should_fail(model: &str) -> TestResult<()> {
	// -- Setup & Fixtures
	let client = Client::default();
	let texts: Vec<String> = vec![];

	// -- Exec
	let result = client.embed_batch(model, texts, None).await;

	// -- Check
	assert!(result.is_err(), "Empty batch should fail");

	println!("✓ Empty batch correctly failed");

	Ok(())
}

// endregion: --- Embeddings

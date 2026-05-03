//! Example demonstrating prompt caching with mixed TTLs (1h + 5m).
//!
//! This example shows how to:
//! 1. Use `CacheControl::Ephemeral1h` and `CacheControl::Ephemeral5m` on system messages
//! 2. Verify cache creation on the first request
//! 3. Verify cache hits on a subsequent identical request
//! 4. Inspect TTL-specific token breakdowns
//!
//! Requires: ANTHROPIC_API_KEY environment variable
//!
//! Run with: `cargo run --example c18-cache-ttl`

use genai::Client;
use genai::chat::{CacheControl, ChatMessage, ChatRequest};

const MODEL: &str = "claude-haiku-4-5-20251001";

/// Large text for the 1h-cached system message (~3000 tokens).
/// Anthropic requires a minimum of 2048 tokens per cacheable block for Haiku.
fn long_system_text() -> String {
	let paragraph = "The field of artificial intelligence has seen remarkable progress over \
		the past decade. Machine learning models have grown from simple classifiers to complex \
		systems capable of generating human-like text, creating images from descriptions, and \
		solving intricate reasoning problems. These advances have been driven by improvements \
		in hardware, the availability of massive datasets, and breakthroughs in model \
		architectures such as transformers. The transformer architecture, introduced in 2017, \
		revolutionized natural language processing by enabling models to attend to all parts of \
		an input sequence simultaneously, rather than processing tokens one at a time. This \
		parallel processing capability, combined with scaling laws that showed predictable \
		performance improvements with increased compute and data, led to the development of \
		large language models that can perform a wide variety of tasks. ";
	// Repeat enough to exceed 4096 tokens (Haiku 4.5 minimum for caching)
	paragraph.repeat(40)
}

/// Large text for the 5m-cached system message (~3000 tokens).
fn medium_system_text() -> String {
	let paragraph = "When responding to user queries, always provide clear, accurate, and \
		well-structured answers. Break down complex topics into digestible parts. Use examples \
		where appropriate to illustrate concepts. Maintain a professional and helpful tone \
		throughout the conversation. If you are unsure about something, say so rather than \
		guessing. Cite sources when possible. Keep responses concise but thorough. ";
	// Repeat enough to exceed 4096 tokens (Haiku 4.5 minimum for caching)
	paragraph.repeat(55)
}

fn build_chat_request(user_msg: &str) -> ChatRequest {
	let sys1 = ChatMessage::system(long_system_text()).with_options(CacheControl::Ephemeral1h);
	let sys2 = ChatMessage::system(medium_system_text()).with_options(CacheControl::Ephemeral5m);

	ChatRequest::default()
		.append_message(sys1)
		.append_message(sys2)
		.append_message(ChatMessage::user(user_msg))
}

fn get_or_zero(val: Option<i32>) -> i32 {
	val.unwrap_or(0)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let client = Client::default();
	let mut all_passed = true;

	// -- Request 1: Cache creation
	println!("=== Request 1: Cache Creation ===\n");

	let req = build_chat_request("What is 2+2?");
	let res = client.exec_chat(MODEL, req, None).await?;

	if let Some(text) = res.content.first_text() {
		println!("Response: {}\n", text);
	}

	let usage = &res.usage;
	let details = usage.prompt_tokens_details.as_ref();

	let prompt_tokens = get_or_zero(usage.prompt_tokens);
	let completion_tokens = get_or_zero(usage.completion_tokens);
	let total_tokens = get_or_zero(usage.total_tokens);
	let cache_creation_tokens = get_or_zero(details.and_then(|d| d.cache_creation_tokens));
	let cached_tokens = get_or_zero(details.and_then(|d| d.cached_tokens));
	let eph_1h = get_or_zero(
		details
			.and_then(|d| d.cache_creation_details.as_ref())
			.and_then(|cd| cd.ephemeral_1h_tokens),
	);
	let eph_5m = get_or_zero(
		details
			.and_then(|d| d.cache_creation_details.as_ref())
			.and_then(|cd| cd.ephemeral_5m_tokens),
	);

	println!("  prompt_tokens:          {prompt_tokens}");
	println!("  completion_tokens:      {completion_tokens}");
	println!("  total_tokens:           {total_tokens}");
	println!("  cache_creation_tokens:  {cache_creation_tokens}");
	println!("  cached_tokens:          {cached_tokens}");
	println!("  ephemeral_1h_tokens:    {eph_1h}");
	println!("  ephemeral_5m_tokens:    {eph_5m}");
	println!();

	// Verify creation request
	if cache_creation_tokens <= 0 {
		println!("  FAIL: cache_creation_tokens should be > 0");
		all_passed = false;
	}
	if cached_tokens != 0 {
		println!("  FAIL: cached_tokens should be 0 on first request, got {cached_tokens}");
		all_passed = false;
	}
	if eph_1h <= 0 {
		println!("  FAIL: ephemeral_1h_tokens should be > 0");
		all_passed = false;
	}
	if eph_5m <= 0 {
		println!("  FAIL: ephemeral_5m_tokens should be > 0");
		all_passed = false;
	}

	// -- Request 2: Cache read
	println!("=== Request 2: Cache Read ===\n");

	let req = build_chat_request("What is 3+3?");
	let res = client.exec_chat(MODEL, req, None).await?;

	if let Some(text) = res.content.first_text() {
		println!("Response: {}\n", text);
	}

	let usage = &res.usage;
	let details = usage.prompt_tokens_details.as_ref();

	let prompt_tokens = get_or_zero(usage.prompt_tokens);
	let completion_tokens = get_or_zero(usage.completion_tokens);
	let total_tokens = get_or_zero(usage.total_tokens);
	let cache_creation_tokens = get_or_zero(details.and_then(|d| d.cache_creation_tokens));
	let cached_tokens = get_or_zero(details.and_then(|d| d.cached_tokens));
	let eph_1h = get_or_zero(
		details
			.and_then(|d| d.cache_creation_details.as_ref())
			.and_then(|cd| cd.ephemeral_1h_tokens),
	);
	let eph_5m = get_or_zero(
		details
			.and_then(|d| d.cache_creation_details.as_ref())
			.and_then(|cd| cd.ephemeral_5m_tokens),
	);

	println!("  prompt_tokens:          {prompt_tokens}");
	println!("  completion_tokens:      {completion_tokens}");
	println!("  total_tokens:           {total_tokens}");
	println!("  cache_creation_tokens:  {cache_creation_tokens}");
	println!("  cached_tokens:          {cached_tokens}");
	println!("  ephemeral_1h_tokens:    {eph_1h}");
	println!("  ephemeral_5m_tokens:    {eph_5m}");
	println!();

	// Verify cache hit
	if cached_tokens <= 0 {
		println!("  FAIL: cached_tokens should be > 0 (cache hit)");
		all_passed = false;
	}
	if cache_creation_tokens != 0 {
		println!("  FAIL: cache_creation_tokens should be 0 on cache hit, got {cache_creation_tokens}");
		all_passed = false;
	}

	// -- Final result
	println!();
	if all_passed {
		println!("Cache TTL test PASSED");
	} else {
		println!("Cache TTL test FAILED");
	}

	Ok(())
}

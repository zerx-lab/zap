//! ZAI (Zhipu AI) adapter example
//!
//! Demonstrates how to use ZAI models with automatic endpoint routing:
//! - `glm-4.6` → Regular credit-based API
//! - `zai::glm-4.6` → Coding subscription API (automatically routed)

use genai::Client;
use genai::chat::{ChatMessage, ChatRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let client = Client::builder().build();

	// Test cases demonstrating automatic endpoint routing
	let test_cases = vec![
		("glm-4.6", "Regular ZAI model"),
		("zai-coding::glm-4.6", "Coding subscription model"),
	];

	for (model_name, description) in test_cases {
		println!("\n=== {} ===", description);
		println!("Model: {}", model_name);

		let chat_req = ChatRequest::default()
			.with_system("You are a helpful assistant.")
			.append_message(ChatMessage::user("Say 'hello' and nothing else."));

		match client.exec_chat(model_name, chat_req, None).await {
			Ok(response) => {
				println!("✅ Success!");
				if let Some(content) = response.first_text() {
					println!("Response: {}", content);
				}
				if response.usage.prompt_tokens.is_some() || response.usage.completion_tokens.is_some() {
					println!(
						"Usage: prompt={}, output={}",
						response.usage.prompt_tokens.unwrap_or(0),
						response.usage.completion_tokens.unwrap_or(0)
					);
				}
			}
			Err(e) => {
				println!("❌ Error: {}", e);
				if e.to_string().contains("insufficient balance") {
					println!("ℹ️  This model requires credits or subscription");
				} else if e.to_string().contains("401") {
					println!("ℹ️  Set ZAI_API_KEY environment variable");
				}
			}
		}
	}

	println!("\n=== SUMMARY ===");
	println!("✅ ZAI adapter handles namespace routing automatically");
	println!("✅ Use ZAI_API_KEY environment variable");

	Ok(())
}

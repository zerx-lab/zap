//! This example demonstrates how to use a custom ServiceTargetResolver which gives full control over the final
//! mapping for Endpoint, Model/AdapterKind, and Auth.
//!
//! IMPORTANT - Here we are using xAI as an example of a custom ServiceTarget.
//!             However, there is now an XaiAdapter, which gets activated on `starts_with("grok")`.

use genai::adapter::AdapterKind;
use genai::chat::{ChatMessage, ChatOptions, ChatRequest};
use genai::resolver::{AuthData, Endpoint, ServiceTargetResolver};
use genai::{Client, ModelIden, ServiceTarget};
use tracing_subscriber::EnvFilter;

// for fireworks: "https://api.fireworks.ai/inference/v1/",  "accounts/fireworks/models/qwen3-30b-a3b"

const MODEL: &str = "meta-llama/Llama-3-8b-chat-hf";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	tracing_subscriber::fmt()
		.with_env_filter(EnvFilter::new("genai=debug"))
		// .with_max_level(tracing::Level::DEBUG) // To enable all sub-library tracing
		.init();

	let questions = &[
		// Follow-up questions
		"Why is the sky blue?",
		"Why is it red sometimes?",
	];

	// -- Build an auth_resolver and the AdapterConfig
	let target_resolver = ServiceTargetResolver::from_resolver_fn(
		|service_target: ServiceTarget| -> Result<ServiceTarget, genai::resolver::Error> {
			let ServiceTarget { model, .. } = service_target;
			let endpoint = Endpoint::from_static("https://api.together.xyz/v1/");
			let auth = AuthData::from_env("TOGETHER_API_KEY");
			let model = ModelIden::new(AdapterKind::OpenAI, model.model_name);
			// TODO: point to xai
			Ok(ServiceTarget { endpoint, auth, model })
		},
	);

	// -- Build the new client with this adapter_config
	let client = Client::builder().with_service_target_resolver(target_resolver).build();

	// -- Normalize the eventual reasoning content (fireworks use the <think></think> style)
	let chat_options = ChatOptions::default().with_normalize_reasoning_content(true);

	let mut chat_req = ChatRequest::default().with_system("Answer in one sentence");

	for &question in questions {
		chat_req = chat_req.append_message(ChatMessage::user(question));

		println!("\n--- Question:\n{question}");
		let chat_res = client.exec_chat(MODEL, chat_req.clone(), Some(&chat_options)).await?;

		if let Some(reasoning_content) = chat_res.reasoning_content.as_deref() {
			println!("\n--- Reasoning:\n{reasoning_content}")
		}

		println!("\n--- Answer: ");
		let assistant_answer = chat_res.first_text().ok_or("Should have response")?;
		println!("{assistant_answer}");

		chat_req = chat_req.append_message(ChatMessage::assistant(assistant_answer));
	}

	Ok(())
}

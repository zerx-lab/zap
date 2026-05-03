//! Example demonstrating how to create a conversation with GenAI.

use genai::Client;
use genai::chat::printer::print_chat_stream;
use genai::chat::{ChatMessage, ChatRequest};
use tracing_subscriber::EnvFilter;

const MODEL: &str = "gpt-4o-mini";

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

	let client = Client::default();

	let mut chat_req = ChatRequest::default().with_system("Answer in one sentence");
	// This is similar to sending initial system chat messages (which will be cumulative with system chat messages)

	for &question in questions {
		chat_req = chat_req.append_message(ChatMessage::user(question));

		println!("\n--- Question:\n{question}");
		let chat_res = client.exec_chat_stream(MODEL, chat_req.clone(), None).await?;

		println!("\n--- Answer: (streaming)");
		let assistant_answer = print_chat_stream(chat_res, None).await?;

		chat_req = chat_req.append_message(ChatMessage::assistant(assistant_answer));
	}

	Ok(())
}

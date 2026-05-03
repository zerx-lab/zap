use genai::Client;
use genai::chat::printer::{PrintChatStreamOptions, print_chat_stream};
use genai::chat::{ChatMessage, ChatRequest};

// Ollama Cloud is the hosted Ollama service at ollama.com.
// It uses the same native Ollama protocol as local Ollama, but authenticates with a Bearer token.
// Requires OLLAMA_API_KEY environment variable.
//
// Use `ollama_cloud::` namespace to route to the cloud instead of local Ollama:
//   "ollama_cloud::gemma3:4b"
//   "ollama_cloud::gpt-oss:120b"
//   "ollama_cloud::deepseek-v3.2"
const MODEL: &str = "ollama_cloud::gemma3:4b";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let question = "Why is the sky blue?";

	let chat_req = ChatRequest::new(vec![
		ChatMessage::system("Answer in one sentence"),
		ChatMessage::user(question),
	]);

	let client = Client::default();

	let adapter_kind = client.resolve_service_target(MODEL).await?.model.adapter_kind;

	println!("\n===== MODEL: {MODEL} ({adapter_kind}) =====");

	println!("\n--- Question:\n{question}");

	println!("\n--- Answer:");
	let chat_res = client.exec_chat(MODEL, chat_req.clone(), None).await?;
	println!("{}", chat_res.first_text().unwrap_or("NO ANSWER"));

	println!("\n--- Answer: (streaming)");
	let chat_res = client.exec_chat_stream(MODEL, chat_req.clone(), None).await?;
	let print_options = PrintChatStreamOptions::from_print_events(false);
	print_chat_stream(chat_res, Some(&print_options)).await?;

	println!();

	Ok(())
}

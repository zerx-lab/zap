//! This example demonstrates how to properly attach image to the conversations

use genai::Client;
use genai::chat::{ChatMessage, ChatRequest, ContentPart};
use tracing_subscriber::EnvFilter;

const MODEL: &str = "gpt-5.1";
// const MODEL: &str = "claude-sonnet-4-5";

// const IMAGE_URL: &str = "https://aipack.ai/images/test-duck.jpg";

const IMAGE_SOME_PATH: &str = "tests/data/duck-small.jpg";
const IMAGE_OTHER_ONE_PATH: &str = "tests/data/other-one.png";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	tracing_subscriber::fmt()
		.with_env_filter(EnvFilter::new("genai=debug"))
		// .with_max_level(tracing::Level::DEBUG) // To enable all sub-library tracing
		.init();

	let client = Client::default();

	let mut chat_req = ChatRequest::default().with_system("Answer in one sentence");
	chat_req = chat_req
		.append_message(ChatMessage::user(vec![
			ContentPart::from_text("here is the file: 'some-image.jpg'"), // To test when name is different, should take precedence
			ContentPart::from_binary_file(IMAGE_SOME_PATH)?,
		]))
		.append_message(ChatMessage::user(vec![
			ContentPart::from_text("here is the file: 'other-one.png'"), // this is the most model portable way to provide image name/info
			ContentPart::from_binary_file(IMAGE_OTHER_ONE_PATH)?,
		]));

	let questions = [
		"What is the first image about? and what is the file name for this image if you have it?",
		"What is the second image about? and what is the file name for this image if you have it?",
	];

	for question in questions {
		println!("\nQuestion: {question}");

		let chat_req = chat_req.clone().append_message(ChatMessage::user(question));
		let chat_res = client.exec_chat(MODEL, chat_req, None).await?;

		let usage = chat_res.usage;
		let response_content = chat_res.content.joined_texts().ok_or("Should have response")?;

		println!("\nAnswer: {response_content}");
		println!(
			"prompt: {:?} tokens  |  completion: {:?} tokens",
			usage.prompt_tokens, usage.completion_tokens
		);

		println!();
	}

	// NOTE: For web url image, we can `from_binary_url` but not supported by all models (e.g., Anthropic does not support those)
	// ContentPart::from_binary_url(
	// 			"image/jpg",
	// 			IMAGE_URL,
	// 			None,
	// 		)

	Ok(())
}

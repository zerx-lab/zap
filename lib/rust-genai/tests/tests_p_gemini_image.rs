use genai::Client;
use genai::chat::{ChatMessage, ChatRequest};

const MODEL: &str = "gemini-3-pro-image-preview";

#[tokio::test]
async fn test_p_gemini_image_generation() -> Result<(), Box<dyn std::error::Error>> {
	let client = Client::default();
	let prompt = "Generate a small icon of a star";
	let chat_req = ChatRequest::new(vec![ChatMessage::user(prompt)]);

	let chat_res = client.exec_chat(MODEL, chat_req, None).await?;

	// Check if we got any binary parts
	let has_image = chat_res.content.into_iter().any(|part| part.is_image());

	// Note: Since this is a provider test, it depends on API availability.
	// We expect an image if the model is working correctly for image generation.
	println!("Has image: {}", has_image);

	assert!(
		has_image,
		"Expected to receive an image content part from the Gemini response"
	);

	Ok(())
}

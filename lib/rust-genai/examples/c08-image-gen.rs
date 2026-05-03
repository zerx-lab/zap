//! This example demonstrates how to use the Gemini model to generate images and save them locally.

use genai::Client;
use genai::chat::{ChatMessage, ChatRequest};
use std::fs;

const MODEL: &str = "gemini-3-pro-image-preview"; // Or other Gemini model that supports image generation

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	// Reads GEMINI_API_KEY from environment variable by default
	let client = Client::default();

	let prompt = "Generate a picture of a cat, cartoon style, 512x512 resolution";
	println!("Prompt: {prompt}");

	let chat_req = ChatRequest::new(vec![ChatMessage::user(prompt)]);

	// Call exec_chat to get the response
	let chat_res = client.exec_chat(MODEL, chat_req, None).await?;

	// Check if the response contains binary data (image)
	let mut image_count = 0;
	for part in &chat_res.content {
		if let Some(binary) = part.as_binary() {
			// For Gemini, the binary content is expected to be the complete image data with proper MIME type, so we can directly save it as a file.
			if binary.content_type.starts_with("image/") {
				image_count += 1;
				let file_ext = match binary.content_type.as_str() {
					"image/png" => "png",
					"image/jpeg" => "jpg",
					"image/webp" => "webp",
					_ => "bin",
				};
				let filename = format!("generated_image_{}.{}", image_count, file_ext);

				// In rust-genai, BinarySource::Base64 holds the base64 string
				use genai::chat::BinarySource;
				let data = match &binary.source {
					BinarySource::Base64(base64_str) => {
						use base64::{Engine as _, engine::general_purpose};
						general_purpose::STANDARD.decode(base64_str.as_ref())?
					}
					_ => continue,
				};

				fs::write(&filename, data)?;
				println!("Image saved to: {}", filename);
			}
		}
	}

	if image_count == 0 {
		if let Some(text) = chat_res.first_text() {
			println!("No image generated. Model response: {}", text);
		} else {
			println!("No image generated and no text response.");
		}
	}

	Ok(())
}

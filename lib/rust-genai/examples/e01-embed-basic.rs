//! Basic example demonstrating the embedding API
//!
//! This example shows how to:
//! - Create single embeddings
//! - Create batch embeddings
//! - Use embedding options
//! - Handle different providers

use genai::Client;
use genai::embed::{EmbedOptions, EmbedRequest};
use tracing_subscriber::EnvFilter;

// OpenAI embedding models
const MODEL_OPENAI_SMALL: &str = "text-embedding-3-small";
const MODEL_OPENAI_LARGE: &str = "text-embedding-3-large";

// Other providers (will return "not supported" errors for now)
const MODEL_COHERE: &str = "embed-v4.0";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	tracing_subscriber::fmt()
		.with_env_filter(EnvFilter::new("genai=debug"))
		// .with_max_level(tracing::Level::DEBUG) // To enable all sub-library tracing
		.init();

	let client = Client::default();

	println!("=== GenAI Embedding Examples ===\n");

	// Example 1: Single embedding
	println!("1. Single Embedding:");
	let text = "The quick brown fox jumps over the lazy dog";
	println!("   Text: \"{text}\"",);

	match client.embed(MODEL_OPENAI_SMALL, text, None).await {
		Ok(response) => {
			let embedding = response.first_embedding().ok_or("should have embedding")?;
			println!("   Model: {}", response.model_iden.model_name);
			println!("   Dimensions: {}", embedding.dimensions());
			println!(
				"   Vector preview: [{:.4}, {:.4}, {:.4}, ...]",
				embedding.vector()[0],
				embedding.vector()[1],
				embedding.vector()[2]
			);
			if let Some(usage) = response.usage.prompt_tokens {
				println!("   Tokens used: {usage}",);
			}
		}
		Err(e) => println!("   Error: {e}",),
	}
	println!();

	// Example 2: Batch embeddings
	println!("2. Batch Embeddings:");
	let texts = vec![
		"Hello world".to_string(),
		"Goodbye world".to_string(),
		"The meaning of life".to_string(),
	];
	println!("   Texts: {texts:?}",);

	match client.embed_batch(MODEL_OPENAI_SMALL, texts, None).await {
		Ok(response) => {
			println!("   Model: {}", response.model_iden.model_name);
			println!("   Number of embeddings: {}", response.embedding_count());
			for (i, embedding) in response.embeddings.iter().enumerate() {
				println!(
					"   Embedding {}: {} dimensions, vector preview: [{:.4}, {:.4}, ...]",
					i,
					embedding.dimensions(),
					embedding.vector()[0],
					embedding.vector()[1]
				);
			}
			if let Some(usage) = response.usage.prompt_tokens {
				println!("   Total tokens used: {usage}",);
			}
		}
		Err(e) => println!("   Error: {e}",),
	}
	println!();

	// Example 3: Using EmbedRequest directly
	println!("3. Using EmbedRequest:");
	let embed_req = EmbedRequest::from_texts(vec![
		"Machine learning".to_string(),
		"Artificial intelligence".to_string(),
	]);

	match client.exec_embed(MODEL_OPENAI_SMALL, embed_req, None).await {
		Ok(response) => {
			println!("   Created {} embeddings", response.embedding_count());
			for embedding in &response.embeddings {
				println!("   Index {}: {} dimensions", embedding.index(), embedding.dimensions());
			}
		}
		Err(e) => println!("   Error: {e}",),
	}
	println!();

	// Example 4: Using embedding options
	println!("4. With Embedding Options:");
	let options = EmbedOptions::new()
		.with_dimensions(512) // Request smaller dimensions (if supported)
		.with_capture_usage(true)
		.with_user("example-user".to_string());

	match client.embed(MODEL_OPENAI_SMALL, "Hello with options", Some(&options)).await {
		Ok(response) => {
			let embedding = response.first_embedding().ok_or("Should have embedding")?;
			println!("   Requested dimensions: 512");
			println!("   Actual dimensions: {}", embedding.dimensions());
			println!(
				"   Vector preview: [{:.4}, {:.4}, {:.4}, ...]",
				embedding.vector()[0],
				embedding.vector()[1],
				embedding.vector()[2]
			);
		}
		Err(e) => println!("   Error: {e}",),
	}
	println!();

	// Example 5: Provider-specific options
	println!("5. Provider-Specific Options:");

	// Cohere-specific options
	let cohere_options = EmbedOptions::new()
		.with_dimensions(512)
		.with_embedding_type("search_query") // Cohere: specify embedding type
		.with_truncate("START") // Cohere: truncate from start instead of end
		.with_capture_usage(true);

	println!("   Cohere options: embedding_type='search_query', truncate='START'");
	match client
		.embed(MODEL_COHERE, "What is machine learning?", Some(&cohere_options))
		.await
	{
		Ok(response) => {
			let embedding = response.first_embedding().ok_or("Should have embedding")?;
			println!("   ✓ Cohere embedding: {} dimensions", embedding.dimensions());
		}
		Err(e) => println!("   ✗ Cohere error: {e}",),
	}

	// Gemini-specific options
	let gemini_options = EmbedOptions::new()
		.with_embedding_type("RETRIEVAL_QUERY") // Gemini: specify embedding type
		.with_capture_usage(true);

	println!("   Gemini options: embedding_type='RETRIEVAL_QUERY'");
	match client
		.embed("gemini-embedding-001", "Find documents about AI", Some(&gemini_options))
		.await
	{
		Ok(response) => {
			let embedding = response.first_embedding().ok_or("Should have embedding")?;
			println!("   ✓ Gemini embedding: {} dimensions", embedding.dimensions());
		}
		Err(e) => println!("   ✗ Gemini error: {e}",),
	}
	println!();

	// Example 6: Different models (if available)
	println!("6. Different Models:");
	let test_text = "Compare embedding models";

	for model in &[MODEL_OPENAI_SMALL, MODEL_OPENAI_LARGE] {
		print!("   Testing {model}: ",);
		match client.embed(model, test_text, None).await {
			Ok(response) => {
				let embedding = response.first_embedding().ok_or("Should have embedding")?;
				println!("{} dimensions", embedding.dimensions());
			}
			Err(e) => println!("Error - {e}",),
		}
	}
	println!();

	Ok(())
}

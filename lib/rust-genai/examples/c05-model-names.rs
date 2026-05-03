//! Example showing how to get the list of models per AdapterKind
//! Note: Currently, only Ollama and Ollama Cloud make a dynamic query. Other adapters have a static list of models.

use genai::Client;
use genai::adapter::AdapterKind;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	tracing_subscriber::fmt()
		.with_env_filter(EnvFilter::new("genai=debug"))
		// .with_max_level(tracing::Level::DEBUG) // To enable all sub-library tracing
		.init();

	const KINDS: &[AdapterKind] = &[
		AdapterKind::OpenAI,
		AdapterKind::Ollama,
		AdapterKind::Gemini,
		AdapterKind::Anthropic,
		AdapterKind::Groq,
		AdapterKind::Cohere,
	];

	let client = Client::default();

	for &kind in KINDS {
		println!("\n--- Models for {kind}");
		let models = client.all_model_names(kind).await?;
		println!("{models:?}");
	}

	Ok(())
}

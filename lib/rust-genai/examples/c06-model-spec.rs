//! This example shows how to use a custom AdapterKindResolver to have some custom
//! mapping from a model name to an AdapterKind.
//! This allows mapping missing models to their Adapter implementations.

use genai::adapter::AdapterKind;
use genai::chat::{ChatMessage, ChatRequest};
use genai::resolver::{AuthData, Endpoint};
use genai::{Client, ModelIden, ModelSpec, ServiceTarget};
use tracing_subscriber::EnvFilter;

pub enum AppModel {
	Fast,
	Pro,
	Local,
	Custom(String),
}

impl From<&AppModel> for ModelSpec {
	fn from(model: &AppModel) -> Self {
		match model {
			AppModel::Fast => ModelSpec::from_static_name("gemini-3-flash-preview"),

			// ModelName will be Arc<str> (use `ModelIden::from_static(..) for micro optimization)
			AppModel::Pro => ModelSpec::from_iden((AdapterKind::Anthropic, "claude-opus-4-5")),

			AppModel::Local => ModelSpec::Target(ServiceTarget {
				model: ModelIden::from_static(AdapterKind::Ollama, "gemma3:1b"),
				endpoint: Endpoint::from_static("http://localhost:11434"),
				auth: AuthData::Key("".to_string()),
			}),

			AppModel::Custom(name) => ModelSpec::from_name(name),
		}
	}
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	tracing_subscriber::fmt()
		.with_env_filter(EnvFilter::new("genai=debug"))
		// .with_max_level(tracing::Level::DEBUG) // To enable all sub-library tracing
		.init();

	// -- Model Spec (unselect one below)
	let model_spec = AppModel::Fast;
	// let model_spec = AppModel::Custom("gpt-5.2".to_string());

	let question = "Why is the sky red? (be concise)";

	// -- Build the new client with this client_config
	let client = Client::default();

	// -- Build the chat request
	let chat_req = ChatRequest::new(vec![ChatMessage::user(question)]);

	// -- Execute and print
	println!("\n--- Question:\n{question}");
	let chat_res = client.exec_chat(&model_spec, chat_req.clone(), None).await?;

	let model_iden = chat_res.model_iden;
	let res_txt = chat_res.content.into_joined_texts().ok_or("Should have some response")?;

	println!("\n--- Answer: ({model_iden})\n{res_txt}");

	Ok(())
}

//! Base examples demonstrating the core capabilities of genai

use genai::Client;
use genai::chat::printer::{PrintChatStreamOptions, print_chat_stream};
use genai::chat::{ChatMessage, ChatRequest};
use tracing_subscriber::EnvFilter;

const MODEL_OPENAI: &str = "gpt-5.4-mini"; //
const MODEL_ANTHROPIC: &str = "claude-3-haiku-20240307";
// or namespaced with simple anme "fireworks::qwen3-30b-a3b", or "fireworks::accounts/fireworks/models/qwen3-30b-a3b"
const MODEL_FIREWORKS: &str = "fireworks::gpt-oss-20b";
const MODEL_TOGETHER: &str = "together::openai/gpt-oss-20b";
const MODEL_GEMINI: &str = "gemini-2.0-flash";
const MODEL_GROQ: &str = "groq::openai/gpt-oss-20b";
const MODEL_OLLAMA: &str = "gemma4:e2b"; // sh: `ollama pull gemma:2b`
const MODEL_OLLAMA_CLOUD: &str = "ollama_cloud::gemma3:4b";
const MODEL_XAI: &str = "grok-3-mini";
const MODEL_DEEPSEEK: &str = "deepseek-chat";
const MODEL_ZAI: &str = "glm-4-plus";
const MODEL_ALIYUN: &str = "aliyun::qwen-plus"; // required namespace
const MODEL_COHERE: &str = "command-r7b-12-2024";
// or any publisher: "github_copilot::anthropic/claude-sonnet-4-6", "github_copilot::google/gemini-2.5-pro", "github_copilot::xai/grok-3-mini"
const MODEL_GITHUB_COPILOT: &str = "github_copilot::openai/gpt-4.1-mini";

// NOTE: These are the default environment keys for each AI Adapter Type.
//       They can be customized; see `examples/c02-auth.rs`
const MODEL_AND_KEY_ENV_NAME_LIST: &[(&str, &str)] = &[
	// -- De/activate models/providers
	(MODEL_OPENAI, "OPENAI_API_KEY"),
	(MODEL_ANTHROPIC, "ANTHROPIC_API_KEY"),
	(MODEL_GEMINI, "GEMINI_API_KEY"),
	(MODEL_OLLAMA, ""),
	(MODEL_OLLAMA_CLOUD, "OLLAMA_API_KEY"),
	(MODEL_FIREWORKS, "FIREWORKS_API_KEY"),
	(MODEL_TOGETHER, "TOGETHER_API_KEY"),
	(MODEL_GROQ, "GROQ_API_KEY"),
	(MODEL_XAI, "XAI_API_KEY"),
	(MODEL_DEEPSEEK, "DEEPSEEK_API_KEY"),
	(MODEL_ALIYUN, "ALIYUN_API_KEY"),
	(MODEL_ZAI, "ZAI_API_KEY"),
	(MODEL_COHERE, "COHERE_API_KEY"),
	(MODEL_GITHUB_COPILOT, "GITHUB_TOKEN"),
];

// NOTE: Model to AdapterKind (AI Provider) type mapping rule
//  - starts_with "gpt"      -> OpenAI
//  - starts_with "claude"   -> Anthropic
//  - starts_with "command"  -> Cohere
//  - starts_with "gemini"   -> Gemini
//  - model in Groq models   -> Groq
//  - starts_with "glm"      -> ZAI
//  - starts_with "ollama_cloud::" -> OllamaCloud
//  - For anything else      -> Ollama
//
// This can be customized; see `examples/c03-mapper.rs`

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	tracing_subscriber::fmt()
		.with_env_filter(EnvFilter::new("genai=debug"))
		// .with_max_level(tracing::Level::DEBUG) // To enable all sub-library tracing
		.init();

	let question = "Why is the sky red?";

	let chat_req = ChatRequest::new(vec![
		// -- Messages (de/activate to see the differences)
		ChatMessage::system("Answer in one sentence"),
		ChatMessage::user(question),
	]);

	let client = Client::default();

	let print_options = PrintChatStreamOptions::from_print_events(false);

	for (model, env_name) in MODEL_AND_KEY_ENV_NAME_LIST {
		// Skip if the environment name is not set
		if !env_name.is_empty() && std::env::var(env_name).is_err() {
			println!("===== Skipping model: {model} (env var not set: {env_name})");
			continue;
		}

		let adapter_kind = client.resolve_service_target(model).await?.model.adapter_kind;

		println!("\n===== MODEL: {model} ({adapter_kind}) =====");

		println!("\n--- Question:\n{question}");

		println!("\n--- Answer:");
		let chat_res = client.exec_chat(model, chat_req.clone(), None).await?;
		println!("{}", chat_res.first_text().unwrap_or("NO ANSWER"));

		println!("\n--- Answer: (streaming)");
		let chat_res = client.exec_chat_stream(model, chat_req.clone(), None).await?;
		print_chat_stream(chat_res, Some(&print_options)).await?;

		println!();
	}

	Ok(())
}

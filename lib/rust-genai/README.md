# genai, Multi-AI Providers Library for Rust

Currently natively supports: **OpenAI**, **OpenAI Responses**, **Anthropic**, **Gemini**, **xAI**, **Ollama**, **Ollama Cloud**, **Groq**, **DeepSeek**, **Cohere**, **Together**, **Fireworks**, **Nebius**, **Mimo**, **Zai** (Zhipu AI), **BigModel**, **Aliyun**, **Google Vertex**, **GitHub Copilot** (GitHub Models API).

Also supports a custom URL with `ServiceTargetResolver` (see [examples/c06-target-resolver.rs](examples/c06-target-resolver.rs)).

<div align="center">

<a href="https://crates.io/crates/genai"><img src="https://img.shields.io/crates/v/genai.svg" /></a>
<a href="https://github.com/jeremychone/rust-genai"><img alt="Static Badge" src="https://img.shields.io/badge/GitHub-Repo?color=%23336699"></a>
<a href="https://www.youtube.com/watch?v=uqGso3JD3eE&list=PL7r-PXl6ZPcBcLsBdBABOFUuLziNyigqj"><img alt="Static Badge" src="https://img.shields.io/badge/YouTube_genai_Intro-Video?style=flat&logo=youtube&color=%23ff0000"></a>

</div>

Provides a single, ergonomic API for many generative AI providers, such as Anthropic, OpenAI, Gemini, xAI, Ollama, Groq, and more.

**NOTE:** Big update with **v0.5.0**: New adapters (BigModel, MIMO), Gemini Thinking support, Anthropic Reasoning Effort, and a more robust internal streaming engine.

[Docs for LLMs](docs/for-llm/api-reference-for-llm.md) | [CHANGELOG](CHANGELOG.md) | [BIG THANKS](BIG-THANKS.md)

## Model to Adapter Resolution

By default, the library resolves the `AdapterKind` (AI Provider) based on the model name prefix:

- **OpenAI**: `gpt-*` (most), `o1-*`, `o3-*`, `o4-*`, `chatgpt-*`, `codex-*`
- **OpenAI Responses**: `gpt-5-*`, `gpt-*` (containing `codex` or `pro`)
- **Anthropic**: `claude-*`
- **Gemini**: `gemini-*`
- **xAI**: `grok-*`
- **DeepSeek**: `deepseek-*`
- **Zai**: `glm-*`
- **Cohere**: `command-*`, `embed-*`
- **Mimo**: `mimo-*`
- **Fireworks**: contains `fireworks`
- **Ollama**: Fallback for any other names, which defaults to local Ollama.

### Namespacing (Forcing an Adapter)

You can force a specific adapter by using the `adapter_kind::model_name` syntax. This is the recommended way for many providers and for disambiguating OpenAI-compatible services.

- `groq::llama-3.1-8b-instant` (Forces **Groq** adapter)
- `together::meta-llama/Llama-3-8b-chat-hf` (Forces **Together** adapter)
- `ollama_cloud::gemma3:4b` (Forces **OllamaCloud** adapter)
- `github_copilot::openai/gpt-4.1-mini` (Forces **GithubCopilot** adapter)
- `nebius::Qwen/Qwen3-235B-A22B` (Forces **Nebius** adapter)
- `aliyun::qwen-plus` (Forces **Aliyun** adapter)
- `vertex::gemini-2.5-flash` (Forces **Google Vertex** adapter)
- `coding::glm-4.6` (Special namespace for **Zai** coding subscription)

For a complete list of `AdapterKind`, see the [AdapterKind enum](src/adapter/adapter_kind.rs).

## v0.5.x - (2026-01-09...)

- **What's new**:
    - **New Adapters**: BigModel.cn and the MIMO model adapter (thanks to [Akagi201](https://github.com/Akagi201)).
    - **zai: changed namespace strategy**, with (zai:: for default, and zai-codding:: for subscription, same adapter)
    - **Gemini Thinking & Thought**: Full support for Gemini Thought signatures (thanks to [Himmelschmidt](https://github.com/Himmelschmidt)) and thinking levels.
    - **Reasoning Effort Control**: Support for `ReasoningEffort` for Anthropic (Claude 3.7/4.5) and Gemini (Thinking levels), including `ReasoningEffort::None`.
    - **Content & Binary Improvements**: Enhanced binary/PDF API and size tracking.
    - **Internal Stream Refactor**: Switched to a unified `EventSourceStream` and `WebStream` for better reliability and performance across all providers.
    - **Dependency Upgrade**: Now using `reqwest 0.13`.
- **What's still awesome**:
    - Normalized and ergonomic Chat API across all major providers.
    - Native protocol support for Gemini and Anthropic protocols (Reasoning/Thinking controls).
    - PDF, image, and embedding support.
    - Custom auth, endpoint, and header overrides.

See [CHANGELOG](CHANGELOG.md)

## Usage examples

- Check out [AIPACK](https://aipack.ai), which wraps this **genai** library into an agentic runtime to run, build, and share AI Agent Packs. See [`pro@coder`](https://www.youtube.com/watch?v=zL1BzPVM8-Y&list=PL7r-PXl6ZPcB2zN0XHsYIDaD5yW8I40AE) for a simple example of how I use AI PACK/genai for production coding.

> Note: Feel free to send me a short description and a link to your application or library that uses genai.

## Key Features

- Native Multi-AI Provider/Model: OpenAI, OpenAI Responses, Anthropic, Gemini, Ollama, Ollama Cloud, Groq, xAI, DeepSeek, Cohere, Together, Fireworks, Nebius, Mimo, Zai, BigModel, Aliyun, Google Vertex, and GitHub Copilot (direct chat and streaming) (see [examples/c00-readme.rs](examples/c00-readme.rs))
- DeepSeekR1 support, with `reasoning_content` (and stream support), plus DeepSeek Groq and Ollama support (and `reasoning_content` normalization)
- Image Analysis (for OpenAI, Gemini flash-2, Anthropic) (see [examples/c07-image.rs](examples/c07-image.rs))
- Custom Auth/API Key (see [examples/c02-auth.rs](examples/c02-auth.rs))
- Model aliases (see [examples/c05-model-names.rs](examples/c05-model-names.rs))
- Custom endpoint, auth, and model identifier (see [examples/c06-target-resolver.rs](examples/c06-target-resolver.rs))

[Examples](#examples) | [Thanks](#thanks) | [Library Focus](#library-focus) | [Changelog](CHANGELOG.md) | Provider Mapping: [ChatOptions](#chatoptions) | [Usage](#usage)

## Examples

[examples/c00-readme.rs](examples/c00-readme.rs)

```rust
//! Base examples demonstrating the core capabilities of genai

use genai::chat::printer::{print_chat_stream, PrintChatStreamOptions};
use genai::chat::{ChatMessage, ChatRequest};
use genai::Client;

const MODEL_OPENAI: &str = "gpt-4o-mini"; // o1-mini, gpt-4o-mini
const MODEL_ANTHROPIC: &str = "claude-3-haiku-20240307";
// or namespaced with simple name "fireworks::qwen3-30b-a3b", or "fireworks::accounts/fireworks/models/qwen3-30b-a3b"
const MODEL_FIREWORKS: &str = "accounts/fireworks/models/qwen3-30b-a3b";
const MODEL_TOGETHER: &str = "together::openai/gpt-oss-20b";
const MODEL_GEMINI: &str = "gemini-2.0-flash";
const MODEL_GROQ: &str = "groq::llama-3.1-8b-instant";
const MODEL_OLLAMA: &str = "gemma:2b"; // sh: `ollama pull gemma:2b`
const MODEL_OLLAMA_CLOUD: &str = "ollama_cloud::gemma3:4b";
const MODEL_XAI: &str = "grok-3-mini";
const MODEL_DEEPSEEK: &str = "deepseek-chat";
const MODEL_ZAI: &str = "glm-4-plus";
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
	(MODEL_FIREWORKS, "FIREWORKS_API_KEY"),
	(MODEL_TOGETHER, "TOGETHER_API_KEY"),
	(MODEL_GROQ, "GROQ_API_KEY"),
	(MODEL_XAI, "XAI_API_KEY"),
	(MODEL_DEEPSEEK, "DEEPSEEK_API_KEY"),
	(MODEL_OLLAMA, ""),
	(MODEL_OLLAMA_CLOUD, "OLLAMA_API_KEY"),
	(MODEL_ZAI, "ZAI_API_KEY"),
	(MODEL_COHERE, "COHERE_API_KEY"),
	(MODEL_GITHUB_COPILOT, "GITHUB_TOKEN"),
];

// NOTE: Model to AdapterKind (AI Provider) type mapping rule
//  - starts_with "gpt"      -> OpenAI (or OpenAI Responses for gpt-5/codex/pro)
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
```

### More Examples

- [examples/c00-readme.rs](examples/c00-readme.rs) - Quick overview code with multiple providers and streaming.
- [examples/c01-conv.rs](examples/c01-conv.rs) - Shows how to build a conversation flow.
- [examples/c02-auth.rs](examples/c02-auth.rs) - Demonstrates how to provide a custom `AuthResolver` to provide auth data (i.e., for api_key) per adapter kind.
- [examples/c03-mapper.rs](examples/c03-mapper.rs) - Demonstrates how to provide a custom `AdapterKindResolver` to customize the "model name" to "adapter kind" mapping.
- [examples/c04-chat-options.rs](examples/c04-chat-options.rs) - Demonstrates how to set chat generation options such as `temperature` and `max_tokens` at the client level (for all requests) and at the per-request level.
- [examples/c05-model-names.rs](examples/c05-model-names.rs) - Shows how to get model names per AdapterKind.
- [examples/c06-target-resolver.rs](examples/c06-target-resolver.rs) - For custom auth, endpoint, and model.
- [examples/c07-image.rs](examples/c07-image.rs) - Image analysis support

<br />
<a href="https://www.youtube.com/playlist?list=PL7r-PXl6ZPcBcLsBdBABOFUuLziNyigqj"><img alt="Static Badge" src="https://img.shields.io/badge/YouTube_JC_AI_Playlist-Video?style=flat&logo=youtube&color=%23ff0000"></a>
<br />

- [genai ModelMapper code demo (v0.1.7)](https://www.youtube.com/watch?v=5Enfcwrl7pE&list=PL7r-PXl6ZPcBcLsBdBABOFUuLziNyigqj)

- [genai introduction (v0.1.0)](https://www.youtube.com/watch?v=uqGso3JD3eE&list=PL7r-PXl6ZPcBcLsBdBABOFUuLziNyigqj)

- **genai live coding, code design, & best practices**
    - [Adding **Gemini** Structured Output (vid-0060)](https://www.youtube.com/watch?v=GdFsqLJ1_pE&list=PL7r-PXl6ZPcBcLsBdBABOFUuLziNyigqj)
    - [Adding **OpenAI** Structured Output (vid-0059)](https://www.youtube.com/watch?v=FpoNbQMhAH8&list=PL7r-PXl6ZPcBcLsBdBABOFUuLziNyigqj)
    - [Splitting the json value extension trait to its own public crate value-ext](https://www.youtube.com/watch?v=OS5KOz9y7Cg&list=PL7r-PXl6ZPcBcLsBdBABOFUuLziNyigqj) [value-ext](https://crates.io/crates/value-ext)
    - [(part 1/3) Module, Error, constructors/builders](https://www.youtube.com/watch?v=XCrZleaIUO4&list=PL7r-PXl6ZPcBcLsBdBABOFUuLziNyigqj)
    - [(part 2/3) Extension Traits, Project Files, Versioning](https://www.youtube.com/watch?v=LRfDAZfo00o&list=PL7r-PXl6ZPcBcLsBdBABOFUuLziNyigqj)
    - [(part 3/3) When to Async? Project Files, Versioning strategy](https://www.youtube.com/watch?v=93SS3VGsKx4&list=PL7r-PXl6ZPcCIOFaL7nVHXZvBmHNhrh_Q)

## Library Focus:

- Focuses on standardizing chat completion APIs across major AI services.

- Native implementation, meaning no per-service SDKs.
    - Reason: While there are some variations across the various APIs, they all follow the same pattern and high-level flow and constructs. Managing the differences at a lower layer is actually simpler and more cumulative across services than doing SDK gymnastics.

- Prioritizes ergonomics and commonality, with depth being secondary. (If you require a complete client API, consider using [async-openai](https://crates.io/search?q=async-openai) and [ollama-rs](https://crates.io/crates/ollama-rs); they are both excellent and easy to use.)

- Initially, this library will mostly focus on text chat APIs, with images and function calling coming later.

## ChatOptions

- **(1)** - **OpenAI-compatible** notes
	- Models: OpenAI, DeepSeek, Groq, Ollama, xAI, Mimo, Together, Fireworks, Nebius, Zai, Together, Fireworks, Nebius, Zai

| Property      | OpenAI Compatibles (*1) | Anthropic                   | Gemini `generationConfig.` | Cohere        |
|---------------|-------------------------|-----------------------------|----------------------------|---------------|
| `temperature` | `temperature`           | `temperature`               | `temperature`              | `temperature` |
| `max_tokens`  | `max_tokens`            | `max_tokens` (default 1024) | `maxOutputTokens`          | `max_tokens`  |
| `top_p`       | `top_p`                 | `top_p`                     | `topP`                     | `p`           |

## Usage

| Property                    | OpenAI Compatibles (1)      | Anthropic `usage.`      | Gemini `usageMetadata.`    | Cohere `meta.tokens.` |
|-----------------------------|-----------------------------|-------------------------|----------------------------|-----------------------|
| `prompt_tokens`             | `prompt_tokens`             | `input_tokens` (added)  | `promptTokenCount` (2)     | `input_tokens`        |
| `completion_tokens`         | `completion_tokens`         | `output_tokens` (added) | `candidatesTokenCount` (2) | `output_tokens`       |
| `total_tokens`              | `total_tokens`              | (computed)              | `totalTokenCount` (2)      | (computed)            |
| `prompt_tokens_details`     | `prompt_tokens_details`     | `cached/cache_creation` | N/A for now                | N/A for now           |
| `completion_tokens_details` | `completion_tokens_details` | N/A for now             | N/A for now                | N/A for now           |

- **(1)** - **OpenAI-compatible** notes
	- Models: OpenAI, DeepSeek, Groq, Ollama, xAI, Mimo
	- For **Groq**, the property `x_groq.usage.`
	- At this point, **Ollama** does not emit input/output tokens when streaming due to a limitation in the Ollama OpenAI compatibility layer. (see [ollama #4448 - Streaming Chat Completion via OpenAI API should support stream option to include Usage](https://github.com/ollama/ollama/issues/4448))
	- `prompt_tokens_details` and `completion_tokens_details` will have the value sent by the compatible provider (or None)

- **(2)**: **Gemini** tokens
	- Right now, with the [Gemini Stream API](https://ai.google.dev/api/rest/v1beta/models/streamGenerateContent), it's not clear whether usage for each event is cumulative or must be summed. It appears to be cumulative, meaning the last message shows the total number of input, output, and total tokens, so that is the current assumption. See [possible tweet answer](https://twitter.com/jeremychone/status/1813734565967802859) for more info.

## Notes on Possible Direction

- Will add more data to ChatResponse and ChatStream, especially usage metadata.
- Add vision/image support to chat messages and responses.
- Add function calling support to chat messages and responses.
- Add `embed` and `embed_batch`.
- Add the AWS Bedrock variants (e.g., Mistral and Anthropic). Most of the work will be on the "interesting" token signature scheme. To avoid bringing in large SDKs, this might be a lower-priority feature.
- Add the Google Vertex AI variants.
- May add the Azure OpenAI variant (not sure yet).

## Links

- crates.io: [crates.io/crates/genai](https://crates.io/crates/genai)
- GitHub: [github.com/jeremychone/rust-genai](https://github.com/jeremychone/rust-genai)
- Sponsored by [BriteSnow](https://britesnow.com) (Jeremy Chone's consulting company)

//! Macros for adapter dispatching.

use paste::paste;

// Dispatch adapter method call based on `AdapterKind`, avoiding repeated `match` arms.
//
// Usage:
// ```ignore
// dispatch_adapter!(kind, A::some_method(args))
// ```
//
// Inside the provided expression, a type alias `A` is bound to the appropriate
// concrete adapter struct (e.g., `OpenAIAdapter`), allowing static dispatch.
//
// The macro contains the full mapping from every `AdapterKind` variant to
// its corresponding adapter struct, using fully qualified paths.

paste! {
macro_rules! dispatch_adapter {
	($kind:expr, $body:expr) => {
		match $kind {
			crate::adapter::AdapterKind::OpenAI => {
				type A = crate::adapter::openai::[<OpenAI Adapter>];
				$body
			}
			crate::adapter::AdapterKind::OpenAIResp => {
				type A = crate::adapter::adapters::openai_resp::[<OpenAIResp Adapter>];
				$body
			}
			crate::adapter::AdapterKind::Gemini => {
				type A = crate::adapter::gemini::[<Gemini Adapter>];
				$body
			}
			crate::adapter::AdapterKind::Anthropic => {
				type A = crate::adapter::anthropic::[<Anthropic Adapter>];
				$body
			}
			crate::adapter::AdapterKind::Fireworks => {
				type A = crate::adapter::fireworks::[<Fireworks Adapter>];
				$body
			}
			crate::adapter::AdapterKind::Together => {
				type A = crate::adapter::adapters::together::[<Together Adapter>];
				$body
			}
			crate::adapter::AdapterKind::Groq => {
				type A = crate::adapter::groq::[<Groq Adapter>];
				$body
			}
			crate::adapter::AdapterKind::Mimo => {
				type A = crate::adapter::mimo::[<Mimo Adapter>];
				$body
			}
			crate::adapter::AdapterKind::Nebius => {
				type A = crate::adapter::nebius::[<Nebius Adapter>];
				$body
			}
			crate::adapter::AdapterKind::Xai => {
				type A = crate::adapter::xai::[<Xai Adapter>];
				$body
			}
			crate::adapter::AdapterKind::DeepSeek => {
				type A = crate::adapter::deepseek::[<DeepSeek Adapter>];
				$body
			}
			crate::adapter::AdapterKind::Zai => {
				type A = crate::adapter::adapters::zai::[<Zai Adapter>];
				$body
			}
			crate::adapter::AdapterKind::BigModel => {
				type A = crate::adapter::bigmodel::[<BigModel Adapter>];
				$body
			}
			crate::adapter::AdapterKind::Aliyun => {
				type A = crate::adapter::aliyun::[<Aliyun Adapter>];
				$body
			}
			crate::adapter::AdapterKind::Cohere => {
				type A = crate::adapter::cohere::[<Cohere Adapter>];
				$body
			}
			crate::adapter::AdapterKind::Ollama => {
				type A = crate::adapter::adapters::ollama::[<Ollama Adapter>];
				$body
			}
			crate::adapter::AdapterKind::OllamaCloud => {
				type A = crate::adapter::adapters::ollama_cloud::[<OllamaCloud Adapter>];
				$body
			}
			crate::adapter::AdapterKind::Vertex => {
				type A = crate::adapter::vertex::[<Vertex Adapter>];
				$body
			}
			crate::adapter::AdapterKind::GithubCopilot => {
				type A = crate::adapter::adapters::github_copilot::[<GithubCopilot Adapter>];
				$body
			}
		}
	};
}
}

pub(crate) use dispatch_adapter;

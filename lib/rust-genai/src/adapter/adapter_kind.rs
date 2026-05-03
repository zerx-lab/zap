use crate::adapter::adapters::github_copilot::GithubCopilotAdapter;
use crate::adapter::adapters::ollama::OllamaAdapter;
use crate::adapter::adapters::ollama_cloud::OllamaCloudAdapter;
use crate::adapter::adapters::openai_resp::OpenAIRespAdapter;
use crate::adapter::adapters::together::TogetherAdapter;
use crate::adapter::adapters::zai::ZaiAdapter;
use crate::adapter::aliyun::AliyunAdapter;
use crate::adapter::anthropic::AnthropicAdapter;
use crate::adapter::bigmodel::BigModelAdapter;
use crate::adapter::cohere::CohereAdapter;
use crate::adapter::deepseek::DeepSeekAdapter;
use crate::adapter::fireworks::FireworksAdapter;
use crate::adapter::gemini::GeminiAdapter;
use crate::adapter::groq::GroqAdapter;
use crate::adapter::mimo::MimoAdapter;
use crate::adapter::nebius::NebiusAdapter;
use crate::adapter::openai::OpenAIAdapter;
use crate::adapter::vertex::VertexAdapter;
use crate::adapter::xai::XaiAdapter;
use crate::adapter::{Adapter as _, zai};
use crate::{ModelName, Result};
use derive_more::Display;
use serde::{Deserialize, Serialize};

/// AdapterKind is an enum that represents the different types of adapters that can be used to interact with the API.
///
#[derive(Debug, Clone, Copy, Display, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum AdapterKind {
	/// For OpenAI Chat Completions and also can be used for OpenAI compatible APIs
	/// NOTE: This adapter share some behavior that other adapters can use while still providing some variant
	OpenAI,
	/// For OpenAI Responses API
	OpenAIResp,
	/// Gemini adapter supports gemini native protocol. e.g., support thinking budget.
	Gemini,
	/// Anthopric native protocol as well
	Anthropic,
	/// For fireworks.ai, mostly OpenAI.
	Fireworks,
	/// Together AI (Mostly uses OpenAI-compatible protocol)
	Together,
	/// Reuse some of the OpenAI adapter behavior, customize some (e.g., normalize thinking budget)
	Groq,
	/// For Mimo (Mostly use OpenAI)
	Mimo,
	/// For Nebius (Mostly use OpenAI)
	Nebius,
	/// For xAI (Mostly use OpenAI)
	Xai,
	/// For DeepSeek (Mostly use OpenAI)
	DeepSeek,
	/// For ZAI (Mostly use OpenAI)
	Zai,
	/// For big model (only accessible via namespace bigmodel::)
	BigModel,
	/// For aliyun (Mostly use OpenAI)
	Aliyun,
	/// Cohere today use it's own native protocol but might move to OpenAI Adapter
	Cohere,
	/// OpenAI shared behavior + some custom. (currently, localhost only, can be customize with ServerTargetResolver).
	Ollama,
	/// For Ollama Cloud (ollama.com) - uses native Ollama protocol with Bearer auth
	OllamaCloud,
	/// Google Vertex AI (Model Garden). Supports Gemini and Claude models via publishers/google and publishers/anthropic.
	/// Uses namespace routing: `vertex::gemini-2.5-flash`, `vertex::claude-sonnet-4-6`
	Vertex,
	/// GitHub Models inference API (multi-publisher gateway for OpenAI, Anthropic, and Google models).
	/// Uses namespace routing: `github_copilot::openai/gpt-4.1-mini`, `github_copilot::anthropic/claude-sonnet-4-6`, `github_copilot::google/gemini-2.5-pro`
	GithubCopilot,
}

/// Serialization/Parse implementations
impl AdapterKind {
	/// Serialize to a static str
	pub fn as_str(&self) -> &'static str {
		match self {
			AdapterKind::OpenAI => "OpenAI",
			AdapterKind::OpenAIResp => "OpenAIResp",
			AdapterKind::Gemini => "Gemini",
			AdapterKind::Anthropic => "Anthropic",
			AdapterKind::Fireworks => "Fireworks",
			AdapterKind::Together => "Together",
			AdapterKind::Groq => "Groq",
			AdapterKind::Mimo => "Mimo",
			AdapterKind::Nebius => "Nebius",
			AdapterKind::Xai => "xAi",
			AdapterKind::DeepSeek => "DeepSeek",
			AdapterKind::Zai => "Zai",
			AdapterKind::BigModel => "BigModel",
			AdapterKind::Aliyun => "Aliyun",
			AdapterKind::Cohere => "Cohere",
			AdapterKind::Ollama => "Ollama",
			AdapterKind::OllamaCloud => "OllamaCloud",
			AdapterKind::Vertex => "Vertex",
			AdapterKind::GithubCopilot => "GithubCopilot",
		}
	}

	/// Serialize to a lowercase static str
	pub fn as_lower_str(&self) -> &'static str {
		match self {
			AdapterKind::OpenAI => "openai",
			AdapterKind::OpenAIResp => "openai_resp",
			AdapterKind::Gemini => "gemini",
			AdapterKind::Anthropic => "anthropic",
			AdapterKind::Fireworks => "fireworks",
			AdapterKind::Together => "together",
			AdapterKind::Groq => "groq",
			AdapterKind::Mimo => "mimo",
			AdapterKind::Nebius => "nebius",
			AdapterKind::Xai => "xai",
			AdapterKind::DeepSeek => "deepseek",
			AdapterKind::Zai => "zai",
			AdapterKind::BigModel => "bigmodel",
			AdapterKind::Aliyun => "aliyun",
			AdapterKind::Cohere => "cohere",
			AdapterKind::Ollama => "ollama",
			AdapterKind::OllamaCloud => "ollama_cloud",
			AdapterKind::Vertex => "vertex",
			AdapterKind::GithubCopilot => "github_copilot",
		}
	}

	pub fn from_lower_str(name: &str) -> Option<Self> {
		match name {
			"openai" => Some(AdapterKind::OpenAI),
			"openai_resp" => Some(AdapterKind::OpenAIResp),
			"gemini" => Some(AdapterKind::Gemini),
			"anthropic" => Some(AdapterKind::Anthropic),
			"fireworks" => Some(AdapterKind::Fireworks),
			"together" => Some(AdapterKind::Together),
			"groq" => Some(AdapterKind::Groq),
			"mimo" => Some(AdapterKind::Mimo),
			"nebius" => Some(AdapterKind::Nebius),
			"xai" => Some(AdapterKind::Xai),
			"deepseek" => Some(AdapterKind::DeepSeek),
			"zai" => Some(AdapterKind::Zai),
			"bigmodel" => Some(AdapterKind::BigModel),
			"aliyun" => Some(AdapterKind::Aliyun),
			"cohere" => Some(AdapterKind::Cohere),
			"ollama" => Some(AdapterKind::Ollama),
			"ollama_cloud" => Some(AdapterKind::OllamaCloud),
			"vertex" => Some(AdapterKind::Vertex),
			"github_copilot" => Some(AdapterKind::GithubCopilot),
			_ => None,
		}
	}
}

/// Utilities
impl AdapterKind {
	/// Get the default key environment variable name for the adapter kind.
	pub fn default_key_env_name(&self) -> Option<&'static str> {
		match self {
			AdapterKind::OpenAI => OpenAIAdapter::DEFAULT_API_KEY_ENV_NAME,
			AdapterKind::OpenAIResp => OpenAIRespAdapter::DEFAULT_API_KEY_ENV_NAME,
			AdapterKind::Gemini => GeminiAdapter::DEFAULT_API_KEY_ENV_NAME,
			AdapterKind::Anthropic => AnthropicAdapter::DEFAULT_API_KEY_ENV_NAME,
			AdapterKind::Fireworks => FireworksAdapter::DEFAULT_API_KEY_ENV_NAME,
			AdapterKind::Together => TogetherAdapter::DEFAULT_API_KEY_ENV_NAME,
			AdapterKind::Groq => GroqAdapter::DEFAULT_API_KEY_ENV_NAME,
			AdapterKind::Mimo => MimoAdapter::DEFAULT_API_KEY_ENV_NAME,
			AdapterKind::Nebius => NebiusAdapter::DEFAULT_API_KEY_ENV_NAME,
			AdapterKind::Xai => XaiAdapter::DEFAULT_API_KEY_ENV_NAME,
			AdapterKind::DeepSeek => DeepSeekAdapter::DEFAULT_API_KEY_ENV_NAME,
			AdapterKind::Zai => ZaiAdapter::DEFAULT_API_KEY_ENV_NAME,
			AdapterKind::BigModel => BigModelAdapter::DEFAULT_API_KEY_ENV_NAME,
			AdapterKind::Aliyun => AliyunAdapter::DEFAULT_API_KEY_ENV_NAME,
			AdapterKind::Cohere => CohereAdapter::DEFAULT_API_KEY_ENV_NAME,
			AdapterKind::Ollama => OllamaAdapter::DEFAULT_API_KEY_ENV_NAME,
			AdapterKind::OllamaCloud => OllamaCloudAdapter::DEFAULT_API_KEY_ENV_NAME,
			AdapterKind::Vertex => VertexAdapter::DEFAULT_API_KEY_ENV_NAME,
			AdapterKind::GithubCopilot => GithubCopilotAdapter::DEFAULT_API_KEY_ENV_NAME,
		}
	}
}

/// From Model implementations
impl AdapterKind {
	/// This is a default static mapping from model names to AdapterKind.
	///
	/// When more control is needed, the `ServiceTypeResolver` can be used
	/// to map a model name to any adapter and endpoint.
	///
	///  - OpenAI     - starts_with "gpt", "o3", "o1", "chatgpt"
	///  - Gemini     - starts_with "gemini"
	///  - Anthropic  - starts_with "claude"
	///  - Fireworks  - contains "fireworks" (might add leading or trailing '/' later)
	///  - Groq       - model in Groq models
	///  - DeepSeek   - model in DeepSeek models (deepseek.com)
	///  - Zhipu      - starts_with "glm"
	///  - Cohere     - starts_with "command"
	///  - Ollama     - For anything else
	///
	/// Other Some adapters have to have model name namespaced to be used,
	/// - e.g., for together.ai `together::meta-llama/Llama-3-8b-chat-hf`
	/// - e.g., for nebius with `nebius::Qwen/Qwen3-235B-A22B`
	/// - e.g., for ZAI coding plan with `coding::glm-4.6`
	/// - e.g., for vertex with `vertex::gemini-2.5-flash` or `vertex::claude-sonnet-4-6`
	///
	/// And all adapters can be force namspaced as well.
	///
	/// Note: At this point, this will never fail as the fallback is the Ollama adapter.
	///       This might change in the future, hence the Result return type.
	///
	/// IMPORTANT: Since v0.6.0, Groq and Deepseek models needs to be namespaced e.g., `groq::_model_name_`
	//             (because now, list_names are dynamic, so, automatic mapping can only be done base on clear model "prefixes")
	pub fn from_model(model: &str) -> Result<Self> {
		// -- First check if namespaced
		if let Some(adapter) = Self::from_model_namespace(model) {
			return Ok(adapter);
		};

		// -- Otherwise, Resolve from modelname
		if model.starts_with("o3")
			|| model.starts_with("o4")
			|| model.starts_with("o1")
			|| model.starts_with("chatgpt")
			|| model.starts_with("codex")
			|| (model.starts_with("gpt") && !model.starts_with("gpt-oss"))
			|| model.starts_with("text-embedding")
		// migh be a little generic on this one
		{
			if model.starts_with("gpt-5")
				|| (model.starts_with("gpt") && (model.contains("codex") || model.contains("pro")))
			{
				Ok(Self::OpenAIResp)
			} else {
				Ok(Self::OpenAI)
			}
		} else if model.starts_with("gemini") {
			Ok(Self::Gemini)
		} else if model.starts_with("claude") {
			Ok(Self::Anthropic)
		} else if model.contains("fireworks") {
			Ok(Self::Fireworks)
		} else if model.starts_with("mimo-") {
			Ok(Self::Mimo)
		} else if model.starts_with("command") || model.starts_with("embed-") {
			Ok(Self::Cohere)
		} else if model.starts_with("grok") {
			Ok(Self::Xai)
		} else if model.starts_with("glm") {
			Ok(Self::Zai)
		} else if model.starts_with("deepseek-") {
			Ok(Self::DeepSeek)
		}
		// For now, fallback to Ollama
		else {
			Ok(Self::Ollama)
		}
	}
}

// region:    --- Support

/// Inner api to return
impl AdapterKind {
	fn from_model_namespace(model: &str) -> Option<Self> {
		let (namespace, _) = ModelName::split_as_namespace_and_name(model);
		let namespace = namespace?;

		// -- First, check if simple adapter lower string match
		if let Some(adapter) = Self::from_lower_str(namespace) {
			Some(adapter)
		}
		// -- Second, custom, for now, we harcode this exceptin here (might become more generic later)
		else if namespace == zai::ZAI_CODING_NAMESPACE {
			Some(Self::Zai)
		}
		//
		// -- Otherwise, no adapter from namespace, because no matching namespace
		else {
			None
		}
	}
}

// endregion: --- Support

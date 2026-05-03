use crate::chat::{ContentPart, MessageContent, ToolCall, ToolResponse};
use derive_more::From;
use serde::{Deserialize, Serialize};

/// A single chat message (system, user, assistant, or tool).
///
/// Design:
/// - Uses one struct with a role field instead of role-specific enum variants.
/// - Payload lives in MessageContent; ChatRole distinguishes the role.
/// - MessageContent is a multipart format, with `Vec<ContentPart>`
///
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
	/// The message role.
	pub role: ChatRole,

	/// Message content.
	pub content: MessageContent,

	/// Optional per-message options (e.g., cache control).
	pub options: Option<MessageOptions>,
}

// region:    --- Constructors
impl ChatMessage {
	/// Constructs a message for the provided role.
	pub fn new(role: ChatRole, content: impl Into<MessageContent>) -> Self {
		Self {
			role,
			content: content.into(),
			options: None,
		}
	}

	/// Constructs a system message.
	pub fn system(content: impl Into<MessageContent>) -> Self {
		Self {
			role: ChatRole::System,
			content: content.into(),
			options: None,
		}
	}

	/// Constructs an assistant message.
	pub fn assistant(content: impl Into<MessageContent>) -> Self {
		Self {
			role: ChatRole::Assistant,
			content: content.into(),
			options: None,
		}
	}

	/// Constructs a user message.
	pub fn user(content: impl Into<MessageContent>) -> Self {
		Self {
			role: ChatRole::User,
			content: content.into(),
			options: None,
		}
	}

	/// Constructs a tool message.
	pub fn tool(content: impl Into<MessageContent>) -> Self {
		Self {
			role: ChatRole::Tool,
			content: content.into(),
			options: None,
		}
	}
}
// endregion: --- Constructors

// region:    --- Accessors
impl ChatMessage {
	/// Returns an approximate in-memory size of this `ChatMessage`, in bytes,
	/// computed as the size of the content plus.
	pub fn size(&self) -> usize {
		// Note: Do not include the role len
		self.content.size()
	}
}
// endregion: --- Accessors

// region:    --- Builders
impl ChatMessage {
	/// Attaches options to this message.
	pub fn with_options(mut self, options: impl Into<MessageOptions>) -> Self {
		self.options = Some(options.into());
		self
	}

	/// Attach reasoning content to this message as a `ContentPart::ReasoningContent` part.
	/// This is used for round-tripping assistant reasoning (e.g., DeepSeek, Kimi).
	pub fn with_reasoning_content(mut self, reasoning: Option<String>) -> Self {
		if let Some(reasoning) = reasoning {
			self.content.push(ContentPart::ReasoningContent(reasoning));
		}
		self
	}

	/// Convenience: build an assistant message that contains an optional list
	/// of thought signatures followed by tool calls. Useful for providers
	/// (e.g., Gemini 3) that require the thought signature to appear before
	/// tool calls in the assistant turn when continuing a tool-use exchange.
	pub fn assistant_tool_calls_with_thoughts(tool_calls: Vec<ToolCall>, thought_signatures: Vec<String>) -> Self {
		let mut parts: Vec<ContentPart> = thought_signatures.into_iter().map(ContentPart::ThoughtSignature).collect();
		parts.extend(tool_calls.into_iter().map(ContentPart::ToolCall));
		ChatMessage::assistant(MessageContent::from_parts(parts))
	}
}
// endregion: --- Builders

// region:    --- MessageOptions

#[derive(Debug, Clone, Default, Serialize, Deserialize, From)]
/// Per-message options (e.g., cache control).
pub struct MessageOptions {
	#[from]
	/// Per-provider cache behavior hint.
	pub cache_control: Option<CacheControl>,
}

impl MessageOptions {
	pub fn with_cache_control(mut self, cache_control: impl Into<CacheControl>) -> Self {
		self.cache_control = Some(cache_control.into());
		self
	}
}

/// Cache control for prompt caching.
///
/// Notes:
/// - This is the unified cache policy abstraction used at both chat-request and message level.
/// - Anthropic applies cache_control at the content-part level; genai exposes it at the
///   ChatMessage level and maps it appropriately.
/// - OpenAI currently uses request-level mappings for a subset of variants and ignores
///   unsupported message-level cache control.
/// - Different providers support different variants and scopes.
///
/// ## TTL Ordering Constraint (Anthropic)
///
/// When mixing different TTLs in the same request, cache entries with longer TTL
/// must appear **before** shorter TTLs. That is, `Ephemeral1h` entries must appear
/// before any `Ephemeral`, `Memory`, or `Ephemeral5m` entries in the message sequence.
///
/// Violating this constraint may cause the API to reject the request or behave unexpectedly.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CacheControl {
	/// Default ephemeral cache (5 minutes TTL).
	Ephemeral,
	/// Memory cache.
	///
	/// Note: On providers without a distinct ephemeral default, this may map to the
	/// provider's memory-oriented request cache mode.
	Memory,
	/// Explicit 5-minute TTL cache.
	Ephemeral5m,
	/// Extended 1-hour TTL cache.
	///
	/// **Important:** When mixing TTLs, 1-hour cache entries must appear before
	/// any 5-minute cache entries in the request.
	///
	/// Note: Costs 2x base input token price vs 1.25x for 5m.
	Ephemeral1h,
	/// Extended 24-hour TTL cache.
	Ephemeral24h,
}

impl From<CacheControl> for MessageOptions {
	fn from(cache_control: CacheControl) -> Self {
		Self {
			cache_control: Some(cache_control),
		}
	}
}
// endregion: --- MessageOptions

// region:    --- ChatRole

/// Chat roles recognized across providers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, derive_more::Display)]
#[allow(missing_docs)]
pub enum ChatRole {
	System,
	User,
	Assistant,
	Tool,
}

// endregion: --- ChatRole

// region:    --- Froms

/// Will create a Assisttant ChatMessage with this vect of tool
impl From<Vec<ToolCall>> for ChatMessage {
	fn from(tool_calls: Vec<ToolCall>) -> Self {
		if let Some(first) = tool_calls.first()
			&& let Some(thoughts) = &first.thought_signatures
		{
			let mut parts: Vec<ContentPart> = thoughts.iter().cloned().map(ContentPart::ThoughtSignature).collect();
			parts.extend(tool_calls.into_iter().map(ContentPart::ToolCall));
			return ChatMessage::assistant(MessageContent::from_parts(parts));
		}
		Self {
			role: ChatRole::Assistant,
			content: MessageContent::from(tool_calls),
			options: None,
		}
	}
}

impl From<ToolResponse> for ChatMessage {
	fn from(value: ToolResponse) -> Self {
		ChatMessage::tool(value)
	}
}

impl From<Vec<ToolResponse>> for ChatMessage {
	fn from(responses: Vec<ToolResponse>) -> Self {
		ChatMessage::tool(MessageContent::from(responses))
	}
}

// endregion: --- Froms

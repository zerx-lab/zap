//! Internal stream event types that serve as intermediaries between the provider event and the GenAI stream event.
//!
//! This allows for flexibility if we want to capture events across providers that do not need to
//! be reflected in the public ChatStream event.
//!
//! NOTE: This might be removed at some point as it may not be needed, and we could go directly to the GenAI stream.

use crate::chat::{StopReason, Usage};

#[derive(Debug, Default)]
pub struct InterStreamEnd {
	// When `ChatOptions..capture_usage == true`
	pub captured_usage: Option<Usage>,

	// Normalised stop reason.
	pub captured_stop_reason: Option<StopReason>,

	// When `ChatOptions..capture_content == true`
	pub captured_text_content: Option<String>,

	// When `ChatOptions..capture_reasoning_content == true`
	pub captured_reasoning_content: Option<String>,

	// When `ChatOptions..capture_tool_calls == true`
	pub captured_tool_calls: Option<Vec<crate::chat::ToolCall>>,

	// When `ChatOptions..capture_thought_signatures == true` (implied or explicit)
	pub captured_thought_signatures: Option<Vec<String>>,

	// Response ID for stateful sessions (OpenAI Responses API).
	pub captured_response_id: Option<String>,
}

/// Intermediary StreamEvent
#[derive(Debug)]
pub enum InterStreamEvent {
	Start,
	Chunk(String),
	ReasoningChunk(String),
	ThoughtSignatureChunk(String),
	ToolCallChunk(crate::chat::ToolCall),
	End(InterStreamEnd),
}

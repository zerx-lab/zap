use crate::adapter::inter_stream::{InterStreamEnd, InterStreamEvent};
use crate::chat::{ChatMessage, ContentPart, MessageContent, StopReason, ToolCall, Usage};
use futures::Stream;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::task::{Context, Poll};

type InterStreamType = Pin<Box<dyn Stream<Item = crate::Result<InterStreamEvent>> + Send>>;

/// A stream of chat events produced by a streaming chat request.
pub struct ChatStream {
	inter_stream: InterStreamType,
}

impl ChatStream {
	pub(crate) fn new(inter_stream: InterStreamType) -> Self {
		ChatStream { inter_stream }
	}

	pub(crate) fn from_inter_stream<T>(inter_stream: T) -> Self
	where
		T: Stream<Item = crate::Result<InterStreamEvent>> + Send + 'static,
	{
		let boxed_stream: InterStreamType = Box::pin(inter_stream);
		ChatStream::new(boxed_stream)
	}
}

// region:    --- Stream Impl

impl Stream for ChatStream {
	type Item = crate::Result<ChatStreamEvent>;

	fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		let this = self.get_mut();

		match Pin::new(&mut this.inter_stream).poll_next(cx) {
			Poll::Ready(Some(Ok(event))) => {
				let chat_event = match event {
					InterStreamEvent::Start => ChatStreamEvent::Start,
					InterStreamEvent::Chunk(content) => ChatStreamEvent::Chunk(StreamChunk { content }),
					InterStreamEvent::ReasoningChunk(content) => {
						ChatStreamEvent::ReasoningChunk(StreamChunk { content })
					}
					InterStreamEvent::ThoughtSignatureChunk(content) => {
						ChatStreamEvent::ThoughtSignatureChunk(StreamChunk { content })
					}
					InterStreamEvent::ToolCallChunk(tool_call) => {
						ChatStreamEvent::ToolCallChunk(ToolChunk { tool_call })
					}
					InterStreamEvent::End(inter_end) => ChatStreamEvent::End(inter_end.into()),
				};
				Poll::Ready(Some(Ok(chat_event)))
			}
			Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(e))),
			Poll::Ready(None) => Poll::Ready(None),
			Poll::Pending => Poll::Pending,
		}
	}
}

// endregion: --- Stream Impl

// region:    --- ChatStreamEvent

/// Provider-agnostic chat events returned by `Client::exec()` when streaming.
#[derive(Debug, Serialize, Deserialize)]
pub enum ChatStreamEvent {
	/// Emitted once at the start of the stream.
	Start,

	/// Assistant content chunk (text).
	Chunk(StreamChunk),

	/// Reasoning content chunk.
	ReasoningChunk(StreamChunk),

	/// Thought signature content chunk.
	ThoughtSignatureChunk(StreamChunk),

	/// Tool-call chunk.
	ToolCallChunk(ToolChunk),

	/// End of stream.
	/// May include captured usage and/or content when enabled via `ChatOptions`.
	End(StreamEnd),
}

/// Content of `ChatStreamEvent::Chunk`.
/// Currently text only.
#[derive(Debug, Serialize, Deserialize)]
pub struct StreamChunk {
	/// Text content.
	pub content: String,
}

/// Content of `ChatStreamEvent::ToolCallChunk`.
#[derive(Debug, Serialize, Deserialize)]
pub struct ToolChunk {
	/// The tool call.
	pub tool_call: ToolCall,
}

/// Terminal event data with optionally captured usage and content.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct StreamEnd {
	/// Captured usage if `ChatOptions.capture_usage` is enabled.
	pub captured_usage: Option<Usage>,

	/// Normalised stop reason captured at stream end (see [`StopReason`]).
	pub captured_stop_reason: Option<StopReason>,

	/// Captured final content (text and tool calls) if `ChatOptions.capture_content`
	/// or `capture_tool_calls` is enabled.
	/// Note: Since 0.4.0 this includes tool calls as well (for API symmetry with `ChatResponse`);
	///       use `.captured_tool_calls()` or `.captured_texts()`.
	pub captured_content: Option<MessageContent>,

	/// Captured reasoning content if `ChatOptions.capture_reasoning` is enabled.
	pub captured_reasoning_content: Option<String>,

	/// Response ID for stateful sessions (OpenAI Responses API).
	pub captured_response_id: Option<String>,
}

impl From<InterStreamEnd> for StreamEnd {
	fn from(inter_end: InterStreamEnd) -> Self {
		let captured_text_content = inter_end.captured_text_content;
		let mut captured_tool_calls = inter_end.captured_tool_calls;

		// -- create public captured_content
		// Ordering policy: ThoughtSignature -> Text -> ToolCall
		// This matches provider expectations (e.g., Gemini 3 requires thought first).
		let mut captured_content: Option<MessageContent> = None;
		if let Some(captured_thoughts) = inter_end.captured_thought_signatures {
			let thoughts_content = captured_thoughts
				.into_iter()
				.map(ContentPart::ThoughtSignature)
				.collect::<Vec<_>>();
			// Also attach thoughts to the first tool call so that
			// ChatMessage::from(Vec<ToolCall>) can auto-prepend them.
			if let Some(tool_calls) = captured_tool_calls.as_mut()
				&& let Some(first_call) = tool_calls.first_mut()
			{
				first_call.thought_signatures = Some(
					thoughts_content
						.iter()
						.filter_map(|p| p.as_thought_signature().map(|s| s.to_string()))
						.collect(),
				);
			}
			if let Some(existing_content) = &mut captured_content {
				existing_content.extend_front(thoughts_content);
			} else {
				captured_content = Some(MessageContent::from_parts(thoughts_content));
			}
		}
		if let Some(captured_text_content) = captured_text_content {
			// This `captured_text_content` is the concatenation of all text chunks received.
			if let Some(existing_content) = &mut captured_content {
				existing_content.extend(MessageContent::from_text(captured_text_content));
			} else {
				captured_content = Some(MessageContent::from_text(captured_text_content));
			}
		}
		if let Some(captured_tool_calls) = captured_tool_calls {
			if let Some(existing_content) = &mut captured_content {
				existing_content.extend(MessageContent::from_tool_calls(captured_tool_calls));
			} else {
				// This `captured_tool_calls` is the concatenation of all tool call chunks received.
				captured_content = Some(MessageContent::from_tool_calls(captured_tool_calls));
			}
		}

		// -- Return result
		StreamEnd {
			captured_usage: inter_end.captured_usage,
			captured_stop_reason: inter_end.captured_stop_reason,
			captured_content,
			captured_reasoning_content: inter_end.captured_reasoning_content,
			captured_response_id: inter_end.captured_response_id,
		}
	}
}

/// Getters
impl StreamEnd {
	/// Returns the first captured text, if any.
	/// This is the concatenation of all streamed text chunks.
	pub fn captured_first_text(&self) -> Option<&str> {
		let captured_content = self.captured_content.as_ref()?;
		captured_content.first_text()
	}

	/// Consumes `self` and returns the first captured text, if any.
	/// This is the concatenation of all streamed text chunks.
	pub fn captured_into_first_text(self) -> Option<String> {
		let captured_content = self.captured_content?;
		captured_content.into_first_text()
	}

	/// Returns all captured text segments, if any.
	pub fn captured_texts(&self) -> Option<Vec<&str>> {
		let captured_content = self.captured_content.as_ref()?;
		Some(captured_content.texts())
	}

	/// Consumes `self` and returns all captured text segments, if any.
	pub fn into_texts(self) -> Option<Vec<String>> {
		let captured_content = self.captured_content?;
		Some(captured_content.into_texts())
	}

	/// Returns all captured tool calls, if any.
	pub fn captured_tool_calls(&self) -> Option<Vec<&ToolCall>> {
		let captured_content = self.captured_content.as_ref()?;
		Some(captured_content.tool_calls())
	}

	/// Consumes `self` and returns all captured tool calls, if any.
	pub fn captured_into_tool_calls(self) -> Option<Vec<ToolCall>> {
		let captured_content = self.captured_content?;
		Some(captured_content.into_tool_calls())
	}

	/// Returns all captured thought signatures, if any.
	pub fn captured_thought_signatures(&self) -> Option<Vec<&str>> {
		let captured_content = self.captured_content.as_ref()?;
		Some(
			captured_content
				.parts()
				.iter()
				.filter_map(|p| p.as_thought_signature())
				.collect(),
		)
	}

	/// Consumes `self` and returns all captured thought signatures, if any.
	pub fn captured_into_thought_signatures(self) -> Option<Vec<String>> {
		let captured_content = self.captured_content?;
		Some(
			captured_content
				.into_parts()
				.into_iter()
				.filter_map(|p| p.into_thought_signature())
				.collect(),
		)
	}

	/// Convenience: build an assistant message for a tool-use handoff that places
	/// thought signatures (if any) before tool calls. Returns None if no tool calls
	/// were captured.
	pub fn into_assistant_message_for_tool_use(self) -> Option<ChatMessage> {
		let content = self.captured_content?;
		let mut thought_signatures: Vec<String> = Vec::new();
		let mut tool_calls: Vec<ToolCall> = Vec::new();
		for part in content.into_parts() {
			match part {
				ContentPart::ThoughtSignature(t) => thought_signatures.push(t),
				ContentPart::ToolCall(tc) => tool_calls.push(tc),
				_ => {}
			}
		}
		if tool_calls.is_empty() {
			return None;
		}
		Some(
			ChatMessage::assistant_tool_calls_with_thoughts(tool_calls, thought_signatures)
				.with_reasoning_content(self.captured_reasoning_content),
		)
	}
}

// endregion: --- ChatStreamEvent

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_stream_end_preserves_captured_stop_reason() {
		let inter_end = InterStreamEnd {
			captured_stop_reason: Some(StopReason::Completed("stop".to_string())),
			..Default::default()
		};
		let stream_end = StreamEnd::from(inter_end);
		assert_eq!(
			stream_end.captured_stop_reason,
			Some(StopReason::Completed("stop".to_string()))
		);
	}
}

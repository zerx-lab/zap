use crate::adapter::adapters::support::{StreamerCapturedData, StreamerOptions};
use crate::adapter::gemini::{GeminiAdapter, GeminiChatResponse};
use crate::adapter::inter_stream::{InterStreamEnd, InterStreamEvent};
use crate::chat::{ChatOptionsSet, StopReason, ToolCall};
use crate::webc::WebStream;
use crate::{Error, ModelIden, Result};
use serde_json::Value;
use std::pin::Pin;
use std::task::{Context, Poll};

use super::GeminiChatContent;

use std::collections::VecDeque;

pub struct GeminiStreamer {
	inner: WebStream,
	options: StreamerOptions,

	// -- Set by the poll_next
	/// Flag to not poll the EventSource after a MessageStop event.
	done: bool,
	captured_data: StreamerCapturedData,
	pending_events: VecDeque<InterStreamEvent>,
}

impl GeminiStreamer {
	pub fn new(inner: WebStream, model_iden: ModelIden, options_set: ChatOptionsSet<'_, '_>) -> Self {
		Self {
			inner,
			done: false,
			options: StreamerOptions::new(model_iden, options_set),
			captured_data: Default::default(),
			pending_events: VecDeque::new(),
		}
	}
}

// Implement futures::Stream for InterStream<GeminiStream>
impl futures::Stream for GeminiStreamer {
	type Item = Result<InterStreamEvent>;

	fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		if self.done {
			return Poll::Ready(None);
		}

		// 1. Check if we have pending events
		if let Some(event) = self.pending_events.pop_front() {
			return Poll::Ready(Some(Ok(event)));
		}

		while let Poll::Ready(item) = Pin::new(&mut self.inner).poll_next(cx) {
			match item {
				Some(Ok(raw_message)) => {
					// This is the message sent by the WebStream in PrettyJsonArray mode.
					// - `[` document start
					// - `{...}` block
					// - `]` document end
					match raw_message.as_str() {
						"[" => return Poll::Ready(Some(Ok(InterStreamEvent::Start))),
						"]" => {
							let inter_stream_end = InterStreamEnd {
								captured_usage: self.captured_data.usage.take(),
								captured_stop_reason: self.captured_data.stop_reason.take().map(StopReason::from),
								captured_text_content: self.captured_data.content.take(),
								captured_reasoning_content: self.captured_data.reasoning_content.take(),
								captured_tool_calls: self.captured_data.tool_calls.take(),
								captured_thought_signatures: self.captured_data.thought_signatures.take(),
								captured_response_id: None,
							};

							return Poll::Ready(Some(Ok(InterStreamEvent::End(inter_stream_end))));
						}
						block_string => {
							// -- Parse the block to JSON
							let json_block = match serde_json::from_str::<Value>(block_string).map_err(|serde_error| {
								Error::StreamParse {
									model_iden: self.options.model_iden.clone(),
									serde_error,
								}
							}) {
								Ok(json_block) => json_block,
								Err(err) => {
									tracing::error!("Gemini Adapter Stream Error: {}", err);
									return Poll::Ready(Some(Err(err)));
								}
							};

							// -- Extract the Gemini Response
							let gemini_response =
								match GeminiAdapter::body_to_gemini_chat_response(&self.options.model_iden, json_block)
								{
									Ok(gemini_response) => gemini_response,
									Err(err) => {
										tracing::error!("Gemini Adapter Stream Error: {}", err);
										return Poll::Ready(Some(Err(err)));
									}
								};

							let GeminiChatResponse {
								content,
								usage,
								stop_reason,
							} = gemini_response;

							// -- Capture stop_reason if present (typically in the last chunk)
							if stop_reason.is_some() {
								self.captured_data.stop_reason = stop_reason;
							}

							// -- Extract text, reasoning, and tool calls from content parts.
							// Gemini can return multiple functionCall parts in a single
							// streaming chunk (parallel tool calls), so we collect all of
							// them instead of keeping only the last one.
							let mut stream_text_content: String = String::new();
							let mut stream_reasoning_content: Option<String> = None;
							let mut stream_tool_calls: Vec<ToolCall> = Vec::new();
							let mut stream_thought: Option<String> = None;

							for g_content_item in content {
								match g_content_item {
									GeminiChatContent::Reasoning(reasoning) => {
										stream_reasoning_content = Some(reasoning)
									}
									GeminiChatContent::Text(text) => stream_text_content.push_str(&text),
									GeminiChatContent::Binary(_) => {
										// For now, we do not stream binary content.
									}
									GeminiChatContent::ToolCall(tool_call) => stream_tool_calls.push(tool_call),
									GeminiChatContent::ThoughtSignature(thought) => stream_thought = Some(thought),
								}
							}

							// -- Queue Events
							// Priority: Thought -> Text -> ToolCall

							// 1. Thought
							if let Some(thought) = stream_thought {
								// Capture thought
								match self.captured_data.thought_signatures {
									Some(ref mut thoughts) => thoughts.push(thought.clone()),
									None => self.captured_data.thought_signatures = Some(vec![thought.clone()]),
								}

								if self.options.capture_usage {
									self.captured_data.usage = Some(usage.clone());
								}

								self.pending_events.push_back(InterStreamEvent::ThoughtSignatureChunk(thought));
							}
							if let Some(reasoning_content) = stream_reasoning_content {
								// Capture reasoning content
								if self.options.capture_content {
									match self.captured_data.reasoning_content {
										Some(ref mut rc) => rc.push_str(&reasoning_content),
										None => self.captured_data.reasoning_content = Some(reasoning_content.clone()),
									}
								}
								if self.options.capture_usage {
									self.captured_data.usage = Some(usage.clone());
								}
								self.pending_events
									.push_back(InterStreamEvent::ReasoningChunk(reasoning_content));
							}

							// 2. Text
							if !stream_text_content.is_empty() {
								// Capture content
								if self.options.capture_content {
									match self.captured_data.content {
										Some(ref mut c) => c.push_str(&stream_text_content),
										None => self.captured_data.content = Some(stream_text_content.clone()),
									}
								}

								if self.options.capture_usage {
									self.captured_data.usage = Some(usage.clone());
								}

								self.pending_events.push_back(InterStreamEvent::Chunk(stream_text_content));
							}

							// 3. Tool Calls (supports parallel tool calls from Gemini)
							for tool_call in stream_tool_calls {
								if self.options.capture_tool_calls {
									match self.captured_data.tool_calls {
										Some(ref mut tool_calls) => tool_calls.push(tool_call.clone()),
										None => self.captured_data.tool_calls = Some(vec![tool_call.clone()]),
									}
								}
								if self.options.capture_usage {
									self.captured_data.usage = Some(usage.clone());
								}
								self.pending_events.push_back(InterStreamEvent::ToolCallChunk(tool_call));
							}

							// Return the first event if any
							if let Some(event) = self.pending_events.pop_front() {
								return Poll::Ready(Some(Ok(event)));
							}
						}
					};
				}
				Some(Err(err)) => {
					tracing::error!("Gemini Adapter Stream Error: {}", err);
					return Poll::Ready(Some(Err(Error::WebStream {
						model_iden: self.options.model_iden.clone(),
						cause: err.to_string(),
						error: err,
					})));
				}
				None => {
					self.done = true;
					return Poll::Ready(None);
				}
			}
		}
		Poll::Pending
	}
}

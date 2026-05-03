use crate::adapter::AdapterKind;
use crate::adapter::adapters::support::{StreamerCapturedData, StreamerOptions};
use crate::adapter::inter_stream::{InterStreamEnd, InterStreamEvent};
use crate::adapter::openai::OpenAIAdapter;
use crate::chat::{ChatOptionsSet, StopReason, ToolCall};
use crate::webc::{Event, EventSourceStream};
use crate::{Error, ModelIden, Result};
use serde_json::Value;
use std::pin::Pin;
use std::task::{Context, Poll};
use value_ext::JsonValueExt;

fn take_stream_error(message_data: &mut Value, model_iden: &ModelIden) -> Option<Error> {
	let error_body = message_data.x_take::<Value>("error").ok()?;
	Some(Error::ChatResponse {
		model_iden: model_iden.clone(),
		body: error_body,
	})
}

fn take_finish_reason_usage(
	message_data: &mut Value,
	adapter_kind: AdapterKind,
	capture_usage: bool,
) -> Option<crate::chat::Usage> {
	if !capture_usage {
		return None;
	}

	match adapter_kind {
		AdapterKind::Groq => Some(
			message_data
				.x_take("/x_groq/usage")
				.map(|v| OpenAIAdapter::into_usage(adapter_kind, v))
				.unwrap_or_default(),
		),
		AdapterKind::DeepSeek | AdapterKind::Zai | AdapterKind::Fireworks | AdapterKind::Together => Some(
			message_data
				.x_take("usage")
				.map(|v| OpenAIAdapter::into_usage(adapter_kind, v))
				.unwrap_or_default(),
		),
		_ => message_data
			.x_take("usage")
			.ok()
			.map(|v| OpenAIAdapter::into_usage(adapter_kind, v)),
	}
}

pub struct OpenAIStreamer {
	inner: EventSourceStream,
	options: StreamerOptions,

	// -- Set by the poll_next
	/// Flag to prevent polling the EventSource after a MessageStop event
	done: bool,
	captured_data: StreamerCapturedData,
}

impl OpenAIStreamer {
	pub fn new(inner: EventSourceStream, model_iden: ModelIden, options_set: ChatOptionsSet<'_, '_>) -> Self {
		Self {
			inner,
			done: false,
			options: StreamerOptions::new(model_iden, options_set),
			captured_data: Default::default(),
		}
	}

	/// Captures a single tool call into `captured_data.tool_calls`, merging with existing if needed.
	/// Returns the (possibly merged) tool call for use in events.
	fn capture_tool_call(&mut self, index: usize, call_id: String, fn_name: String, arguments: String) -> ToolCall {
		let tool_call = ToolCall {
			call_id: call_id.clone(),
			fn_name: fn_name.clone(),
			fn_arguments: Value::String(arguments.clone()),
			thought_signatures: None,
		};

		if !self.options.capture_tool_calls {
			return tool_call;
		}

		let calls = self.captured_data.tool_calls.get_or_insert_with(Vec::new);

		if let Some(existing_call) = calls.get_mut(index) {
			// Merge with existing: accumulate arguments as strings
			if let Some(existing_args) = existing_call.fn_arguments.as_str() {
				let accumulated = format!("{existing_args}{arguments}");
				existing_call.fn_arguments = Value::String(accumulated);
			}
			// Update call_id and fn_name on first chunk that has them
			if !fn_name.is_empty() {
				existing_call.call_id = call_id;
				existing_call.fn_name = fn_name;
			}
			existing_call.clone()
		} else {
			// New tool call - resize to handle potential gaps (though unlikely in streaming)
			calls.resize(index + 1, tool_call.clone());
			tool_call
		}
	}
}

impl futures::Stream for OpenAIStreamer {
	type Item = Result<InterStreamEvent>;

	fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		if self.done {
			// The last poll was definitely the end, so end the stream.
			// This will prevent triggering a stream ended error
			return Poll::Ready(None);
		}
		while let Poll::Ready(event) = Pin::new(&mut self.inner).poll_next(cx) {
			match event {
				Some(Ok(Event::Open)) => return Poll::Ready(Some(Ok(InterStreamEvent::Start))),
				Some(Ok(Event::Message(message))) => {
					// -- End Message
					// According to OpenAI Spec, this is the end message
					if message.data == "[DONE]" {
						self.done = true;

						// -- Build the usage and captured_content
						// TODO: Needs to clarify wh for usage we do not adopt the same strategy from captured content below
						let captured_usage = if self.options.capture_usage {
							self.captured_data.usage.take()
						} else {
							None
						};

						// -- Process the captured_tool_calls
						// NOTE: here we attempt to parse the `fn_arguments` if it is string, because it means that it was accumulated
						let captured_tool_calls = if let Some(tools_calls) = self.captured_data.tool_calls.take() {
							let tools_calls: Vec<ToolCall> = tools_calls
								.into_iter()
								.map(|tool_call| {
									// extrat
									let ToolCall {
										call_id,
										fn_name,
										fn_arguments,
										..
									} = tool_call;
									// parse fn_arguments if needed
									let fn_arguments = match fn_arguments {
										Value::String(fn_arguments_string) => {
											// NOTE: Here we are resilient for now, if we cannot parse, just return the original String
											match serde_json::from_str::<Value>(&fn_arguments_string) {
												Ok(fn_arguments) => fn_arguments,
												Err(_) => Value::String(fn_arguments_string),
											}
										}
										_ => fn_arguments,
									};

									ToolCall {
										call_id,
										fn_name,
										fn_arguments,
										thought_signatures: None,
									}
								})
								.collect();
							Some(tools_calls)
						} else {
							None
						};

						// Return the internal stream end
						let inter_stream_end = InterStreamEnd {
							captured_usage,
							captured_stop_reason: self.captured_data.stop_reason.take().map(StopReason::from),
							captured_text_content: self.captured_data.content.take(),
							captured_reasoning_content: self.captured_data.reasoning_content.take(),
							captured_tool_calls,
							captured_thought_signatures: None,
							captured_response_id: None,
						};

						return Poll::Ready(Some(Ok(InterStreamEvent::End(inter_stream_end))));
					}

					// -- Other Content Messages
					// Parse to get the choice
					let mut message_data: Value =
						serde_json::from_str(&message.data).map_err(|serde_error| Error::StreamParse {
							model_iden: self.options.model_iden.clone(),
							serde_error,
						})?;

					if let Some(error) = take_stream_error(&mut message_data, &self.options.model_iden) {
						return Poll::Ready(Some(Err(error)));
					}

					let first_choice: Option<Value> = message_data.x_take("/choices/0").ok();

					let adapter_kind = self.options.model_iden.adapter_kind;

					// If we have a first choice, then it's a normal message
					if let Some(mut first_choice) = first_choice {
						// -- Finish Reason
						// If finish_reason exists, it's the end of this choice.
						// Since we support only a single choice, we can proceed,
						// as there might be other messages, and the last one contains data: `[DONE]`
						// NOTE: xAI has no `finish_reason` when not finished, so, need to just account for both null/absent
						if let Ok(Some(finish_reason)) = first_choice.x_take::<Option<String>>("finish_reason") {
							self.captured_data.stop_reason = Some(finish_reason);
							// NOTE: Some providers (e.g., Ollama) send tool_calls AND finish_reason in the same message.
							// We need to capture tool_calls here before continuing to the next message.
							// Capture tool_calls that arrive in the same chunk as finish_reason.
							// After capturing, emit the first ToolCallChunk so downstream
							// consumers (e.g. agent loops) see the tool call event.
							let mut first_tool_call_event: Option<ToolCall> = None;
							if let Ok(delta_tool_calls) = first_choice.x_take::<Value>("/delta/tool_calls")
								&& delta_tool_calls != Value::Null
								&& let Some(delta_tool_calls) = delta_tool_calls.as_array()
							{
								for tool_call_obj_val in delta_tool_calls {
									let mut tool_call_obj = tool_call_obj_val.clone();
									if let (Ok(index), Ok(mut function)) = (
										tool_call_obj.x_take::<u32>("index"),
										tool_call_obj.x_take::<Value>("function"),
									) {
										let call_id = tool_call_obj
											.x_take::<String>("id")
											.unwrap_or_else(|_| format!("call_{index}"));
										let fn_name = function.x_take::<String>("name").unwrap_or_default();
										let arguments = function.x_take::<String>("arguments").unwrap_or_default();

										let tc = self.capture_tool_call(index as usize, call_id, fn_name, arguments);
										if first_tool_call_event.is_none() {
											first_tool_call_event = Some(tc);
										}
									}
								}
							}

							if let Some(usage) =
								take_finish_reason_usage(&mut message_data, adapter_kind, self.options.capture_usage)
							{
								self.captured_data.usage = Some(usage);
							}

							// NOTE: Some providers (e.g., mistral) send delta/content AND finish_reason
							// in the same SSE message. We must capture and emit that final content chunk
							// before continuing to the next message, otherwise it is silently lost.
							let content = first_choice.x_take::<Option<String>>("/delta/content").ok().flatten();
							let reasoning_content = first_choice
								.x_take::<Option<String>>("/delta/reasoning_content")
								.ok()
								.flatten()
								.or_else(|| first_choice.x_take::<Option<String>>("/delta/reasoning").ok().flatten());

							if let Some(content) = content
								&& !content.is_empty()
							{
								if self.options.capture_content {
									match self.captured_data.content {
										Some(ref mut c) => c.push_str(&content),
										None => self.captured_data.content = Some(content.clone()),
									}
								}
								return Poll::Ready(Some(Ok(InterStreamEvent::Chunk(content))));
							} else if let Some(reasoning_content) = reasoning_content
								&& !reasoning_content.is_empty()
							{
								if self.options.capture_reasoning_content {
									match self.captured_data.reasoning_content {
										Some(ref mut c) => c.push_str(&reasoning_content),
										None => self.captured_data.reasoning_content = Some(reasoning_content.clone()),
									}
								}
								return Poll::Ready(Some(Ok(InterStreamEvent::ReasoningChunk(reasoning_content))));
							}

							// If we captured a tool call in the finish_reason chunk,
							// emit it as a ToolCallChunk so the agent loop sees it.
							if let Some(tc) = first_tool_call_event {
								return Poll::Ready(Some(Ok(InterStreamEvent::ToolCallChunk(tc))));
							}

							continue;
						}
						// -- Tool Call
						else if let Ok(delta_tool_calls) = first_choice.x_take::<Value>("/delta/tool_calls")
							&& delta_tool_calls != Value::Null
						{
							// Check if there's a tool call in the delta
							if let Some(delta_tool_calls) = delta_tool_calls.as_array()
								&& let Some(tool_call_obj_val) = delta_tool_calls.first()
							{
								// Extract the first tool call object as a mutable value
								let mut tool_call_obj = tool_call_obj_val.clone();

								// Extract tool call data
								if let (Ok(index), Ok(mut function)) = (
									tool_call_obj.x_take::<u32>("index"),
									tool_call_obj.x_take::<Value>("function"),
								) {
									let call_id = tool_call_obj
										.x_take::<String>("id")
										.unwrap_or_else(|_| format!("call_{index}"));
									let fn_name = function.x_take::<String>("name").unwrap_or_default();
									let arguments = function.x_take::<String>("arguments").unwrap_or_default();

									let tool_call = self.capture_tool_call(index as usize, call_id, fn_name, arguments);

									// Return the ToolCallChunk event
									return Poll::Ready(Some(Ok(InterStreamEvent::ToolCallChunk(tool_call))));
								}
							}
							// No valid tool call found, continue to next message
							continue;
						}
						// -- Content / Reasoning Content
						// Some providers (e.g., Ollama) emit reasoning in `delta.reasoning` and send empty content.
						else {
							let content = first_choice.x_take::<Option<String>>("/delta/content").ok().flatten();
							let reasoning_content = first_choice
								.x_take::<Option<String>>("/delta/reasoning_content")
								.ok()
								.flatten()
								.or_else(|| first_choice.x_take::<Option<String>>("/delta/reasoning").ok().flatten());

							if let Some(content) = content
								&& !content.is_empty()
							{
								// Add to the captured_content if chat options allow it
								if self.options.capture_content {
									match self.captured_data.content {
										Some(ref mut c) => c.push_str(&content),
										None => self.captured_data.content = Some(content.clone()),
									}
								}

								// Return the Event
								return Poll::Ready(Some(Ok(InterStreamEvent::Chunk(content))));
							} else if let Some(reasoning_content) = reasoning_content
								&& !reasoning_content.is_empty()
							{
								// Add to the captured_content if chat options allow it
								if self.options.capture_reasoning_content {
									match self.captured_data.reasoning_content {
										Some(ref mut c) => c.push_str(&reasoning_content),
										None => self.captured_data.reasoning_content = Some(reasoning_content.clone()),
									}
								}

								// Return the Event
								return Poll::Ready(Some(Ok(InterStreamEvent::ReasoningChunk(reasoning_content))));
							}

							// If we do not have content, then log a trace message
							// TODO: use tracing debug
							tracing::warn!("EMPTY CHOICE CONTENT");
						}
					}
					// -- Usage message
					else {
						// If it's not Groq, xAI, DeepSeek the usage is captured at the end when choices are empty or null
						if !matches!(adapter_kind, AdapterKind::Groq)
							&& !matches!(adapter_kind, AdapterKind::DeepSeek)
							&& self.captured_data.usage.is_none() // this might be redundant
							&& self.options.capture_usage
						{
							// permissive for now
							let usage = message_data
								.x_take("usage")
								.map(|v| OpenAIAdapter::into_usage(adapter_kind, v))
								.unwrap_or_default();
							self.captured_data.usage = Some(usage);
						}
					}
				}
				Some(Err(err)) => {
					tracing::error!("Error: {}", err);
					return Poll::Ready(Some(Err(Error::WebStream {
						model_iden: self.options.model_iden.clone(),
						cause: err.to_string(),
						error: err,
					})));
				}
				None => {
					return Poll::Ready(None);
				}
			}
		}
		Poll::Pending
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::adapter::AdapterKind;

	fn test_model() -> ModelIden {
		ModelIden::new(AdapterKind::OpenAI, "test-model")
	}

	#[test]
	fn test_take_stream_error_reads_openai_error_payload() {
		let mut message_data = serde_json::json!({
			"error": {
				"message": "Error in input stream",
				"type": "server_error",
			}
		});

		let err = take_stream_error(&mut message_data, &test_model()).expect("expected stream error");
		match err {
			Error::ChatResponse { body, .. } => {
				assert_eq!(body["message"], "Error in input stream");
				assert_eq!(body["type"], "server_error");
			}
			other => panic!("unexpected error variant: {other:?}"),
		}
	}

	#[test]
	fn test_take_stream_error_none_when_error_key_missing() {
		let mut message_data = serde_json::json!({
			"choices": [{"delta": {"content": "hi"}}]
		});
		assert!(take_stream_error(&mut message_data, &test_model()).is_none());
	}

	#[test]
	fn test_take_finish_reason_usage_reads_inline_openai_usage() {
		let mut message_data = serde_json::json!({
			"usage": {
				"prompt_tokens": 11,
				"completion_tokens": 3,
				"total_tokens": 14
			}
		});

		let usage =
			take_finish_reason_usage(&mut message_data, AdapterKind::OpenAI, true).expect("usage should be captured");

		assert_eq!(usage.prompt_tokens, Some(11));
		assert_eq!(usage.completion_tokens, Some(3));
		assert_eq!(usage.total_tokens, Some(14));
		assert!(message_data.get("usage").is_some_and(Value::is_null));
	}

	#[test]
	fn test_take_finish_reason_usage_respects_capture_flag() {
		let mut message_data = serde_json::json!({
			"usage": {
				"prompt_tokens": 11,
				"completion_tokens": 3,
				"total_tokens": 14
			}
		});

		let usage = take_finish_reason_usage(&mut message_data, AdapterKind::OpenAI, false);

		assert!(usage.is_none());
		assert_eq!(message_data["usage"]["prompt_tokens"], 11);
	}
}

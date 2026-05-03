use crate::adapter::adapters::support::{StreamerCapturedData, StreamerOptions};
use crate::adapter::cohere::CohereAdapter;
use crate::adapter::inter_stream::{InterStreamEnd, InterStreamEvent};
use crate::chat::{ChatOptionsSet, StopReason};
use crate::webc::WebStream;
use crate::{Error, ModelIden, Result};
use serde::Deserialize;
use serde_json::Value;
use std::pin::Pin;
use std::task::{Context, Poll};
use value_ext::JsonValueExt;

pub struct CohereStreamer {
	inner: WebStream,
	options: StreamerOptions,

	// -- Set by the poll_next
	/// Flag to prevent polling the EventSource after a MessageStop event
	done: bool,
	captured_data: StreamerCapturedData,
}

impl CohereStreamer {
	pub fn new(inner: WebStream, model_iden: ModelIden, options_set: ChatOptionsSet<'_, '_>) -> Self {
		Self {
			inner,
			done: false,
			options: StreamerOptions::new(model_iden, options_set),
			captured_data: Default::default(),
		}
	}
}

/// Required properties that need to be parsed
#[derive(Deserialize, Debug)]
struct CohereStreamMessage {
	#[allow(unused)]
	is_finished: bool,
	event_type: String,
	text: Option<String>,
	finish_reason: Option<String>,
	response: Option<CohereStreamMessageResponse>,
}
#[derive(Deserialize, Debug)]
struct CohereStreamMessageResponse {
	meta: Option<Value>,
}

// Implement futures::Stream for InterStream<CohereStream>
impl futures::Stream for CohereStreamer {
	type Item = Result<InterStreamEvent>;

	fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		if self.done {
			return Poll::Ready(None);
		}

		while let Poll::Ready(item) = Pin::new(&mut self.inner).poll_next(cx) {
			match item {
				Some(Ok(raw_string)) => {
					let cohere_message =
						serde_json::from_str::<CohereStreamMessage>(&raw_string).map_err(|serde_error| {
							Error::StreamParse {
								model_iden: self.options.model_iden.clone(),
								serde_error,
							}
						});

					match cohere_message {
						Ok(cohere_message) => {
							let inter_event = match cohere_message.event_type.as_str() {
								"stream-start" => InterStreamEvent::Start,
								"text-generation" => {
									if let Some(content) = cohere_message.text {
										// Add to the captured content if chat options allow it
										if self.options.capture_content {
											match self.captured_data.content {
												Some(ref mut c) => c.push_str(&content),
												None => self.captured_data.content = Some(content.clone()),
											}
										}
										InterStreamEvent::Chunk(content)
									} else {
										continue;
									}
								}
								"stream-end" => {
									// -- Capture stop_reason
									self.captured_data.stop_reason = cohere_message.finish_reason;

									// -- Capture usage
									let meta = cohere_message.response.and_then(|r| r.meta);
									let captured_usage = if self.options.capture_usage {
										meta.and_then(|mut v| v.x_take("tokens").ok())
											.map(CohereAdapter::into_usage)
											.map(|mut usage| {
												// Compute the total if any of input/output are not null
												if usage.prompt_tokens.is_some() || usage.completion_tokens.is_some() {
													usage.total_tokens = Some(
														usage.prompt_tokens.unwrap_or(0)
															+ usage.completion_tokens.unwrap_or(0),
													);
												}
												usage
											})
									} else {
										None
									};

									let inter_stream_end = InterStreamEnd {
										captured_usage,
										captured_stop_reason: self
											.captured_data
											.stop_reason
											.take()
											.map(StopReason::from),
										captured_text_content: self.captured_data.content.take(),
										captured_reasoning_content: self.captured_data.reasoning_content.take(),
										captured_tool_calls: self.captured_data.tool_calls.take(),
										captured_thought_signatures: None,
										captured_response_id: None,
									};

									InterStreamEvent::End(inter_stream_end)
								}
								_ => continue, // Skip the "other" event
							};

							return Poll::Ready(Some(Ok(inter_event)));
						}
						Err(err) => {
							tracing::error!("Cohere Adapter Stream Error: {}", err);
							return Poll::Ready(Some(Err(err)));
						}
					}
				}
				Some(Err(err)) => {
					tracing::error!("Cohere Adapter Stream Error: {}", err);
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

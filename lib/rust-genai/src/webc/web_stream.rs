use bytes::Bytes;
use futures::stream::TryStreamExt;
use futures::{Future, Stream};
use reqwest::{RequestBuilder, Response};
use std::collections::VecDeque;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::error::{BoxError, Error as GenaiError};

/// WebStream is a simple web stream implementation that splits the stream messages by a given delimiter.
/// - It is intended to be a pragmatic solution for services that do not adhere to the `text/event-stream` format and content type.
/// - For providers that support the standard `text/event-stream`, `genai` uses the `reqwest-eventsource`/`eventsource-stream` crates.
/// - This stream item is just a `String` and has different stream modes that define the message delimiter strategy (without any event typing).
/// - Each "Event" is just string-based and has only one event type, which is a string.
/// - It is the responsibility of the user of this stream to wrap it into a semantically correct stream of events depending on the domain.
#[allow(clippy::type_complexity)]
pub struct WebStream {
	stream_mode: StreamMode,
	reqwest_builder: Option<RequestBuilder>,
	response_future: Option<Pin<Box<dyn Future<Output = Result<Response, BoxError>> + Send>>>,
	bytes_stream: Option<Pin<Box<dyn Stream<Item = Result<Bytes, BoxError>> + Send>>>,
	// If a poll was a partial message, then we keep the previous part
	partial_message: Option<String>,
	// If a poll retrieved multiple messages, we keep them to be sent in the next poll
	remaining_messages: Option<VecDeque<String>>,
	// Incomplete trailing UTF-8 bytes from the previous chunk.
	// When a multi-byte character is split across TCP/HTTP chunk boundaries,
	// the trailing bytes are carried over to be prepended to the next chunk.
	utf8_carry: Vec<u8>,
}

pub enum StreamMode {
	// This is used for Cohere with a single `\n`
	Delimiter(&'static str),
	// This is for Gemini (standard JSON array, pretty formatted)
	PrettyJsonArray,
}

impl WebStream {
	pub fn new_with_delimiter(reqwest_builder: RequestBuilder, message_delimiter: &'static str) -> Self {
		Self {
			stream_mode: StreamMode::Delimiter(message_delimiter),
			reqwest_builder: Some(reqwest_builder),
			response_future: None,
			bytes_stream: None,
			partial_message: None,
			remaining_messages: None,
			utf8_carry: Vec::new(),
		}
	}

	pub fn new_with_pretty_json_array(reqwest_builder: RequestBuilder) -> Self {
		Self {
			stream_mode: StreamMode::PrettyJsonArray,
			reqwest_builder: Some(reqwest_builder),
			response_future: None,
			bytes_stream: None,
			partial_message: None,
			remaining_messages: None,
			utf8_carry: Vec::new(),
		}
	}
}

impl Stream for WebStream {
	type Item = Result<String, BoxError>;

	fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		let this = self.get_mut();

		// -- First, we check if we have any remaining messages to send.
		if let Some(ref mut remaining_messages) = this.remaining_messages
			&& let Some(msg) = remaining_messages.pop_front()
		{
			return Poll::Ready(Some(Ok(msg)));
		}

		// -- Then execute the web poll and processing loop
		loop {
			if let Some(ref mut fut) = this.response_future {
				match Pin::new(fut).poll(cx) {
					Poll::Ready(Ok(response)) => {
						// Check HTTP status before proceeding with the stream
						let status = response.status();
						if !status.is_success() {
							this.response_future = None;
							// For error responses, we need to read the body to get the error message
							// Store a future that reads the body and returns an error
							let error_future = async move {
								let body = response
									.text()
									.await
									.unwrap_or_else(|e| format!("Failed to read error body: {}", e));
								Err::<Response, BoxError>(Box::new(GenaiError::HttpError {
									status,
									canonical_reason: status.canonical_reason().unwrap_or("Unknown").to_string(),
									body,
								}))
							};
							this.response_future = Some(Box::pin(error_future));
							continue;
						}
						let bytes_stream = response.bytes_stream().map_err(|e| Box::new(e) as BoxError);
						this.bytes_stream = Some(Box::pin(bytes_stream));
						this.response_future = None;
					}
					Poll::Ready(Err(e)) => {
						this.response_future = None;
						return Poll::Ready(Some(Err(e)));
					}
					Poll::Pending => return Poll::Pending,
				}
			}

			if let Some(ref mut stream) = this.bytes_stream {
				match stream.as_mut().poll_next(cx) {
					Poll::Ready(Some(Ok(bytes))) => {
						// -- Incremental UTF-8 decoding: prepend any carried-over
						// bytes from the previous chunk, decode as much valid UTF-8
						// as possible, and carry over any incomplete trailing sequence.
						let mut raw = std::mem::take(&mut this.utf8_carry);
						raw.extend_from_slice(&bytes);

						let valid_up_to = match std::str::from_utf8(&raw) {
							Ok(_) => raw.len(),
							Err(e) => {
								if e.error_len().is_some() {
									// Actual invalid UTF-8 (not just incomplete) — fatal error.
									return Poll::Ready(Some(Err(
										Box::new(String::from_utf8(raw).unwrap_err()) as BoxError
									)));
								}
								e.valid_up_to()
							}
						};

						// Carry over incomplete trailing bytes for the next chunk.
						this.utf8_carry = raw[valid_up_to..].to_vec();

						// We already validated raw[..valid_up_to] is valid UTF-8 above.
						let buff_string = String::from_utf8(raw[..valid_up_to].to_vec()).unwrap();

						// -- Iterate through the parts
						let buff_response = match this.stream_mode {
							StreamMode::Delimiter(delimiter) => {
								process_buff_string_delimited(buff_string, &mut this.partial_message, delimiter)
							}
							StreamMode::PrettyJsonArray => {
								new_with_pretty_json_array(buff_string, &mut this.partial_message)
							}
						};

						let BuffResponse {
							mut first_message,
							next_messages,
							candidate_message,
						} = buff_response?;

						// -- Add next_messages as remaining messages if present
						if let Some(next_messages) = next_messages {
							this.remaining_messages.get_or_insert(VecDeque::new()).extend(next_messages);
						}

						// -- If we still have a candidate, it's the partial for the next one
						if let Some(candidate_message) = candidate_message {
							// For now, we will just log this
							if this.partial_message.is_some() {
								tracing::warn!("GENAI - WARNING - partial_message is not none");
							}
							this.partial_message = Some(candidate_message);
						}

						// -- If we have a first message, we have to send it.
						if let Some(first_message) = first_message.take() {
							return Poll::Ready(Some(Ok(first_message)));
						} else {
							continue;
						}
					}
					Poll::Ready(Some(Err(e))) => return Poll::Ready(Some(Err(e))),
					Poll::Ready(None) => {
						if let Some(partial) = this.partial_message.take()
							&& !partial.is_empty()
						{
							return Poll::Ready(Some(Ok(partial)));
						}
						this.bytes_stream = None;
					}
					Poll::Pending => return Poll::Pending,
				}
			}

			if let Some(reqwest_builder) = this.reqwest_builder.take() {
				let fut = async move { reqwest_builder.send().await.map_err(|e| Box::new(e) as BoxError) };
				this.response_future = Some(Box::pin(fut));
				continue;
			}

			return Poll::Ready(None);
		}
	}
}

struct BuffResponse {
	first_message: Option<String>,
	next_messages: Option<Vec<String>>,
	candidate_message: Option<String>,
}

/// Process a string buffer for the pretty_json_array (for Gemini)
/// It will split the messages as follows:
/// - If it starts with `[`, then the message will be `[`
/// - Then, each main JSON object (from the first `{` to the last `}`) will become a message
/// - Main JSON object `,` delimiter will be skipped
/// - The ending `]` will be sent as a `]` message as well.
///
/// IMPORTANT: Right now, it assumes each buff_string will contain the full main JSON object
///            for each array item (which seems to be the case with Gemini).
///            This probably needs to be made more robust later.
fn new_with_pretty_json_array(
	buff_string: String,
	partial_message: &mut Option<String>,
) -> Result<BuffResponse, crate::webc::Error> {
	let mut buff_str = buff_string.as_str();

	let mut messages: Vec<String> = Vec::new();

	// -- 1. Prepend partial message if any
	let full_string_holder: String;
	if let Some(partial) = partial_message.take() {
		full_string_holder = format!("{}{}", partial, buff_str);
		buff_str = full_string_holder.as_str();
	}

	// -- 2. Process the buffer
	// We want to extract valid JSON objects.
	// The stream is expected to be: `[` (optional), `{...}`, `,`, `{...}`, `]` (optional)
	// We need to be robust against whitespace and commas.

	let mut depth = 0;
	let mut in_string = false;
	let mut escape = false;
	let mut start_idx = 0;
	let mut last_idx = 0; // Track the end of the last processed object

	for (idx, c) in buff_str.char_indices() {
		if in_string {
			if escape {
				escape = false;
			} else if c == '\\' {
				escape = true;
			} else if c == '"' {
				in_string = false;
			}
		} else {
			match c {
				'"' => in_string = true,
				'{' => {
					if depth == 0 {
						start_idx = idx;
					}
					depth += 1;
				}
				'}' => {
					depth -= 1;
					if depth == 0 {
						// Found a complete JSON object
						// idx is the byte index of '}'. We want to include it.
						// '}' is 1 byte, so end range is idx + 1
						let json_str = &buff_str[start_idx..idx + 1];

						// Verify it's valid JSON (optional but good for safety)
						if serde_json::from_str::<serde_json::Value>(json_str).is_ok() {
							messages.push(json_str.to_string());
						} else {
							// Should not happen if logic is correct
							tracing::warn!("WebStream: Extracted block failed JSON validation: {}", json_str);
						}
						// Update last_idx to point after this object
						last_idx = idx + 1;
					}
				}
				'[' if depth == 0 => {
					messages.push("[".to_string());
					last_idx = idx + 1;
				}
				']' if depth == 0 => {
					messages.push("]".to_string());
					last_idx = idx + 1;
				}
				_ => {
					// Ignore other characters outside of objects (whitespace, commas)
				}
			}
		}
	}

	// -- 3. Handle remaining partial
	// last_idx points to the byte after the last successfully processed object/token
	if last_idx < buff_str.len() {
		let remaining = &buff_str[last_idx..];
		if !remaining.trim().is_empty() {
			*partial_message = Some(remaining.to_string());
		}
	}

	// -- Return the buff response
	let first_message = if !messages.is_empty() {
		Some(messages[0].to_string())
	} else {
		None
	};

	let next_messages = if messages.len() > 1 {
		Some(messages[1..].to_vec())
	} else {
		None
	};

	Ok(BuffResponse {
		first_message,
		next_messages,
		candidate_message: partial_message.take(),
	})
}

/// Process a string buffer for the delimited mode (e.g., Cohere)
fn process_buff_string_delimited(
	buff_string: String,
	partial_message: &mut Option<String>,
	delimiter: &str,
) -> Result<BuffResponse, crate::webc::Error> {
	let full_string = if let Some(partial) = partial_message.take() {
		format!("{partial}{buff_string}")
	} else {
		buff_string
	};

	let mut parts: Vec<String> = full_string.split(delimiter).map(|s| s.to_string()).collect();

	// The last part is the new partial (what's after the last delimiter)
	let candidate_message = parts.pop();

	// Filter out empty strings that result from multiple delimiters (e.g., \n\n\n\n)
	let mut messages: Vec<String> = parts.into_iter().filter(|s| !s.is_empty()).collect();

	let mut first_message = None;
	let mut next_messages = None;

	if !messages.is_empty() {
		first_message = Some(messages.remove(0));
		if !messages.is_empty() {
			next_messages = Some(messages);
		}
	}

	Ok(BuffResponse {
		first_message,
		next_messages,
		candidate_message,
	})
}

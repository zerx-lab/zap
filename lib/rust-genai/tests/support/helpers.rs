use super::TestResult;
use bitflags::parser::to_writer;
use genai::chat::{ChatStream, ChatStreamEvent, StreamEnd, ToolCall};
use tokio_stream::StreamExt;

/// A macro to retrieve the value of an `Option` field from a struct, returning an error if the field is `None`.
///
/// # Arguments
///
/// * `$expr` - The expression representing the struct and its field.
///
/// # Example
///
/// ```rust
/// let meta_usage = get_option_value!(stream_end.meta_usage);
/// ```
///
/// This macro expands to:
///
/// ```rust
/// let meta_usage = stream_end.meta_usage.ok_or("Should have meta_usage")?;
/// ```
#[macro_export]
macro_rules! get_option_value {
	($struct:ident.$field:ident) => {
		$struct.$field.ok_or(concat!("Should have ", stringify!($field)))?
	};
}

// region:    --- Check Flags

bitflags::bitflags! {
	#[derive(Clone)]
	pub struct Check: u8 {
		/// Check if the
		const REASONING_CONTENT       = 0b00000001;
		const REASONING_USAGE = 0b00000010;
		const USAGE           = 0b00000100;
	}
}

// Custom Debug Implementation Using `to_writer`
impl std::fmt::Debug for Check {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let mut buffer = String::new();
		to_writer(self, &mut buffer)?;
		write!(f, "{buffer}")
	}
}

pub fn contains_checks(checks: Option<Check>, matching_check: Check) -> bool {
	let Some(checks) = checks else { return false };

	checks.contains(matching_check)
}

// Function to validate flags
pub fn validate_checks(checks: Option<Check>, valid_flags: Check) -> TestResult<()> {
	let Some(checks) = checks else { return Ok(()) };

	let unsupported = checks - valid_flags;
	if !unsupported.is_empty() {
		return Err(format!("Unsupported flags passed for this test: {unsupported:?}").into());
	}

	Ok(())
}

// endregion: --- Check Flags

// region:    --- Stream Support

pub struct StreamExtract {
	// The stream end event
	pub stream_end: StreamEnd,

	// The extracted text content (does not need to have capture ChatOptions)
	pub content: Option<String>,

	pub reasoning_content: Option<String>,

	/// All ToolCallChunk events received during streaming, in order.
	pub tool_call_chunks: Vec<ToolCall>,
}

pub async fn extract_stream_end(mut chat_stream: ChatStream) -> TestResult<StreamExtract> {
	let mut stream_end: Option<StreamEnd> = None;

	let mut content: Vec<String> = Vec::new();
	let mut reasoning_content: Vec<String> = Vec::new();
	let mut tool_call_chunks: Vec<ToolCall> = Vec::new();

	while let Some(Ok(stream_event)) = chat_stream.next().await {
		match stream_event {
			ChatStreamEvent::Start => (), // nothing to do
			ChatStreamEvent::Chunk(s_chunk) => content.push(s_chunk.content),
			ChatStreamEvent::ReasoningChunk(s_chunk) => reasoning_content.push(s_chunk.content),
			ChatStreamEvent::ThoughtSignatureChunk(_) => (), // ignore thought signature chunks for now
			ChatStreamEvent::ToolCallChunk(tc) => tool_call_chunks.push(tc.tool_call),
			ChatStreamEvent::End(s_end) => {
				stream_end = Some(s_end);
				break;
			}
		}
	}

	let stream_end = stream_end.ok_or("Should have a StreamEnd event")?;
	let content = (!content.is_empty()).then(|| content.join(""));
	let reasoning_content = (!reasoning_content.is_empty()).then(|| reasoning_content.join(""));

	Ok(StreamExtract {
		stream_end,
		content,
		reasoning_content,
		tool_call_chunks,
	})
}

// endregion: --- Stream Support

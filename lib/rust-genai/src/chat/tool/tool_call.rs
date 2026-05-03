use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The tool call function name and arguments sent back by the LLM.
/// Represents a single function/tool invocation emitted by the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
	/// Stable identifier for this tool call (used to correlate tool responses).
	pub call_id: String,

	/// Name of the function to invoke.
	pub fn_name: String,

	/// JSON arguments payload as provided by the model.
	/// Kept as `serde_json::Value` so callers can deserialize into their own types.
	pub fn_arguments: Value,

	/// Optional thought signatures that should precede tool calls in the assistant turn.
	///
	/// When present on the first tool call in a batch, `ChatMessage::from(Vec<ToolCall>)`
	/// will automatically include these as leading `ThoughtSignature` parts in the
	/// assistant message content. This enables simple continuations like:
	/// `append_message(tool_calls).append_message(tool_response)` without having to
	/// manually inject thoughts.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub thought_signatures: Option<Vec<String>>,
}

/// Computed accessors
impl ToolCall {
	/// Returns an approximate in-memory size of this `ToolCall`, in bytes,
	/// computed as the sum of the UTF-8 lengths of:
	/// - `call_id`
	/// - `fn_name`
	/// - JSON-serialized `fn_arguments`
	/// - all `thought_signatures` strings (if any)
	pub fn size(&self) -> usize {
		let mut size = self.call_id.len();
		size += self.fn_name.len();
		size += serde_json::to_string(&self.fn_arguments).map(|j| j.len()).unwrap_or_default();
		size += self
			.thought_signatures
			.as_ref()
			.map(|sigs| sigs.iter().map(|s| s.len()).sum::<usize>())
			.unwrap_or_default();
		size
	}
}

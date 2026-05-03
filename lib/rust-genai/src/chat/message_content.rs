/// Note: MessageContent is used for ChatRequest and ChatResponse.
use crate::chat::{Binary, ContentPart, CustomPart, ToolCall, ToolResponse};
use serde::{Deserialize, Serialize};

/// Message content container used in ChatRequest and ChatResponse.
///
/// Transparent wrapper around a list of ContentPart (Text, Binary, ToolCall, or ToolResponse).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MessageContent {
	/// The parts that compose this message.
	parts: Vec<ContentPart>,
}

/// Constructors
impl MessageContent {
	/// Create a message containing a single text part.
	pub fn from_text(content: impl Into<String>) -> Self {
		Self {
			parts: vec![ContentPart::Text(content.into())],
		}
	}

	/// Build from the provided content parts.
	pub fn from_parts(parts: impl Into<Vec<ContentPart>>) -> Self {
		Self { parts: parts.into() }
	}

	/// Build from the provided tool calls.
	pub fn from_tool_calls(tool_calls: Vec<ToolCall>) -> Self {
		Self {
			parts: tool_calls.into_iter().map(ContentPart::ToolCall).collect(),
		}
	}

	/// Build from the provided tool responses.
	pub fn from_tool_responses(tool_responses: Vec<ToolResponse>) -> Self {
		Self {
			parts: tool_responses.into_iter().map(ContentPart::ToolResponse).collect(),
		}
	}
}

/// Fluid Setters/Builders
impl MessageContent {
	/// Append one part and return self (builder style).
	pub fn append(mut self, part: impl Into<ContentPart>) -> Self {
		self.parts.push(part.into());
		self
	}

	/// Append one part (mutating).
	pub fn push(&mut self, part: impl Into<ContentPart>) {
		self.parts.push(part.into());
	}

	/// Insert one part at the given index (mutating).
	pub fn insert(&mut self, index: usize, part: impl Into<ContentPart>) {
		self.parts.insert(index, part.into());
	}

	/// Prepend one part to the beginning (mutating).
	pub fn prepend(&mut self, part: impl Into<ContentPart>) {
		self.parts.insert(0, part.into());
	}

	/// Prepend multiple parts while preserving their original order.
	pub fn extend_front<I>(&mut self, iter: I)
	where
		I: IntoIterator<Item = ContentPart>,
	{
		// Collect then insert in reverse so that the first element in `iter`
		// ends up closest to the front after all insertions.
		let collected: Vec<ContentPart> = iter.into_iter().collect();
		for part in collected.into_iter().rev() {
			self.parts.insert(0, part);
		}
	}

	/// Extend with an iterator of parts, returning self.
	pub fn extended<I>(mut self, iter: I) -> Self
	where
		I: IntoIterator<Item = ContentPart>,
	{
		self.parts.extend(iter);
		self
	}
}

impl Extend<ContentPart> for MessageContent {
	fn extend<T: IntoIterator<Item = ContentPart>>(&mut self, iter: T) {
		self.parts.extend(iter);
	}
}

/// Computed accessors
impl MessageContent {
	/// Returns an approximate in-memory size of this `MessageContent`, in bytes,
	/// computed as the sum of the sizes of all parts.
	pub fn size(&self) -> usize {
		self.parts.iter().map(|p| p.size()).sum()
	}
}

// region:    --- Iterator Support

use crate::support;
use std::iter::FromIterator;
use std::slice::{Iter, IterMut};

impl IntoIterator for MessageContent {
	type Item = ContentPart;
	type IntoIter = std::vec::IntoIter<ContentPart>;
	fn into_iter(self) -> Self::IntoIter {
		self.parts.into_iter()
	}
}

impl<'a> IntoIterator for &'a MessageContent {
	type Item = &'a ContentPart;
	type IntoIter = Iter<'a, ContentPart>;
	fn into_iter(self) -> Self::IntoIter {
		self.parts.iter()
	}
}

impl<'a> IntoIterator for &'a mut MessageContent {
	type Item = &'a mut ContentPart;
	type IntoIter = IterMut<'a, ContentPart>;
	fn into_iter(self) -> Self::IntoIter {
		self.parts.iter_mut()
	}
}

// collect() support
impl FromIterator<ContentPart> for MessageContent {
	fn from_iter<T: IntoIterator<Item = ContentPart>>(iter: T) -> Self {
		Self {
			parts: iter.into_iter().collect(),
		}
	}
}

// endregion: --- Iterator Support

/// Getters
impl MessageContent {
	pub fn iter(&self) -> Iter<'_, ContentPart> {
		self.parts.iter()
	}

	pub fn iter_mut(&mut self) -> IterMut<'_, ContentPart> {
		self.parts.iter_mut()
	}

	/// Return all parts.
	pub fn parts(&self) -> &Vec<ContentPart> {
		&self.parts
	}

	/// Consume and return the underlying parts.
	pub fn into_parts(self) -> Vec<ContentPart> {
		self.parts
	}

	/// Return all text parts as &str.
	pub fn texts(&self) -> Vec<&str> {
		self.parts.iter().filter_map(|p| p.as_text()).collect()
	}

	/// Consume and return all text parts as owned Strings.
	pub fn into_texts(self) -> Vec<String> {
		self.parts.into_iter().filter_map(|p| p.into_text()).collect()
	}

	pub fn binaries(&self) -> Vec<&Binary> {
		self.parts.iter().filter_map(|p| p.as_binary()).collect()
	}

	pub fn into_binaries(self) -> Vec<Binary> {
		self.parts.into_iter().filter_map(|p| p.into_binary()).collect()
	}

	/// Return references to all ThoughtSignature parts as &str.
	pub fn thought_signatures(&self) -> Vec<&str> {
		self.parts.iter().filter_map(|p| p.as_thought_signature()).collect()
	}

	/// Consume and return all ThoughtSignature parts as owned Strings.
	pub fn into_thought_signatures(self) -> Vec<String> {
		self.parts.into_iter().filter_map(|p| p.into_thought_signature()).collect()
	}

	/// Return references to all ToolCall parts.
	pub fn tool_calls(&self) -> Vec<&ToolCall> {
		self.parts
			.iter()
			.filter_map(|p| match p {
				ContentPart::ToolCall(tc) => Some(tc),
				_ => None,
			})
			.collect()
	}

	/// Consume and return all ToolCall parts.
	pub fn into_tool_calls(self) -> Vec<ToolCall> {
		self.parts
			.into_iter()
			.filter_map(|p| match p {
				ContentPart::ToolCall(tc) => Some(tc),
				_ => None,
			})
			.collect()
	}

	/// Return references to all ToolResponse parts.
	pub fn tool_responses(&self) -> Vec<&ToolResponse> {
		self.parts
			.iter()
			.filter_map(|p| match p {
				ContentPart::ToolResponse(tr) => Some(tr),
				_ => None,
			})
			.collect()
	}

	/// Consume and return all ToolResponse parts.
	pub fn into_tool_responses(self) -> Vec<ToolResponse> {
		self.parts
			.into_iter()
			.filter_map(|p| match p {
				ContentPart::ToolResponse(tr) => Some(tr),
				_ => None,
			})
			.collect()
	}

	/// Return references to all custom parts.
	pub fn custom_parts(&self) -> Vec<&CustomPart> {
		self.parts.iter().filter_map(|p| p.as_custom()).collect()
	}

	/// Consume and return all custom parts.
	pub fn into_custom_parts(self) -> Vec<CustomPart> {
		self.parts.into_iter().filter_map(|p| p.into_custom()).collect()
	}

	/// True if there are no parts.
	pub fn is_empty(&self) -> bool {
		self.parts.is_empty()
	}

	/// Number of parts.
	pub fn len(&self) -> usize {
		self.parts.len()
	}

	/// Return references to all ReasoningContent parts as &str.
	pub fn reasoning_contents(&self) -> Vec<&str> {
		self.parts.iter().filter_map(|p| p.as_reasoning_content()).collect()
	}

	/// Consume and return all ReasoningContent parts as owned Strings.
	pub fn into_reasoning_contents(self) -> Vec<String> {
		self.parts.into_iter().filter_map(|p| p.into_reasoning_content()).collect()
	}

	/// Join all reasoning content parts with a newline separator.
	/// Returns None if there are no reasoning content parts.
	pub fn joined_reasoning_content(&self) -> Option<String> {
		let parts = self.reasoning_contents();
		if parts.is_empty() {
			return None;
		}
		Some(parts.join("\n"))
	}

	/// True if empty, or if all parts are text whose content is empty or whitespace.
	pub fn is_text_empty(&self) -> bool {
		if self.parts.is_empty() {
			return true;
		}
		self.parts
			.iter()
			.all(|p| matches!(p, ContentPart::Text(t) if t.trim().is_empty()))
	}
}

/// Convenient Getters
impl MessageContent {
	/// Return the first text part, if any.
	///
	/// Does not concatenate multiple text parts.
	pub fn first_text(&self) -> Option<&str> {
		let first_text_part = self.parts.iter().find(|p| p.is_text())?;
		first_text_part.as_text()
	}

	/// Consume and return the first text part as a String, if any.
	///
	/// Does not concatenate multiple text parts.
	pub fn into_first_text(self) -> Option<String> {
		let first_text_part = self.parts.into_iter().find(|p| p.is_text())?;
		first_text_part.into_text()
	}

	/// Return the first reasoning content part, if any.
	///
	/// Does not concatenate multiple reasoning content parts.
	pub fn first_reasoning_content(&self) -> Option<&str> {
		let first_reasoning_part = self.parts.iter().find(|p| p.is_reasoning_content())?;
		first_reasoning_part.as_reasoning_content()
	}

	/// Consume and return the first reasoning content part as a String, if any.
	///
	/// Does not concatenate multiple reasoning content parts.
	pub fn into_first_reasoning_content(self) -> Option<String> {
		let first_reasoning_part = self.parts.into_iter().find(|p| p.is_reasoning_content())?;
		first_reasoning_part.into_reasoning_content()
	}

	/// Return the first thought signature part, if any.
	///
	/// Does not concatenate multiple thought signature parts.
	pub fn first_thought_signature(&self) -> Option<&str> {
		let first_thought_signature_part = self.parts.iter().find(|p| p.is_thought_signature())?;
		first_thought_signature_part.as_thought_signature()
	}

	/// Consume and return the first thought signature part as a String, if any.
	///
	/// Does not concatenate multiple thought signature parts.
	pub fn into_first_thought_signature(self) -> Option<String> {
		let first_thought_signature_part = self.parts.into_iter().find(|p| p.is_thought_signature())?;
		first_thought_signature_part.into_thought_signature()
	}

	/// Join all text parts, separating segments with a blank line.
	pub fn joined_texts(&self) -> Option<String> {
		let texts = self.texts();
		if texts.is_empty() {
			return None;
		}

		if texts.len() == 1 {
			return texts.first().map(|s| s.to_string());
		}

		let mut combined = String::new();
		for text in texts {
			support::combine_text_with_empty_line(&mut combined, text);
		}
		Some(combined)
	}

	/// Consume and join all text parts, separating segments with a blank line.
	pub fn into_joined_texts(self) -> Option<String> {
		let texts = self.into_texts();
		if texts.is_empty() {
			return None;
		}

		if texts.len() == 1 {
			return texts.into_iter().next();
		}

		let mut combined = String::new();
		for text in texts {
			support::combine_text_with_empty_line(&mut combined, &text);
		}
		Some(combined)
	}
}

/// is_.., contains_..
impl MessageContent {
	/// True if every part is text.
	pub fn is_text_only(&self) -> bool {
		self.parts.iter().all(|p| p.is_text())
	}

	/// True if at least one part is text.
	pub fn contains_text(&self) -> bool {
		self.parts.iter().any(|p| p.is_text())
	}

	/// True if at least one part is binary.
	pub fn contains_binary(&self) -> bool {
		self.parts.iter().any(|p| p.is_binary())
	}

	/// True if at least one part is a ToolCall.
	pub fn contains_tool_call(&self) -> bool {
		self.parts.iter().any(|p| p.is_tool_call())
	}

	/// True if at least one part is a ToolResponse.
	pub fn contains_tool_response(&self) -> bool {
		self.parts.iter().any(|p| p.is_tool_response())
	}

	/// True if at least one part is a ThoughtSignature.
	pub fn contains_thought_signature(&self) -> bool {
		self.parts.iter().any(|p| p.is_thought_signature())
	}

	/// True if at least one part is ReasoningContent.
	pub fn contains_reasoning_content(&self) -> bool {
		self.parts.iter().any(|p| p.is_reasoning_content())
	}

	/// True if at least one part is Custom.
	pub fn contains_custom(&self) -> bool {
		self.parts.iter().any(|p| p.is_custom())
	}
}

// region:    --- Froms

impl From<&str> for MessageContent {
	fn from(s: &str) -> Self {
		Self {
			parts: vec![ContentPart::Text(s.to_string())],
		}
	}
}

impl From<&String> for MessageContent {
	fn from(s: &String) -> Self {
		Self {
			parts: vec![ContentPart::Text(s.clone())],
		}
	}
}

impl From<String> for MessageContent {
	fn from(s: String) -> Self {
		Self {
			parts: vec![ContentPart::Text(s)],
		}
	}
}

impl From<Vec<ToolCall>> for MessageContent {
	fn from(tool_calls: Vec<ToolCall>) -> Self {
		Self {
			parts: tool_calls.into_iter().map(ContentPart::ToolCall).collect(),
		}
	}
}

impl From<ToolCall> for MessageContent {
	fn from(tool_call: ToolCall) -> Self {
		Self {
			parts: vec![ContentPart::ToolCall(tool_call)],
		}
	}
}

impl From<ToolResponse> for MessageContent {
	fn from(tool_response: ToolResponse) -> Self {
		Self {
			parts: vec![ContentPart::ToolResponse(tool_response)],
		}
	}
}

impl From<Vec<ToolResponse>> for MessageContent {
	fn from(tool_responses: Vec<ToolResponse>) -> Self {
		Self {
			parts: tool_responses.into_iter().map(ContentPart::ToolResponse).collect(),
		}
	}
}

impl From<ContentPart> for MessageContent {
	fn from(part: ContentPart) -> Self {
		Self { parts: vec![part] }
	}
}

impl From<Binary> for MessageContent {
	fn from(bin: Binary) -> Self {
		Self {
			parts: vec![bin.into()],
		}
	}
}

impl From<CustomPart> for MessageContent {
	fn from(custom_part: CustomPart) -> Self {
		Self {
			parts: vec![ContentPart::Custom(custom_part)],
		}
	}
}

impl From<Vec<ContentPart>> for MessageContent {
	fn from(parts: Vec<ContentPart>) -> Self {
		Self { parts }
	}
}

// endregion: --- Froms

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_message_content_joined_texts_empty() {
		assert_eq!(MessageContent::from_parts(vec![]).joined_texts(), None);
	}

	#[test]
	fn test_message_content_joined_texts_single_part() {
		assert_eq!(
			MessageContent::from_parts(vec![ContentPart::Text("Hello".to_string())]).joined_texts(),
			Some("Hello".to_string())
		);
	}

	#[test]
	fn test_message_content_joined_texts_two_parts() {
		assert_eq!(
			MessageContent::from_parts(vec![
				ContentPart::Text("Hello".to_string()),
				ContentPart::Text("World".to_string()),
			])
			.joined_texts(),
			Some("Hello\n\nWorld".to_string())
		);
	}

	#[test]
	fn test_message_content_into_joined_texts_empty() {
		assert_eq!(MessageContent::from_parts(vec![]).into_joined_texts(), None);
	}

	#[test]
	fn test_message_content_into_joined_texts_single_part() {
		assert_eq!(
			MessageContent::from_parts(vec![ContentPart::Text("Hello".to_string())]).into_joined_texts(),
			Some("Hello".to_string())
		);
	}

	#[test]
	fn test_message_content_into_joined_texts_two_parts() {
		assert_eq!(
			MessageContent::from_parts(vec![
				ContentPart::Text("Hello".to_string()),
				ContentPart::Text("World".to_string()),
			])
			.into_joined_texts(),
			Some("Hello\n\nWorld".to_string())
		);
	}
}

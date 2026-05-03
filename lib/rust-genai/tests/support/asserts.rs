//! Some assert convenient functions

use crate::support::TestResult;
use genai::chat::{ChatResponse, Usage};
use std::convert::Into;
use std::fmt::Formatter;

pub fn assert_contains<'a, T>(data: T, val: &str)
where
	T: Into<DataContainer<'a>>,
{
	let container: DataContainer = data.into();
	assert!(container.contains(val), "Should contain: {val}\nBut was: {container:?}");
}

pub fn assert_not_contains<'a, T>(data: T, val: &str)
where
	T: Into<DataContainer<'a>>,
{
	let container: DataContainer = data.into();
	assert!(
		!container.contains(val),
		"Should not contain: {val}\nBut was: {container:?}"
	);
}

pub fn assert_reasoning_usage(usage: &Usage) -> TestResult<()> {
	let reasoning_tokens = usage
		.completion_tokens_details
		.as_ref()
		.and_then(|v| v.reasoning_tokens)
		.ok_or("should have reasoning_tokens")?;
	assert!(reasoning_tokens > 0, "reasoning_usage should be > 0");

	Ok(())
}

pub fn assert_reasoning_content(chat_res: &ChatResponse) -> TestResult<()> {
	let reasoning_content = chat_res
		.reasoning_content
		.as_deref()
		.ok_or("SHOULD have extracted some reasoning_content")?;
	let content = chat_res.content.joined_texts().unwrap_or_default();
	assert!(!reasoning_content.is_empty(), "reasoning_content should not be empty");
	// We can assume that the reasoning content should be bigger than the content given the prompt to keep content very concise.
	assert!(
		reasoning_content.len() > content.len(),
		"Reasoning content should be > than the content"
	);
	Ok(())
}

// region:    --- Support Types

pub enum DataContainer<'a> {
	Owned(Vec<&'a str>),
	Slice(&'a [&'a str]),
	Str(&'a str),
}

impl std::fmt::Debug for DataContainer<'_> {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match self {
			DataContainer::Owned(vec) => write!(f, "{vec:?}"),
			DataContainer::Slice(slice) => write!(f, "{slice:?}"),
			DataContainer::Str(s) => {
				write!(f, "{s}")
			}
		}
	}
}

impl<'a> From<&'a [&'a str]> for DataContainer<'a> {
	fn from(slice: &'a [&'a str]) -> Self {
		DataContainer::Slice(slice)
	}
}

impl<'a> From<&'a Vec<&'a str>> for DataContainer<'a> {
	fn from(vec: &'a Vec<&'a str>) -> Self {
		DataContainer::Slice(&vec[..])
	}
}

impl<'a> From<&'a Vec<String>> for DataContainer<'a> {
	fn from(vec: &'a Vec<String>) -> Self {
		DataContainer::Owned(vec.iter().map(|s| s.as_str()).collect())
	}
}

impl<'a> From<&'a str> for DataContainer<'a> {
	fn from(string: &'a str) -> Self {
		DataContainer::Str(string)
	}
}

impl<'a> From<&'a String> for DataContainer<'a> {
	fn from(string: &'a String) -> Self {
		DataContainer::Str(string)
	}
}

impl DataContainer<'_> {
	fn contains(&self, val: &str) -> bool {
		match self {
			DataContainer::Owned(vec) => vec.contains(&val),
			DataContainer::Slice(slice) => slice.contains(&val),
			DataContainer::Str(string) => string.contains(val),
		}
	}
}

// endregion: --- Support Types

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>; // For tests.

	use super::*;

	// #[test]
	// fn test_assert_contains() -> Result<()> {
	// 	let data_vec = vec!["apple", "banana", "cherry"];
	// 	assert_contains(&data_vec, "banana");

	// 	let data_slice: &[&str] = &["dog", "cat", "mouse"];
	// 	assert_contains(data_slice, "cat");

	// 	let data_str = "This is a test string";
	// 	assert_contains(data_str, "test");

	// 	Ok(())
	// }
}

// endregion: --- Tests

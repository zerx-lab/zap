use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none};

/// Normalized token usage across providers (OpenAI-compatible).
///
/// - Deserialization treats 0 as None for cross-provider consistency. OpenAI often returns 0 for non-applicable counters.
///
/// - `prompt_tokens` and `completion_tokens` are the total input/output tokens. The corresponding `*_details` carry provider-specific breakdowns.
///
/// - Gemini: `candidatesTokenCount` excludes "thoughts" (reasoning) tokens. We normalize:
///   `completion_tokens = candidatesTokenCount + thoughts_token_count`, and
///   `completion_tokens_details.reasoning_tokens = thoughts_token_count`.
#[serde_as]
#[skip_serializing_none]
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
	/// Total input tokens (formerly `input_tokens`).
	pub prompt_tokens: Option<i32>,
	pub prompt_tokens_details: Option<PromptTokensDetails>,

	/// Total output (completion) tokens.
	pub completion_tokens: Option<i32>,
	pub completion_tokens_details: Option<CompletionTokensDetails>,

	/// Total tokens as reported by the API, or computed as prompt + completion
	/// (including cache read/creation tokens when applicable).
	pub total_tokens: Option<i32>,
}

impl Usage {
	/// Remove detail objects that contain only `None` fields.
	pub fn compact_details(&mut self) {
		if self.prompt_tokens_details.as_ref().is_some_and(|d| d.is_empty()) {
			self.prompt_tokens_details = None;
		}

		if self.completion_tokens_details.as_ref().is_some_and(|d| d.is_empty()) {
			self.completion_tokens_details = None;
		}
	}
}

/// Breakdown of cache creation tokens by TTL.
#[serde_as]
#[skip_serializing_none]
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct CacheCreationDetails {
	/// Tokens written to 5-minute ephemeral cache.
	#[serde(default, deserialize_with = "crate::support::zero_as_none")]
	pub ephemeral_5m_tokens: Option<i32>,
	/// Tokens written to 1-hour ephemeral cache.
	#[serde(default, deserialize_with = "crate::support::zero_as_none")]
	pub ephemeral_1h_tokens: Option<i32>,
}

impl CacheCreationDetails {
	/// True if all fields are `None`.
	pub fn is_empty(&self) -> bool {
		self.ephemeral_5m_tokens.is_none() && self.ephemeral_1h_tokens.is_none()
	}
}

#[serde_as]
#[skip_serializing_none]
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct PromptTokensDetails {
	/// Anthropic: `cache_creation_input_tokens`.
	/// Tokens used to build the cache (not yet cached). These may incur a small surcharge; subsequent requests benefit via `cached_tokens`.
	#[serde(default, deserialize_with = "crate::support::zero_as_none")]
	pub cache_creation_tokens: Option<i32>,
	/// Breakdown of cache creation tokens by TTL (5m vs 1h).
	/// Only populated when the provider returns TTL-specific breakdown.
	pub cache_creation_details: Option<CacheCreationDetails>,
	/// Anthropic: `cache_read_input_tokens`.
	#[serde(default, deserialize_with = "crate::support::zero_as_none")]
	pub cached_tokens: Option<i32>,
	#[serde(default, deserialize_with = "crate::support::zero_as_none")]
	pub audio_tokens: Option<i32>,
}

impl PromptTokensDetails {
	/// True if all fields are `None`.
	pub fn is_empty(&self) -> bool {
		self.cache_creation_tokens.is_none()
			&& self.cache_creation_details.as_ref().map(|d| d.is_empty()).unwrap_or(true)
			&& self.cached_tokens.is_none()
			&& self.audio_tokens.is_none()
	}
}

#[serde_as]
#[skip_serializing_none]
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct CompletionTokensDetails {
	#[serde(default, deserialize_with = "crate::support::zero_as_none")]
	pub accepted_prediction_tokens: Option<i32>,
	#[serde(default, deserialize_with = "crate::support::zero_as_none")]
	pub rejected_prediction_tokens: Option<i32>,
	#[serde(default, deserialize_with = "crate::support::zero_as_none")]
	pub reasoning_tokens: Option<i32>,
	#[serde(default, deserialize_with = "crate::support::zero_as_none")]
	pub audio_tokens: Option<i32>,
}

impl CompletionTokensDetails {
	/// True if all fields are `None`.
	pub fn is_empty(&self) -> bool {
		self.accepted_prediction_tokens.is_none()
			&& self.rejected_prediction_tokens.is_none()
			&& self.reasoning_tokens.is_none()
			&& self.audio_tokens.is_none()
	}
}

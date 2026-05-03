use crate::adapter::AdapterKind;
use crate::chat::ChatRole;
use crate::{ModelIden, resolver, webc};
use derive_more::{Display, From};
use reqwest::StatusCode;
use value_ext::JsonValueExtError;

/// Type alias for boxed errors that are Send + Sync
pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// GenAI main Result type alias (with genai::Error)
pub type Result<T> = core::result::Result<T, Error>;

/// Main GenAI error
#[derive(Debug, From, Display)]
#[allow(missing_docs)]
pub enum Error {
	// -- Chat Input
	#[display("Chat Request has no messages. (for model {model_iden}")]
	ChatReqHasNoMessages { model_iden: ModelIden },

	#[display("Last chat request message is not of Role 'user' (Actual role '{actual_role}') for model '{model_iden}'")]
	LastChatMessageIsNotUser {
		model_iden: ModelIden,
		actual_role: ChatRole,
	},

	#[display("Role '{role}' not supported for model '{model_iden}'")]
	MessageRoleNotSupported { model_iden: ModelIden, role: ChatRole },

	#[display("Content type not supported for model '{model_iden}'.\nCause: {cause}")]
	MessageContentTypeNotSupported { model_iden: ModelIden, cause: &'static str },

	#[display("JSON mode requested but no instruction/prompt provided.")]
	JsonModeWithoutInstruction,

	#[display("Failed to parse verbosity. Actual: '{actual}'")]
	VerbosityParsing { actual: String },

	#[display("Failed to parse reasoning. Actual: '{actual}'")]
	ReasoningParsingError { actual: String },

	#[display("Failed to parse service tier. Actual: '{actual}'")]
	ServiceTierParsing { actual: String },

	#[display("Failed to parse prompt cache retention. Actual: '{actual}'")]
	PromptCacheRetentionParsing { actual: String },

	// -- Chat Output
	#[display("No chat response from model '{model_iden}'")]
	NoChatResponse { model_iden: ModelIden },

	#[display("Invalid JSON response element: {info}")]
	InvalidJsonResponseElement { info: &'static str },

	// -- Auth
	#[display("Model '{model_iden}' requires an API key.")]
	RequiresApiKey { model_iden: ModelIden },

	#[display("No authentication resolver found for model '{model_iden}'.")]
	NoAuthResolver { model_iden: ModelIden },

	#[display("No authentication data available for model '{model_iden}'.")]
	NoAuthData { model_iden: ModelIden },

	// -- ModelMapper
	#[display("Model mapping failed for '{model_iden}'.\nCause: {cause}")]
	ModelMapperFailed {
		model_iden: ModelIden,
		cause: resolver::Error,
	},

	// -- Web Call error
	#[display("Web call failed for adapter '{adapter_kind}'.\nCause: {webc_error}")]
	WebAdapterCall {
		adapter_kind: AdapterKind,
		webc_error: webc::Error,
	},

	#[display("Web call failed for model '{model_iden}'.\nCause: {webc_error}")]
	WebModelCall {
		model_iden: ModelIden,
		webc_error: webc::Error,
	},

	#[display(
		"Error while generating a ChatResponse from a ChatRequest. (for Model: '{model_iden}')
Request Payload:\n{request_payload:#}
Response Body:\n{response_body:#}
Cause:\n{cause}
"
	)]
	ChatResponseGeneration {
		model_iden: ModelIden,
		request_payload: Box<serde_json::Value>,
		/// Require ChatOptions::default().with_capture_raw_body(true); otherwise "null"
		response_body: Box<serde_json::Value>,
		cause: String,
	},

	#[display("Error event in stream for model '{model_iden}'. Body: {body}")]
	ChatResponse {
		model_iden: ModelIden,
		body: serde_json::Value,
	},

	// -- Chat Stream
	#[display("Failed to parse stream data for model '{model_iden}'.\nCause: {serde_error}")]
	StreamParse {
		model_iden: ModelIden,
		serde_error: serde_json::Error,
	},

	#[display("Web stream error for model '{model_iden}'.\nCause: {cause}")]
	WebStream {
		model_iden: ModelIden,
		cause: String,
		error: BoxError,
	},

	#[display("HTTP error.\nStatus: {status} {canonical_reason}\nBody: {body}")]
	HttpError {
		status: StatusCode,
		canonical_reason: String,
		body: String,
	},

	// -- Modules
	#[display("Resolver error for model '{model_iden}'.\nCause: {resolver_error}")]
	Resolver {
		model_iden: ModelIden,
		resolver_error: resolver::Error,
	},

	// -- Adapter Support
	#[display("Adapter '{adapter_kind}' does not support feature '{feature}'")]
	AdapterNotSupported { adapter_kind: AdapterKind, feature: String },

	#[display("Internal error: {_0}")]
	Internal(String),

	// -- Externals
	#[display("JSON value extension error: {_0}")]
	#[from]
	JsonValueExt(JsonValueExtError),

	#[display("Serde JSON error: {_0}")]
	#[from]
	SerdeJson(serde_json::Error),
}

// region:    --- Error Boilerplate

// The Display trait is now derived via derive_more::Display
// impl core::fmt::Display for Error {
// 	fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::result::Result<(), core::fmt::Error> {
// 		write!(fmt, "{self:?}")
// 	}
// }

impl std::error::Error for Error {}

// endregion: --- Error Boilerplate

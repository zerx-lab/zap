use crate::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;

/// Binary payload attached to a message (e.g., image or PDF).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Binary {
	/// MIME type, such as "image/png" or "application/pdf".
	pub content_type: String,

	/// Where the bytes come from (base64 or URL).
	pub source: BinarySource,

	/// Optional display name or filename.
	pub name: Option<String>,
}

/// Constructors
impl Binary {
	/// Construct a new Binary value.
	pub fn new(content_type: impl Into<String>, source: BinarySource, name: Option<String>) -> Self {
		Self {
			name,
			content_type: content_type.into(),
			source,
		}
	}

	/// Create a binary from a base64 payload.
	///
	/// - content_type: MIME type (e.g., "image/png", "application/pdf").
	/// - content: base64-encoded bytes.
	/// - name: optional display name or filename.
	pub fn from_base64(content_type: impl Into<String>, content: impl Into<Arc<str>>, name: Option<String>) -> Binary {
		Binary {
			name,
			content_type: content_type.into(),
			source: BinarySource::Base64(content.into()),
		}
	}

	/// Create a binary referencing a URL.
	///
	/// Note: Only some providers accept URL-based inputs.
	pub fn from_url(content_type: impl Into<String>, url: impl Into<String>, name: Option<String>) -> Binary {
		Binary {
			name,
			content_type: content_type.into(),
			source: BinarySource::Url(url.into()),
		}
	}

	/// Create a binary from a file path.
	///
	/// Reads the file, determines the MIME type from the file extension,
	/// and base64-encodes the content.
	///
	/// - file_path: Path to the file to read.
	///
	/// Returns an error if the file cannot be read.
	pub fn from_file(file_path: impl AsRef<Path>) -> Result<Binary> {
		let file_path = file_path.as_ref();

		// Read the file content
		let content = std::fs::read(file_path)
			.map_err(|e| crate::Error::Internal(format!("Failed to read file '{}': {}", file_path.display(), e)))?;

		// Determine MIME type from extension
		let content_type = mime_guess::from_path(file_path).first_or_octet_stream().to_string();

		// Base64 encode
		let b64_content = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &content);

		// Extract file name
		let name = file_path.file_name().and_then(|n| n.to_str()).map(String::from);

		Ok(Binary {
			name,
			content_type,
			source: BinarySource::Base64(b64_content.into()),
		})
	}
}

/// is_.., into_.. Accessors
impl Binary {
	/// Returns true if this binary is an image (content_type starts with "image/").
	pub fn is_image(&self) -> bool {
		self.content_type.trim().to_ascii_lowercase().starts_with("image/")
	}

	/// Returns true if this binary is an audio file (content_type starts with "audio/").
	pub fn is_audio(&self) -> bool {
		self.content_type.trim().to_ascii_lowercase().starts_with("audio/")
	}

	/// Returns true if this binary is a PDF (content_type equals "application/pdf").
	pub fn is_pdf(&self) -> bool {
		self.content_type.trim().eq_ignore_ascii_case("application/pdf")
	}

	/// Generate the web or data url from this binary
	pub fn into_url(self) -> String {
		match self.source {
			BinarySource::Url(url) => url,
			BinarySource::Base64(b64_content) => {
				// NOTE: Openai does not support filename in the URL.
				// let filename_section: Cow<str> = if let Some(name) = self.name {
				// 	let name = normalize_name(&name);
				// 	format!("filename={name};").into()
				// } else {
				// 	"".into()
				// };
				let filename_section = "";

				format!("data:{};{filename_section}base64,{b64_content}", self.content_type)
			}
		}
	}
}

/// Computed assessors
impl Binary {
	/// Returns an approximate in-memory size of this `Binary`, in bytes,
	/// computed as the sum of the UTF-8 lengths of:
	/// - `content_type`
	/// - `name` (if any)
	/// - the underlying URL or base64 string in `source`.
	///
	/// This does **not** return the decoded byte length of the file.
	/// This does **not** return the size of the URL content
	pub fn size(&self) -> usize {
		let mut size = self.content_type.len();
		size += self.name.as_ref().map(|n| n.len()).unwrap_or_default();
		size += match &self.source {
			BinarySource::Url(url) => url.len(),
			BinarySource::Base64(data) => data.len(),
		};
		size
	}
}

// region:    --- BinarySource

/// Origin of a binary payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BinarySource {
	/// For models/services that support URL as input
	/// NOTE: Few AI services support this.
	Url(String),

	/// The base64 string of the image
	///
	/// NOTE: Here we use an `Arc<str>` to avoid cloning large amounts of data when cloning a ChatRequest.
	///       The overhead is minimal compared to cloning relatively large data.
	///       The downside is that it will be an Arc even when used only once, but for this particular data type, the net benefit is positive.
	Base64(Arc<str>),
}

// endregion: --- BinarySource

// No `Local` location; this would require handling errors like "file not found" etc.
// Such a file can be easily provided by the user as Base64, and we can implement a convenient
// TryFrom<File> to Base64 version. All LLMs accept local images only as Base64.

// region:    --- Support

#[allow(unused)]
fn normalize_name(input: &str) -> String {
	input
		.chars()
		.map(|c| {
			match c {
				// allowed
				'a'..='z' | 'A'..='Z' | '0'..='9' | '.' | '_' | '-' | '(' | ')' => c,

				// everything else becomes '-'
				_ => '-',
			}
		})
		.collect()
}

// endregion: --- Support

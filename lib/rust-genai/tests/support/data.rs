//! Data files / constants for tests

use crate::support::TestResult;
use base64::Engine;
use base64::engine::general_purpose;
use simple_fs::SPath;
use std::fs::File;
use std::io::Read;

pub const IMAGE_URL_JPG_DUCK: &str = "https://aipack.ai/images/test-duck.jpg";
pub const AUDIO_TEST_FILE_PATH: &str = "./tests/data/phrase_neil_armstrong.wav";
pub const TEST_IMAGE_FILE_PATH: &str = "./tests/data/duck-small.jpg";

/// Get the base64 of the image above (but resized/lower to fit 5kb)
pub fn get_b64_duck() -> TestResult<String> {
	get_b64_file(TEST_IMAGE_FILE_PATH)
}

pub fn has_audio_file() -> bool {
	SPath::new(AUDIO_TEST_FILE_PATH).exists()
}

pub fn get_b64_audio() -> TestResult<String> {
	get_b64_file(AUDIO_TEST_FILE_PATH)
}

pub fn get_b64_pdf() -> TestResult<String> {
	get_b64_file("./tests/data/small.pdf")
}

pub fn get_b64_file(file_path: &str) -> TestResult<String> {
	// Open the file and read its contents into a buffer
	let mut file = File::open(file_path)?;
	let mut buffer = Vec::new();
	file.read_to_end(&mut buffer)?;

	// Use the general-purpose Base64 engine for encoding
	let base64_encoded = general_purpose::STANDARD.encode(&buffer);

	Ok(base64_encoded)
}

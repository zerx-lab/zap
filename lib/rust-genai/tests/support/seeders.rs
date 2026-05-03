use super::TestResult;
use genai::chat::{BinarySource, ChatMessage, ChatRequest, ContentPart, Tool};
use serde_json::json;
use simple_fs::{list_files, read_to_string};

pub fn get_big_content() -> TestResult<String> {
	// resolver/... about 13567 (len)
	// it has to be that to have cache activate
	let files = list_files("./src", Some(&["./src/resolver/**/*.rs"]), None)?;

	let mut buff = String::new();

	for file in files {
		let content = read_to_string(&file)?;
		buff.push_str(&format!("\n\n````// file: {file}\n{content}\n````\n"));
	}

	Ok(buff)
}

pub fn seed_chat_req_simple() -> ChatRequest {
	ChatRequest::new(vec![
		// -- Messages (deactivate to see the differences)
		ChatMessage::system("Answer in one sentence"),
		ChatMessage::user("Why is the sky blue?"),
	])
}

pub fn seed_chat_req_tool_simple() -> ChatRequest {
	ChatRequest::new(vec![
		// -- Messages (deactivate to see the differences)
		ChatMessage::user("What is the temperature in C and weather, in Paris, France"),
	])
	.append_tool(Tool::new("get_weather").with_schema(json!({
		"type": "object",
		"properties": {
			"city": {
					"type": "string",
					"description": "The city name"
			},
			"country": {
					"type": "string",
					"description": "The most likely country of this city name"
			},
			"unit": {
					"type": "string",
					"enum": ["C", "F"],
					"description": "The temperature unit of the country. C for Celsius, and F for Fahrenheit"
			}
		},
		"required": ["city", "country", "unit"],
	})))
}

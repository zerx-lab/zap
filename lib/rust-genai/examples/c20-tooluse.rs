use genai::Client;
use genai::chat::printer::print_chat_stream;
use genai::chat::{ChatMessage, ChatRequest, Tool, ToolResponse};
use serde_json::json;
use tracing_subscriber::EnvFilter;

const MODEL: &str = "gpt-4o-mini"; // or "gemini-2.0-flash" or other model supporting tool calls

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	tracing_subscriber::fmt()
		.with_env_filter(EnvFilter::new("genai=debug"))
		// .with_max_level(tracing::Level::DEBUG) // To enable all sub-library tracing
		.init();

	let client = Client::default();

	// 1. Define a tool for getting weather information
	let weather_tool = Tool::new("get_weather")
		.with_description("Get the current weather for a location")
		.with_schema(json!({
			"type": "object",
			"properties": {
				"city": {
					"type": "string",
					"description": "The city name"
				},
				"country": {
					"type": "string",
					"description": "The country of the city"
				},
				"unit": {
					"type": "string",
					"enum": ["C", "F"],
					"description": "Temperature unit (C for Celsius, F for Fahrenheit)"
				}
			},
			"required": ["city", "country", "unit"]
		}));

	// 2. Create initial chat request with the user query and the tool
	let chat_req = ChatRequest::new(vec![ChatMessage::user("What's the weather like in Tokyo, Japan?")])
		.with_tools(vec![weather_tool]);

	// 3. Make the initial call to get the function call
	println!("--- Getting function call from model");
	let chat_res = client.exec_chat(MODEL, chat_req.clone(), None).await?;

	// 4. Extract the tool calls from the response
	let tool_calls = chat_res.into_tool_calls();

	if tool_calls.is_empty() {
		return Err("Expected tool calls in the response".into());
	}

	println!("--- Tool calls received:");
	for tool_call in &tool_calls {
		println!("Function: {}", tool_call.fn_name);
		println!("Arguments: {}", tool_call.fn_arguments);
	}

	// 5. Simulate executing the function and getting a result
	// In a real app, you would call your actual API or service here
	let first_tool_call = &tool_calls[0];
	let tool_response = ToolResponse::new(
		first_tool_call.call_id.clone(),
		json!({
			"temperature": 22.5,
			"condition": "Sunny",
			"humidity": 65
		})
		.to_string(),
	);

	// 6. Add both the tool calls from the model and your tool response to the chat history
	let chat_req = chat_req.append_message(tool_calls).append_message(tool_response);

	// 7. Get the final response from the model with the function results
	println!("\n--- Getting final response with function results");
	let chat_res = client.exec_chat_stream(MODEL, chat_req, None).await?;

	println!("\n--- Final response:");
	print_chat_stream(chat_res, None).await?;

	Ok(())
}

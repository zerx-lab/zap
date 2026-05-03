use futures::StreamExt;
use genai::Client;
use genai::chat::printer::{PrintChatStreamOptions, print_chat_stream};
use genai::chat::{ChatMessage, ChatOptions, ChatRequest, Tool, ToolResponse};
use genai::chat::{ChatStreamEvent, ToolCall};
use serde_json::json;
use tracing_subscriber::EnvFilter;

// const MODEL: &str = "gemini-2.0-flash";
// const MODEL: &str = "deepseek-chat";
const MODEL: &str = "gemini-3-pro-preview";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	tracing_subscriber::fmt()
		.with_env_filter(EnvFilter::new("genai=debug"))
		// .with_max_level(tracing::Level::DEBUG) // To enable all sub-library tracing
		.init();

	let client = Client::default();

	println!("--- Model: {MODEL}");

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
	let chat_req = ChatRequest::new(vec![ChatMessage::user("What's the weather like in Shenzhen, China?")])
		.with_tools(vec![weather_tool]);

	// 3. Set options to capture tool calls in the streaming response
	let chat_options = ChatOptions::default().with_capture_tool_calls(true);
	let print_options = PrintChatStreamOptions::from_print_events(false);

	// 4. Make the streaming call and handle the events
	let mut chat_stream = client.exec_chat_stream(MODEL, chat_req.clone(), Some(&chat_options)).await?;

	let mut tool_calls: Vec<ToolCall> = [].to_vec();
	let mut captured_thoughts: Option<Vec<String>> = None;

	// print_chat_stream(chat_res, Some(&print_options)).await?;
	println!("--- Streaming response with tool calls");
	while let Some(result) = chat_stream.stream.next().await {
		match result? {
			ChatStreamEvent::Start => {
				println!("Stream started");
			}
			ChatStreamEvent::Chunk(chunk) => {
				print!("{}", chunk.content);
			}
			ChatStreamEvent::ToolCallChunk(chunk) => {
				println!("  ToolCallChunk: {:?}", chunk.tool_call);
			}
			ChatStreamEvent::ReasoningChunk(chunk) => {
				println!("  ReasoningChunk: {:?}", chunk.content);
			}
			ChatStreamEvent::ThoughtSignatureChunk(chunk) => {
				println!("  ThoughtSignatureChunk: {:?}", chunk.content);
			}
			ChatStreamEvent::End(end) => {
				println!("\nStream ended");

				// Check if we captured any tool calls
				// Note: captured_into_tool_calls consumes self, so we can't use end afterwards.
				// We should access captured_content directly or use references if possible,
				// but StreamEnd getters often consume or clone.
				// Let's access captured_content directly since we need both tool calls and thoughts.

				if let Some(content) = end.captured_content {
					// Let's refactor to avoid ownership issues.
					// We have `content` (MessageContent).
					// We want `tool_calls` (Vec<ToolCall>) and `thoughts` (Vec<String>).

					// We can iterate and split.
					let parts = content.into_parts();
					let mut extracted_tool_calls = Vec::new();
					let mut extracted_thoughts = Vec::new();

					for part in parts {
						match part {
							genai::chat::ContentPart::ToolCall(tc) => extracted_tool_calls.push(tc),
							genai::chat::ContentPart::ThoughtSignature(t) => extracted_thoughts.push(t),
							_ => {}
						}
					}

					if !extracted_tool_calls.is_empty() {
						println!("\nCaptured Tool Calls:");
						for tool_call in &extracted_tool_calls {
							println!("- Function: {}", tool_call.fn_name);
							println!("  Arguments: {}", tool_call.fn_arguments);
						}
						tool_calls = extracted_tool_calls;
					}

					if !extracted_thoughts.is_empty() {
						captured_thoughts = Some(extracted_thoughts);
					}
				}
			}
		}
	}

	// 5. Now demonstrate how to handle the tool call and continue the conversation
	println!("\n--- Demonstrating full tool call workflow");
	if tool_calls.is_empty() {
		println!("No tool calls captured, cannot continue.");
		return Ok(());
	}
	// Simulate executing the function
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

	// Add both the tool calls and response to chat history
	// Note: For Gemini 3, we MUST include the thoughtSignature in the history if it was generated.
	let mut assistant_msg = ChatMessage::from(tool_calls);
	if let Some(thoughts) = captured_thoughts {
		// We need to insert the thought at the beginning.
		// MessageContent wraps Vec<ContentPart>, but doesn't expose insert.
		// We can convert to Vec, insert, and convert back.
		let mut parts = assistant_msg.content.into_parts();
		for thought in thoughts.into_iter().rev() {
			parts.insert(0, genai::chat::ContentPart::ThoughtSignature(thought));
		}
		assistant_msg.content = genai::chat::MessageContent::from_parts(parts);
	}
	let chat_req = chat_req.append_message(assistant_msg).append_message(tool_response);

	// Get final streaming response
	let chat_options = ChatOptions::default();
	let chat_res = client.exec_chat_stream(MODEL, chat_req, Some(&chat_options)).await?;

	print_chat_stream(chat_res, Some(&print_options)).await?;

	Ok(())
}

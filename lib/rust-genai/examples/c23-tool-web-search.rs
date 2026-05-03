#![allow(unused)]

use genai::{
	ClientConfig,
	chat::{ChatRequest, Tool, ToolName, WebSearchConfig},
};
use value_ext::JsonValueExt;

// const MODEL: &str = "gemini-3-flash-preview";
// const MODEL: &str = "claude-haiku-4-5";
const MODEL: &str = "gpt-5.1-codex-mini";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let question = "Give me the latest 3 news about Rust Programming (not older than 3 days).
For each new, have it in a markdown section
## _date_time_ _concise_summary_,
3 bullet points, Use - for bullet point lines.
and source link.";

	let options = genai::chat::ChatOptions::default().with_capture_raw_body(true);
	let client = genai::Client::builder()
		.with_config(ClientConfig::default().with_chat_options(options))
		.build();

	// -- Set web search tool (enable one)
	// Manual for google
	// let web_search_tool = Tool::new("googleSearch").with_config(serde_json::json!({})); // manual

	// Manual for Anthropic
	// let web_search_tool = Tool::new("web_search").with_config(serde_json::json!({
	// 	"max_uses": 5,
	// 	"allowed_domains": ["rust-lang.org", "docs.rs", "this-week-in-rust.org"]
	// }));

	// Normalized
	// let web_search_tool = Tool::new(ToolName::WebSearch); // no config
	let web_search_tool = Tool::new(ToolName::WebSearch).with_config(WebSearchConfig::default().with_max_uses(3)); // use normalized WebSearch

	// -- Prep and Exec query
	let chat_req = ChatRequest::from_user(question).append_tool(web_search_tool);
	println!("\n=== AI Question:\n{question}");
	let res = client.exec_chat(MODEL, chat_req, None).await;

	let res = match res {
		Err(err) => {
			println!("{err}");
			return Ok(());
		}
		Ok(res) => res,
	};

	if let Some(raw) = res.captured_raw_body {
		println!("\n=== Raw Response:\n{}", raw.x_pretty()?);
	}

	// -- Check / Print
	let res_txt = res.content.into_joined_texts().ok_or("Should have result")?;
	assert!(res_txt.contains("Rust"), "should contains 'Rust'");

	println!("\n=== AI Response:\n{res_txt}");

	Ok(())
}

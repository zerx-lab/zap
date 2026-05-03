use anyhow::{Context, Result};
use clap::Parser;

mod client;
mod config;
mod tools;

use client::{ChatMessage, TurnResult};

#[derive(Parser)]
#[command(name = "warp-ai", about = "AI sysadmin agent that executes tasks via OpenAI-compatible APIs")]
struct Args {
    /// Path to file containing the user prompt
    #[arg(long = "prompt-file")]
    prompt_file: String,

    /// Path to file containing the system prompt
    #[arg(long = "system-prompt-file")]
    system_prompt_file: String,

    /// Model to use (overrides config/env)
    #[arg(long = "model")]
    model: Option<String>,

    /// API base URL (overrides config/env)
    #[arg(long = "base-url")]
    base_url: Option<String>,

    /// API key (overrides config/env)
    #[arg(long = "api-key")]
    api_key: Option<String>,

    /// Maximum number of agent loop iterations (default: 50)
    #[arg(long = "max-turns", default_value = "50")]
    max_turns: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let cfg = config::Config::resolve(
        args.api_key.as_deref(),
        args.base_url.as_deref(),
        args.model.as_deref(),
    )?;

    let prompt = std::fs::read_to_string(&args.prompt_file)
        .with_context(|| format!("Failed to read prompt file: {}", args.prompt_file))?;

    let system_prompt = std::fs::read_to_string(&args.system_prompt_file)
        .with_context(|| format!("Failed to read system prompt file: {}", args.system_prompt_file))?;

    let mut messages = vec![ChatMessage::system(&system_prompt), ChatMessage::user(&prompt)];

    for _ in 0..args.max_turns {
        let result = client::chat_turn(&cfg, messages.clone(), true).await?;

        match result {
            TurnResult::Done(_text) => {
                // Final answer streamed to stdout already.
                return Ok(());
            }
            TurnResult::ToolCalls { text, calls } => {
                // Add the assistant message with tool calls to history.
                let tool_calls_json: Vec<serde_json::Value> = calls
                    .iter()
                    .map(|c| {
                        serde_json::json!({
                            "id": c.id,
                            "type": "function",
                            "function": {
                                "name": c.name,
                                "arguments": c.arguments
                            }
                        })
                    })
                    .collect();

                messages.push(ChatMessage::assistant(
                    if text.is_empty() { None } else { Some(text) },
                    Some(tool_calls_json),
                ));

                // Execute each tool and add results.
                for call in &calls {
                    eprintln!("[{}] {}", call.name, summarize_args(&call.name, &call.arguments));
                    let result = tools::execute_tool(call);
                    eprintln!(
                        "[{}] {}",
                        if result.content.starts_with("Error:") { "FAIL" } else { "OK" },
                        truncate(&result.content, 200)
                    );
                    messages.push(ChatMessage::tool_result(result.tool_call_id, result.content));
                }
            }
        }
    }

    // Hit max turns — ask for a final summary without tools.
    messages.push(ChatMessage::user(
        "Please provide a summary of what was done. Do not call any more tools.",
    ));
    let _ = client::chat_turn(&cfg, messages, false).await?;
    Ok(())
}

/// Brief summary of tool call arguments for display.
fn summarize_args(name: &str, arguments: &str) -> String {
    match name {
        "run_command" => {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(arguments) {
                v.get("command")
                    .and_then(|c| c.as_str())
                    .map(|s| truncate(s, 120).to_owned())
                    .unwrap_or_else(|| arguments.to_owned())
            } else {
                truncate(arguments, 120).to_owned()
            }
        }
        "read_file" => extract_path(arguments).unwrap_or_else(|| arguments.to_owned()),
        "write_file" => extract_path(arguments).unwrap_or_else(|| arguments.to_owned()),
        "list_directory" => extract_path(arguments).unwrap_or_else(|| ".".to_owned()),
        _ => truncate(arguments, 120).to_owned(),
    }
}

fn extract_path(arguments: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(arguments)
        .ok()
        .and_then(|v| v.get("path").and_then(|p| p.as_str()).map(|s| s.to_owned()))
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max {
        s
    } else {
        // Find a valid char boundary to avoid panicking on multi-byte UTF-8.
        let mut end = max;
        while !s.is_char_boundary(end) && end > 0 {
            end -= 1;
        }
        &s[..end]
    }
}

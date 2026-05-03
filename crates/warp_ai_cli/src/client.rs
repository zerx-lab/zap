use anyhow::{Context, Result};
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::io::Write;

use crate::config::Config;
use crate::tools::ToolCall;

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<serde_json::Value>>,
    stream: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl ChatMessage {
    pub fn system(content: &str) -> Self {
        Self {
            role: "system".to_owned(),
            content: Some(content.to_owned()),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn user(content: &str) -> Self {
        Self {
            role: "user".to_owned(),
            content: Some(content.to_owned()),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn assistant(content: Option<String>, tool_calls: Option<Vec<serde_json::Value>>) -> Self {
        Self {
            role: "assistant".to_owned(),
            content,
            tool_calls,
            tool_call_id: None,
        }
    }

    pub fn tool_result(tool_call_id: String, content: String) -> Self {
        Self {
            role: "tool".to_owned(),
            content: Some(content),
            tool_calls: None,
            tool_call_id: Some(tool_call_id),
        }
    }
}

/// The result of a single turn in the agent loop.
pub enum TurnResult {
    /// The LLM produced final text output for the user.
    Done(String),
    /// The LLM wants to call tools.
    ToolCalls { text: String, calls: Vec<ToolCall> },
}

/// Send a chat request with optional tools, stream the response, and return
/// either final text or parsed tool calls.
pub async fn chat_turn(
    config: &Config,
    messages: Vec<ChatMessage>,
    include_tools: bool,
) -> Result<TurnResult> {
    let api_key = config
        .api_key
        .as_ref()
        .context("No API key configured. Set WARP_AI_API_KEY or add api_key to ~/.config/warp-ai/config.json")?;

    let url = format!("{}/chat/completions", config.base_url.trim_end_matches('/'));

    let tools = if include_tools {
        Some(crate::tools::tool_definitions())
    } else {
        None
    };

    let request = ChatRequest {
        model: config.model.clone(),
        messages,
        tools,
        stream: true,
    };

    let client = Client::new();
    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await
        .with_context(|| format!("Failed to connect to {}", config.base_url))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("API request failed ({status}): {body}");
    }

    // Parse the SSE stream, collecting text deltas and tool call chunks.
    let mut text_content = String::new();
    let mut tool_call_map: std::collections::HashMap<usize, PartialToolCall> =
        std::collections::HashMap::new();
    let mut buffer = String::new();
    let mut stream = response.bytes_stream();
    let mut stdout = std::io::stdout();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context("Error reading stream")?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(line_end) = buffer.find('\n') {
            let line = buffer[..line_end].trim().to_owned();
            buffer = buffer[line_end + 1..].to_owned();

            if let Some(data) = line.strip_prefix("data: ") {
                if data == "[DONE]" {
                    // Stream finished — assemble results.
                    if tool_call_map.is_empty() {
                        stdout.write_all(b"\n")?;
                        stdout.flush()?;
                        return Ok(TurnResult::Done(text_content));
                    } else {
                        let calls: Vec<ToolCall> = tool_call_map
                            .into_iter()
                            .map(|(_, partial)| ToolCall {
                                id: partial.id,
                                name: partial.name,
                                arguments: partial.arguments,
                            })
                            .collect();
                        return Ok(TurnResult::ToolCalls {
                            text: text_content,
                            calls,
                        });
                    }
                }

                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data) {
                    if let Some(choices) = parsed.get("choices").and_then(|c| c.as_array()) {
                        for choice in choices {
                            if let Some(delta) = choice.get("delta") {
                                // Text content
                                if let Some(content) =
                                    delta.get("content").and_then(|c| c.as_str())
                                {
                                    text_content.push_str(content);
                                    stdout.write_all(content.as_bytes())?;
                                    stdout.flush()?;
                                }

                                // Tool calls
                                if let Some(tool_calls) =
                                    delta.get("tool_calls").and_then(|tc| tc.as_array())
                                {
                                    for tc in tool_calls {
                                        let index = tc
                                            .get("index")
                                            .and_then(|i| i.as_u64())
                                            .unwrap_or(0) as usize;

                                        let entry =
                                            tool_call_map.entry(index).or_default();

                                        if let Some(id) =
                                            tc.get("id").and_then(|i| i.as_str())
                                        {
                                            entry.id = id.to_owned();
                                        }
                                        if let Some(func) = tc.get("function") {
                                            if let Some(name) =
                                                func.get("name").and_then(|n| n.as_str())
                                            {
                                                entry.name = name.to_owned();
                                            }
                                            if let Some(args) = func
                                                .get("arguments")
                                                .and_then(|a| a.as_str())
                                            {
                                                entry.arguments.push_str(args);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Stream ended without [DONE] — return what we have.
    if tool_call_map.is_empty() {
        stdout.write_all(b"\n")?;
        stdout.flush()?;
        Ok(TurnResult::Done(text_content))
    } else {
        let calls: Vec<ToolCall> = tool_call_map
            .into_iter()
            .map(|(_, partial)| ToolCall {
                id: partial.id,
                name: partial.name,
                arguments: partial.arguments,
            })
            .collect();
        Ok(TurnResult::ToolCalls {
            text: text_content,
            calls,
        })
    }
}

/// Partial tool call being assembled from streaming chunks.
#[derive(Default)]
struct PartialToolCall {
    id: String,
    name: String,
    arguments: String,
}

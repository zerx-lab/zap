//! `websearch` BYOP 工具 descriptor。
//!
//! 实际 HTTP 执行在 `web_runtime::run_websearch`(走 Exa MCP 端点)。本 descriptor
//! 提供给 genai SDK 用于把 tool 描述发给上游 LLM(name + description + JSON Schema)。
//!
//! ## 不走 protobuf executor
//!
//! `from_args` 永远返回 `Err`,`result_to_json` 永远返回 `None`。`chat_stream::
//! parse_incoming_tool_call` 之前按 name 命中后直接调 `web_runtime`。
//!
//! 参数 schema 与 opencode `websearch.ts:7-22` 对齐。

use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use warp_multi_agent_api as api;

use super::OpenAiTool;

pub const TOOL_NAME: &str = "websearch";

fn parameters() -> Value {
    json!({
        "type": "object",
        "properties": {
            "query": {
                "type": "string",
                "description": "Web search query."
            },
            "numResults": {
                "type": "integer",
                "description": "Number of search results to return (default 8).",
                "minimum": 1,
                "maximum": 50
            },
            "livecrawl": {
                "type": "string",
                "enum": ["fallback", "preferred"],
                "description": "Live-crawl mode. 'fallback' (default): use cached content, live-crawl as backup. 'preferred': always live-crawl."
            },
            "type": {
                "type": "string",
                "enum": ["auto", "fast", "deep"],
                "description": "Search type. 'auto' (default, balanced), 'fast' (quick), 'deep' (comprehensive)."
            },
            "contextMaxCharacters": {
                "type": "integer",
                "description": "Cap for the LLM-optimized context string."
            }
        },
        "required": ["query"],
        "additionalProperties": false
    })
}

fn from_args(_args: &str) -> Result<api::message::tool_call::Tool> {
    Err(anyhow!(
        "websearch is intercepted by chat_stream BYOP web tool dispatcher; \
         from_args should never be called"
    ))
}

fn result_to_json(_result: &api::message::tool_call_result::Result) -> Option<Value> {
    None
}

pub static WEBSEARCH: OpenAiTool = OpenAiTool {
    name: TOOL_NAME,
    description: include_str!("../prompts/tool_descriptions/websearch.md"),
    parameters,
    from_args,
    result_to_json,
};

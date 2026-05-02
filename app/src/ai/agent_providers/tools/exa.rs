//! Exa MCP wire 协议(纯逻辑,不含 HTTP I/O)。
//!
//! 镜像 opencode `packages/opencode/src/tool/mcp-exa.ts`:
//! - 端点:`https://mcp.exa.ai/mcp`(默认匿名)或带 `?exaApiKey=...`
//! - 协议:JSON-RPC 2.0 POST,`Accept: application/json, text/event-stream`
//! - 响应:SSE,逐行扫描 `data: ` 前缀,解析 `result.content[0].text`
//!
//! 所有 HTTP 调用在 `web_runtime.rs` 里;本模块只负责构造请求 body 和解析响应字符串。

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const EXA_BASE_URL: &str = "https://mcp.exa.ai/mcp";
pub const SEARCH_TOOL_NAME: &str = "web_search_exa";

/// 拼出最终的 Exa 端点 URL。`api_key=Some` 时把 key 拼到 querystring(percent-encode)。
pub fn endpoint_url(api_key: Option<&str>) -> String {
    match api_key {
        Some(k) if !k.trim().is_empty() => {
            let encoded: String =
                url::form_urlencoded::byte_serialize(k.as_bytes()).collect();
            format!("{EXA_BASE_URL}?exaApiKey={encoded}")
        }
        _ => EXA_BASE_URL.to_owned(),
    }
}

/// `web_search_exa` 入参(直接发给 Exa)。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SearchArgs {
    pub query: String,
    /// "auto" / "fast" / "deep"
    #[serde(rename = "type")]
    pub search_type: String,
    #[serde(rename = "numResults")]
    pub num_results: u32,
    /// "fallback" / "preferred"
    pub livecrawl: String,
    #[serde(rename = "contextMaxCharacters", skip_serializing_if = "Option::is_none")]
    pub context_max_characters: Option<u32>,
}

impl SearchArgs {
    /// opencode 默认值(websearch.ts:54-58)。
    pub fn with_defaults(query: String) -> Self {
        Self {
            query,
            search_type: "auto".to_owned(),
            num_results: 8,
            livecrawl: "fallback".to_owned(),
            context_max_characters: None,
        }
    }
}

/// JSON-RPC 2.0 `tools/call` 请求 body。`id` 固定为 1(单次调用,不需要 id 区分)。
pub fn build_request_body(tool_name: &str, args: &SearchArgs) -> Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": tool_name,
            "arguments": args,
        }
    })
}

/// 解析 Exa SSE 响应:扫描每一行,首个 `data: ` 行 JSON parse 后取 `result.content[0].text`。
///
/// 返回 `Ok(Some(text))` = 找到内容;`Ok(None)` = 没有任何 content(空结果);
/// `Err` = data 行存在但 JSON 解析失败 / 结构不符。
pub fn parse_sse_body(body: &str) -> Result<Option<String>> {
    let mut last_err: Option<anyhow::Error> = None;
    for line in body.split('\n') {
        let Some(payload) = line.strip_prefix("data: ").or_else(|| line.strip_prefix("data:"))
        else {
            continue;
        };
        let payload = payload.trim();
        if payload.is_empty() {
            continue;
        }
        match serde_json::from_str::<Value>(payload) {
            Ok(v) => {
                if let Some(text) = extract_first_text(&v) {
                    return Ok(Some(text));
                }
                // data: 行解析了但没 content,继续看下一条
            }
            Err(e) => {
                last_err = Some(anyhow!("invalid Exa SSE JSON payload: {e}"));
            }
        }
    }
    if let Some(e) = last_err {
        return Err(e).context("no Exa SSE data line yielded usable content");
    }
    Ok(None)
}

fn extract_first_text(v: &Value) -> Option<String> {
    let content = v.get("result")?.get("content")?.as_array()?;
    let first = content.first()?;
    let text = first.get("text")?.as_str()?;
    Some(text.to_owned())
}

#[cfg(test)]
#[path = "exa_tests.rs"]
mod exa_tests;

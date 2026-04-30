//! `SearchCodebase`:语义搜索代码库。
//!
//! 与 `Grep`(逐字面匹配)、`FileGlobV2`(文件名通配)的区别在于:SearchCodebase
//! 是**语义化**搜索 — 通过 warp 内置的代码索引(embeddings/symbol outline 等),
//! 用自然语言描述就能找到相关代码片段。仅 Local 会话支持。

use anyhow::Result;
use serde::Deserialize;
use serde_json::{json, Value};
use warp_multi_agent_api as api;

use super::OpenAiTool;

#[derive(Debug, Deserialize)]
struct Args {
    query: String,
    #[serde(default)]
    path_filters: Vec<String>,
    /// 可选的代码库绝对路径,缺省用当前工作目录。
    #[serde(default)]
    codebase_path: String,
}

fn parameters() -> Value {
    json!({
        "type": "object",
        "properties": {
            "query": {
                "type": "string",
                "description": "用自然语言描述你想找的代码、概念、模式(例如 \
                                'where do we handle JWT refresh' 或 '找出所有数据库连接初始化逻辑')。"
            },
            "path_filters": {
                "type": "array",
                "description": "可选: 限定子路径(相对 codebase 根)。例如 [\"src/auth/\", \"crates/server/\"]。",
                "items": {"type": "string"},
                "default": []
            },
            "codebase_path": {
                "type": "string",
                "description": "可选: 代码库绝对路径。缺省用当前工作目录。",
                "default": ""
            }
        },
        "required": ["query"],
        "additionalProperties": false
    })
}

fn from_args(args: &str) -> Result<api::message::tool_call::Tool> {
    let parsed: Args = serde_json::from_str(args)?;
    Ok(api::message::tool_call::Tool::SearchCodebase(
        api::message::tool_call::SearchCodebase {
            query: parsed.query,
            path_filters: parsed.path_filters,
            codebase_path: parsed.codebase_path,
        },
    ))
}

fn result_to_json(result: &api::message::tool_call_result::Result) -> Option<Value> {
    use api::message::tool_call_result::Result as R;
    use api::search_codebase_result::Result as SearchR;
    let r = match result {
        R::SearchCodebase(r) => r,
        _ => return None,
    };
    let value = match &r.result {
        Some(SearchR::Success(s)) => {
            let files: Vec<Value> = s
                .files
                .iter()
                .map(|f| {
                    json!({
                        "path": f.file_path,
                        "content": f.content,
                    })
                })
                .collect();
            json!({ "status": "ok", "files": files })
        }
        Some(SearchR::Error(e)) => json!({ "status": "error", "message": e.message }),
        None => json!({ "status": "cancelled" }),
    };
    Some(value)
}

pub static SEARCH_CODEBASE: OpenAiTool = OpenAiTool {
    name: "search_codebase",
    description: include_str!("../prompts/tool_descriptions/search_codebase.md"),
    parameters,
    from_args,
    result_to_json,
};

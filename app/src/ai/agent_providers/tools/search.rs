//! 搜索类工具:`Grep`(逐行匹配)+ `FileGlobV2`(文件名通配)。

use anyhow::Result;
use serde::Deserialize;
use serde_json::{json, Value};
use warp_multi_agent_api as api;

use super::OpenAiTool;

// ---------------------------------------------------------------------------
// Grep
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct GrepArgs {
    queries: Vec<String>,
    #[serde(default)]
    path: String,
}

fn grep_parameters() -> Value {
    json!({
        "type": "object",
        "properties": {
            "queries": {
                "type": "array",
                "description": "要搜索的关键字/正则模式列表(每个元素是一个独立查询,任一命中都算匹配)。",
                "items": {"type": "string"}
            },
            "path": {
                "type": "string",
                "description": "搜索范围的相对路径(文件或目录)。空字符串或 \".\" 表示当前工作目录。",
                "default": "."
            }
        },
        "required": ["queries"],
        "additionalProperties": false
    })
}

fn grep_from_args(args: &str) -> Result<api::message::tool_call::Tool> {
    let parsed: GrepArgs = serde_json::from_str(args)?;
    Ok(api::message::tool_call::Tool::Grep(
        api::message::tool_call::Grep {
            queries: parsed.queries,
            path: if parsed.path.is_empty() {
                ".".to_owned()
            } else {
                parsed.path
            },
        },
    ))
}

fn grep_result_to_json(result: &api::message::tool_call_result::Result) -> Option<Value> {
    use api::grep_result::Result as GR;
    use api::message::tool_call_result::Result as R;
    let r = match result {
        R::Grep(r) => r,
        _ => return None,
    };
    let value = match &r.result {
        Some(GR::Success(s)) => {
            let files: Vec<Value> = s
                .matched_files
                .iter()
                .map(|f| {
                    json!({
                        "path": f.file_path,
                        "lines": f.matched_lines.iter().map(|l| l.line_number).collect::<Vec<_>>(),
                    })
                })
                .collect();
            json!({ "status": "ok", "files": files })
        }
        Some(GR::Error(e)) => json!({ "status": "error", "message": e.message }),
        None => json!({ "status": "cancelled" }),
    };
    Some(value)
}

pub static GREP: OpenAiTool = OpenAiTool {
    name: "grep",
    description: "在指定路径(文件或目录)下**逐行**搜索关键字或正则,返回匹配的文件路径 + 行号列表。\
                  queries 数组里任一模式命中即算匹配(OR 语义)。区分大小写。仅匹配单行,跨行模式不支持。\
                  比 run_shell_command 调用 grep/rg 更快更安全(只读、自动通过审批,无需 shell 解析)。\
                  优先输入**裸标识符**(如 'InProgressQuote'),避免复杂正则 — `.*\\d+` 等贪婪模式跨 token 通常 0 命中。",
    parameters: grep_parameters,
    from_args: grep_from_args,
    result_to_json: grep_result_to_json,
};

// ---------------------------------------------------------------------------
// FileGlobV2
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct GlobArgs {
    patterns: Vec<String>,
    #[serde(default)]
    search_dir: String,
    #[serde(default)]
    limit: i32,
}

fn glob_parameters() -> Value {
    json!({
        "type": "object",
        "properties": {
            "patterns": {
                "type": "array",
                "description": "文件名通配符列表(支持 ?、*、[…])。例如 [\"**/*.rs\", \"src/**/*.toml\"]。",
                "items": {"type": "string"}
            },
            "search_dir": {
                "type": "string",
                "description": "搜索目录的相对路径,空表示当前工作目录。",
                "default": "."
            },
            "limit": {
                "type": "integer",
                "description": "最大返回条数;0 或缺省表示不限。",
                "default": 0
            }
        },
        "required": ["patterns"],
        "additionalProperties": false
    })
}

fn glob_from_args(args: &str) -> Result<api::message::tool_call::Tool> {
    let parsed: GlobArgs = serde_json::from_str(args)?;
    Ok(api::message::tool_call::Tool::FileGlobV2(
        api::message::tool_call::FileGlobV2 {
            patterns: parsed.patterns,
            search_dir: if parsed.search_dir.is_empty() {
                ".".to_owned()
            } else {
                parsed.search_dir
            },
            max_matches: parsed.limit,
            max_depth: 0, // 不限深度
            min_depth: 0,
        },
    ))
}

fn glob_result_to_json(result: &api::message::tool_call_result::Result) -> Option<Value> {
    use api::file_glob_v2_result::Result as GR;
    use api::message::tool_call_result::Result as R;
    let r = match result {
        R::FileGlobV2(r) => r,
        _ => return None,
    };
    let value = match &r.result {
        Some(GR::Success(s)) => {
            let files: Vec<&str> = s.matched_files.iter().map(|f| f.file_path.as_str()).collect();
            // protobuf 中 Success.warnings: String 是 stderr 警告文本(如权限错误)。
            // 仅在非空时输出,避免给模型噪声。
            let mut value = json!({ "status": "ok", "files": files });
            if !s.warnings.is_empty() {
                value["warnings"] = json!(s.warnings);
            }
            value
        }
        Some(GR::Error(e)) => json!({ "status": "error", "message": e.message }),
        None => json!({ "status": "cancelled" }),
    };
    Some(value)
}

pub static FILE_GLOB_V2: OpenAiTool = OpenAiTool {
    name: "file_glob",
    description: "用文件名通配符查找文件路径。支持 `?`(单字符)、`*`(单层任意)、\
                  `**`(跨层递归,如 `**/*.rs` 匹配所有 .rs)、`[abc]`(字符集)。\
                  patterns 数组任一命中即返回(OR 语义)。\
                  优先用本工具而不是 shell 调 find/ls(更快、自动通过审批、跨平台)。\
                  limit=0 表示不限条数(默认)。",
    parameters: glob_parameters,
    from_args: glob_from_args,
    result_to_json: glob_result_to_json,
};

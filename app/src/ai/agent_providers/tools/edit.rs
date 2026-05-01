//! `apply_file_diffs`:写文件 / 改文件 / 删文件 三合一。
//!
//! warp protobuf 中的 `ApplyFileDiffs` 包含 4 个并列 vec:
//! - `diffs`: search/replace 风格的字符串替换
//! - `v4a_updates`: V4A 风格的多 hunk 修补(高级,Phase 4 再加)
//! - `new_files`: 创建新文件
//! - `deleted_files`: 删除文件
//!
//! 给上游模型提供一个聚合的 `apply_file_diffs(operations)` 工具,通过
//! `op` 字段区分子类型 — 比让模型一次回 4 个并列数组更直观、错误率低。

use anyhow::Result;
use serde::Deserialize;
use serde_json::{json, Value};
use warp_multi_agent_api as api;

use super::OpenAiTool;

#[derive(Debug, Deserialize)]
struct Args {
    summary: String,
    operations: Vec<Operation>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "op")]
enum Operation {
    /// 字符串搜索-替换(最常用,适合改一两处)。
    #[serde(rename = "edit")]
    Edit {
        file_path: String,
        search: String,
        replace: String,
    },
    /// 创建新文件。
    #[serde(rename = "create")]
    Create { file_path: String, content: String },
    /// 删除已有文件。
    #[serde(rename = "delete")]
    Delete { file_path: String },
}

fn parameters() -> Value {
    json!({
        "type": "object",
        "properties": {
            "summary": {
                "type": "string",
                "description": "对本次修改的简短中文总结(1 句),会展示给用户审批用。"
            },
            "operations": {
                "type": "array",
                "description": "本次要执行的所有文件操作(可批量)。op 区分子类型: edit/create/delete。",
                "items": {
                    "oneOf": [
                        {
                            "type": "object",
                            "properties": {
                                "op": {"const": "edit"},
                                "file_path": {"type": "string"},
                                "search": {"type": "string", "description": "要被替换掉的原文片段(必须与文件中已存在的内容完全一致,包括空白/换行)。"},
                                "replace": {"type": "string", "description": "替换后的内容。"}
                            },
                            "required": ["op", "file_path", "search", "replace"]
                        },
                        {
                            "type": "object",
                            "properties": {
                                "op": {"const": "create"},
                                "file_path": {"type": "string"},
                                "content": {"type": "string"}
                            },
                            "required": ["op", "file_path", "content"]
                        },
                        {
                            "type": "object",
                            "properties": {
                                "op": {"const": "delete"},
                                "file_path": {"type": "string"}
                            },
                            "required": ["op", "file_path"]
                        }
                    ]
                }
            }
        },
        "required": ["summary", "operations"],
        "additionalProperties": false
    })
}

fn from_args(args: &str) -> Result<api::message::tool_call::Tool> {
    let parsed: Args = serde_json::from_str(args)?;
    let mut diffs = Vec::new();
    let mut new_files = Vec::new();
    let mut deleted_files = Vec::new();
    for op in parsed.operations {
        match op {
            Operation::Edit {
                file_path,
                search,
                replace,
            } => diffs.push(api::message::tool_call::apply_file_diffs::FileDiff {
                file_path,
                search,
                replace,
            }),
            Operation::Create { file_path, content } => new_files
                .push(api::message::tool_call::apply_file_diffs::NewFile { file_path, content }),
            Operation::Delete { file_path } => deleted_files
                .push(api::message::tool_call::apply_file_diffs::DeleteFile { file_path }),
        }
    }
    Ok(api::message::tool_call::Tool::ApplyFileDiffs(
        api::message::tool_call::ApplyFileDiffs {
            summary: parsed.summary,
            diffs,
            v4a_updates: vec![],
            new_files,
            deleted_files,
        },
    ))
}

fn result_to_json(result: &api::message::tool_call_result::Result) -> Option<Value> {
    use api::apply_file_diffs_result::Result as ApplyR;
    use api::message::tool_call_result::Result as R;
    let r = match result {
        R::ApplyFileDiffs(r) => r,
        _ => return None,
    };
    let value = match &r.result {
        Some(ApplyR::Success(s)) => {
            let updated: Vec<&str> = s
                .updated_files_v2
                .iter()
                .filter_map(|u| u.file.as_ref().map(|f| f.file_path.as_str()))
                .collect();
            let deleted: Vec<&str> = s
                .deleted_files
                .iter()
                .map(|f| f.file_path.as_str())
                .collect();
            json!({
                "status": "ok",
                "updated_files": updated,
                "deleted_files": deleted,
            })
        }
        Some(ApplyR::Error(e)) => json!({ "status": "error", "message": e.message }),
        None => json!({ "status": "cancelled_or_rejected" }),
    };
    Some(value)
}

pub static APPLY_FILE_DIFFS: OpenAiTool = OpenAiTool {
    name: "apply_file_diffs",
    description: include_str!("../prompts/tool_descriptions/apply_file_diffs.md"),
    parameters,
    from_args,
    result_to_json,
};

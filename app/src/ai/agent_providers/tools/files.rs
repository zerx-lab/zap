//! `ReadFiles` 适配。
//!
//! warp 中对应 `api::message::tool_call::Tool::ReadFiles`,
//! 执行后 result 是 `ToolCallResultType::ReadFiles(ReadFilesResult)`。

use anyhow::Result;
use serde::Deserialize;
use serde_json::{json, Value};
use warp_multi_agent_api as api;

use super::OpenAiTool;

#[derive(Debug, Deserialize)]
struct Args {
    files: Vec<FileArg>,
}

#[derive(Debug, Deserialize)]
struct FileArg {
    path: String,
    #[serde(default)]
    line_ranges: Vec<LineRangeArg>,
}

#[derive(Debug, Deserialize)]
struct LineRangeArg {
    start: u32,
    end: u32,
}

fn parameters() -> Value {
    json!({
        "type": "object",
        "properties": {
            "files": {
                "type": "array",
                "description": "要读取的文件列表。",
                "items": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "文件路径(相对当前工作目录或绝对路径均可)。"
                        },
                        "line_ranges": {
                            "type": "array",
                            "description": "可选的行号区间列表(1-based,闭区间)。\
                                            为空时读取整个文件。",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "start": {"type": "integer"},
                                    "end": {"type": "integer"}
                                },
                                "required": ["start", "end"]
                            }
                        }
                    },
                    "required": ["path"]
                }
            }
        },
        "required": ["files"],
        "additionalProperties": false
    })
}

fn from_args(args: &str) -> Result<api::message::tool_call::Tool> {
    let parsed: Args = serde_json::from_str(args)?;
    let files = parsed
        .files
        .into_iter()
        .map(|f| api::message::tool_call::read_files::File {
            name: f.path,
            line_ranges: f
                .line_ranges
                .into_iter()
                .map(|r| api::FileContentLineRange {
                    start: r.start,
                    end: r.end,
                })
                .collect(),
        })
        .collect();
    Ok(api::message::tool_call::Tool::ReadFiles(
        api::message::tool_call::ReadFiles { files },
    ))
}

fn result_to_json(result: &api::message::tool_call_result::Result) -> Option<Value> {
    use api::message::tool_call_result::Result as R;
    use api::read_files_result::Result as ReadR;
    let r = match result {
        R::ReadFiles(r) => r,
        _ => return None,
    };
    let value = match &r.result {
        Some(ReadR::AnyFilesSuccess(s)) => {
            let files: Vec<Value> = s
                .files
                .iter()
                .map(|f| {
                    let (path, content) = match &f.content {
                        Some(api::any_file_content::Content::TextContent(t)) => {
                            (t.file_path.clone(), t.content.clone())
                        }
                        Some(api::any_file_content::Content::BinaryContent(b)) => (
                            b.file_path.clone(),
                            format!("<binary, {} bytes>", b.data.len()),
                        ),
                        None => (String::new(), String::new()),
                    };
                    json!({ "path": path, "content": content })
                })
                .collect();
            json!({ "status": "ok", "files": files })
        }
        Some(ReadR::TextFilesSuccess(s)) => {
            let files: Vec<Value> = s
                .files
                .iter()
                .map(|f| json!({ "path": f.file_path, "content": f.content }))
                .collect();
            json!({ "status": "ok", "files": files })
        }
        Some(ReadR::Error(e)) => json!({ "status": "error", "message": e.message }),
        None => json!({ "status": "cancelled" }),
    };
    Some(value)
}

pub static READ_FILES: OpenAiTool = OpenAiTool {
    name: "read_files",
    description: "读取一个或多个文件的内容。可指定每个文件的行号区间(1-based,闭区间);\
                  不指定则读整个文件。返回 JSON: { files: [{path, content}, ...] }。",
    parameters,
    from_args,
    result_to_json,
};

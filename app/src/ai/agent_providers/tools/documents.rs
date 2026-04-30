//! Warp Drive 本地文档系统的 read / edit / create 三件套。
//!
//! 与 `read_files` / `apply_file_diffs` 区别:这些操作的目标是 **AIDocumentModel
//! 管理的文档**(Drive 内部本地文档,通过 `document_id` 引用),而不是文件系统
//! 中的文件。Executor 走 `crate::ai::document::ai_document_model::AIDocumentModel`,
//! 完全本地,不依赖任何 server。

use anyhow::Result;
use serde::Deserialize;
use serde_json::{json, Value};
use warp_multi_agent_api as api;

use super::OpenAiTool;

// ---------------------------------------------------------------------------
// 共用:DocumentContent → JSON
// ---------------------------------------------------------------------------

fn document_content_to_json(d: &api::DocumentContent) -> Value {
    let mut v = json!({
        "document_id": d.document_id,
        "content": d.content,
    });
    if let Some(lr) = &d.line_range {
        v["line_range"] = json!({ "start": lr.start, "end": lr.end });
    }
    v
}

// ---------------------------------------------------------------------------
// read_documents
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct ReadArgs {
    documents: Vec<ReadDoc>,
}

#[derive(Debug, Deserialize)]
struct ReadDoc {
    document_id: String,
    #[serde(default)]
    line_ranges: Vec<LineRange>,
}

#[derive(Debug, Deserialize)]
struct LineRange {
    start: u32,
    end: u32,
}

fn read_parameters() -> Value {
    json!({
        "type": "object",
        "properties": {
            "documents": {
                "type": "array",
                "description": "要读取的 document 列表(每个由 document_id 标识)。",
                "items": {
                    "type": "object",
                    "properties": {
                        "document_id": { "type": "string" },
                        "line_ranges": {
                            "type": "array",
                            "description": "可选的 1-based 闭区间行号列表,为空读整个文档。",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "start": { "type": "integer" },
                                    "end": { "type": "integer" }
                                },
                                "required": ["start", "end"]
                            }
                        }
                    },
                    "required": ["document_id"]
                }
            }
        },
        "required": ["documents"],
        "additionalProperties": false
    })
}

fn read_from_args(args: &str) -> Result<api::message::tool_call::Tool> {
    let parsed: ReadArgs = serde_json::from_str(args)?;
    let docs = parsed
        .documents
        .into_iter()
        .map(|d| api::message::tool_call::read_documents::Document {
            document_id: d.document_id,
            line_ranges: d
                .line_ranges
                .into_iter()
                .map(|r| api::FileContentLineRange {
                    start: r.start,
                    end: r.end,
                })
                .collect(),
        })
        .collect();
    Ok(api::message::tool_call::Tool::ReadDocuments(
        api::message::tool_call::ReadDocuments { documents: docs },
    ))
}

fn read_result_to_json(result: &api::message::tool_call_result::Result) -> Option<Value> {
    use api::message::tool_call_result::Result as R;
    use api::read_documents_result::Result as DR;
    let r = match result {
        R::ReadDocuments(r) => r,
        _ => return None,
    };
    let value = match &r.result {
        Some(DR::Success(s)) => json!({
            "status": "ok",
            "documents": s.documents.iter().map(document_content_to_json).collect::<Vec<_>>(),
        }),
        Some(DR::Error(e)) => json!({ "status": "error", "message": e.message }),
        None => json!({ "status": "cancelled" }),
    };
    Some(value)
}

pub static READ_DOCUMENTS: OpenAiTool = OpenAiTool {
    name: "read_documents",
    description: "读取 Warp Drive 本地文档(由 document_id 引用,不是文件系统中的文件)。\
                  返回 JSON: { documents: [{document_id, content, line_range?}] }。\
                  当用户提到具体 document_id 或 Drive 中的特定文档时使用。",
    parameters: read_parameters,
    from_args: read_from_args,
    result_to_json: read_result_to_json,
};

// ---------------------------------------------------------------------------
// edit_documents
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct EditArgs {
    diffs: Vec<DocDiff>,
}

#[derive(Debug, Deserialize)]
struct DocDiff {
    document_id: String,
    search: String,
    replace: String,
}

fn edit_parameters() -> Value {
    json!({
        "type": "object",
        "properties": {
            "diffs": {
                "type": "array",
                "description": "对若干 document 各做一次 search→replace。每条 diff 描述一处替换。",
                "items": {
                    "type": "object",
                    "properties": {
                        "document_id": { "type": "string" },
                        "search": {
                            "type": "string",
                            "description": "要被替换的原文(必须与 document 现有内容**完全一致**,含空白和换行)。"
                        },
                        "replace": {
                            "type": "string",
                            "description": "替换后的内容。"
                        }
                    },
                    "required": ["document_id", "search", "replace"]
                }
            }
        },
        "required": ["diffs"],
        "additionalProperties": false
    })
}

fn edit_from_args(args: &str) -> Result<api::message::tool_call::Tool> {
    let parsed: EditArgs = serde_json::from_str(args)?;
    let diffs = parsed
        .diffs
        .into_iter()
        .map(
            |d| api::message::tool_call::edit_documents::DocumentDiff {
                document_id: d.document_id,
                search: d.search,
                replace: d.replace,
            },
        )
        .collect();
    Ok(api::message::tool_call::Tool::EditDocuments(
        api::message::tool_call::EditDocuments { diffs },
    ))
}

fn edit_result_to_json(result: &api::message::tool_call_result::Result) -> Option<Value> {
    use api::edit_documents_result::Result as ER;
    use api::message::tool_call_result::Result as R;
    let r = match result {
        R::EditDocuments(r) => r,
        _ => return None,
    };
    let value = match &r.result {
        Some(ER::Success(s)) => json!({
            "status": "ok",
            "updated_documents": s.updated_documents.iter().map(document_content_to_json).collect::<Vec<_>>(),
        }),
        Some(ER::Error(e)) => json!({ "status": "error", "message": e.message }),
        None => json!({ "status": "cancelled" }),
    };
    Some(value)
}

pub static EDIT_DOCUMENTS: OpenAiTool = OpenAiTool {
    name: "edit_documents",
    description: "对 Warp Drive 中已存在的 document 做字符串搜索-替换。\
                  和 apply_file_diffs::edit 相似,但目标是 Drive document(通过 document_id 引用)。\
                  search 必须与文档现有内容**完全一致**(含空白和换行),否则失败。",
    parameters: edit_parameters,
    from_args: edit_from_args,
    result_to_json: edit_result_to_json,
};

// ---------------------------------------------------------------------------
// create_documents
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct CreateArgs {
    new_documents: Vec<NewDoc>,
}

#[derive(Debug, Deserialize)]
struct NewDoc {
    title: String,
    content: String,
}

fn create_parameters() -> Value {
    json!({
        "type": "object",
        "properties": {
            "new_documents": {
                "type": "array",
                "description": "要创建的新 document 列表。",
                "items": {
                    "type": "object",
                    "properties": {
                        "title": {
                            "type": "string",
                            "description": "文档标题(在 Drive 中显示)。"
                        },
                        "content": {
                            "type": "string",
                            "description": "文档完整内容(markdown / 纯文本)。"
                        }
                    },
                    "required": ["title", "content"]
                }
            }
        },
        "required": ["new_documents"],
        "additionalProperties": false
    })
}

fn create_from_args(args: &str) -> Result<api::message::tool_call::Tool> {
    let parsed: CreateArgs = serde_json::from_str(args)?;
    let new_documents = parsed
        .new_documents
        .into_iter()
        .map(
            |d| api::message::tool_call::create_documents::NewDocument {
                title: d.title,
                content: d.content,
            },
        )
        .collect();
    Ok(api::message::tool_call::Tool::CreateDocuments(
        api::message::tool_call::CreateDocuments { new_documents },
    ))
}

fn create_result_to_json(result: &api::message::tool_call_result::Result) -> Option<Value> {
    use api::create_documents_result::Result as CR;
    use api::message::tool_call_result::Result as R;
    let r = match result {
        R::CreateDocuments(r) => r,
        _ => return None,
    };
    let value = match &r.result {
        Some(CR::Success(s)) => json!({
            "status": "ok",
            "created_documents": s.created_documents.iter().map(document_content_to_json).collect::<Vec<_>>(),
        }),
        Some(CR::Error(e)) => json!({ "status": "error", "message": e.message }),
        None => json!({ "status": "cancelled" }),
    };
    Some(value)
}

pub static CREATE_DOCUMENTS: OpenAiTool = OpenAiTool {
    name: "create_documents",
    description: "在 Warp Drive 中创建一个或多个新 document(各带 title + 完整内容)。\
                  适合把分析结果、笔记、todo 等沉淀为可复用的 Drive 文档。",
    parameters: create_parameters,
    from_args: create_from_args,
    result_to_json: create_result_to_json,
};

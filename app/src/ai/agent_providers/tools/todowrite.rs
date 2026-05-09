//! `todowrite` BYOP 工具 descriptor。
//!
//! 跟 webfetch / websearch 一样**不**映射到 protobuf executor variant —— 由
//! `chat_stream.rs` 在 `parse_incoming_tool_call` 之前按 name 拦截,直接合成
//! `Message::UpdateTodos` 写入 conversation,触发 chip + popup UI 更新。
//!
//! 协议设计对齐 opencode `todowrite`:
//! - 入参:`{ todos: [{ content, status, priority? }] }`,**全量覆盖式**(每次调用都替换整个 list)
//! - status:`pending` / `in_progress` / `completed` / `cancelled`
//! - 客户端按 `content` 算 stable id(SHA-256 前缀,16 hex)避免每次刷新 chip 数字
//!
//! ## emit 策略
//!
//! 每次拦截一次 todowrite 调用 → 合成两条 `Message::UpdateTodos`:
//! 1. `CreateTodoList { initial_todos: [全部 todos] }`(全部进 pending)
//! 2. `MarkTodosCompleted { todo_ids: [status=completed/cancelled 的 id] }`
//!
//! `update_todo_list_from_todo_op` 会把第二条命中的项从 pending 移到 completed
//! (`mark_todos_complete` 在 pending 里 lookup id),最终 `AIAgentTodoList` 状态:
//! `completed_items = [completed]`、`pending_items = [pending + in_progress]`。
//! Warp UI `in_progress_item()` 拿 `pending_items.first()`,所以 in_progress 的
//! todo 应该是 `todos` 数组里第一个 `status != completed/cancelled` 的项。
//!
//! 然后再合成一对 `Message::ToolCall`(carrier,tool=None) + `Message::ToolCallResult`
//! 给上游模型 unblock。

use anyhow::{anyhow, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;
use warp_multi_agent_api as api;

use super::OpenAiTool;

pub const TOOL_NAME: &str = "todowrite";

#[derive(Debug, Deserialize)]
pub struct Args {
    pub todos: Vec<TodoArg>,
}

#[derive(Debug, Deserialize)]
pub struct TodoArg {
    pub content: String,
    /// `pending` | `in_progress` | `completed` | `cancelled`。模型偶发会送别的字符串,
    /// 解析时按未识别值兜底为 `pending`。
    #[serde(default)]
    pub status: String,
    /// opencode 协议带 priority,Warp 数据模型不区分,这里收下但不用,
    /// 保留是为了让模型按 opencode 习惯发参数不报错。
    #[serde(default, rename = "priority")]
    pub _priority: Option<String>,
}

fn parameters() -> Value {
    json!({
        "type": "object",
        "properties": {
            "todos": {
                "type": "array",
                "description": "The full updated todo list. Pass every item every call (overwrite semantics).",
                "items": {
                    "type": "object",
                    "properties": {
                        "content": {
                            "type": "string",
                            "description": "Brief description of the task (1 line)."
                        },
                        "status": {
                            "type": "string",
                            "enum": ["pending", "in_progress", "completed", "cancelled"],
                            "description": "Current status."
                        },
                        "priority": {
                            "type": "string",
                            "enum": ["high", "medium", "low"],
                            "description": "Optional priority. Currently advisory only."
                        }
                    },
                    "required": ["content", "status"]
                }
            }
        },
        "required": ["todos"],
        "additionalProperties": false
    })
}

fn from_args(_args: &str) -> Result<api::message::tool_call::Tool> {
    Err(anyhow!(
        "todowrite is intercepted by chat_stream BYOP todo dispatcher; \
         from_args should never be called"
    ))
}

fn result_to_json(_result: &api::message::tool_call_result::Result) -> Option<Value> {
    None
}

pub static TODOWRITE: OpenAiTool = OpenAiTool {
    name: TOOL_NAME,
    description: include_str!("../prompts/tool_descriptions/todowrite.md"),
    parameters,
    from_args,
    result_to_json,
};

/// 根据 content 计算稳定 id。模型用同样 content 第二次发 todo 时拿到同一个 id,
/// 这样 `mark_todos_complete(todo_ids)` 才能在 pending 里命中 → 把它移到 completed。
fn stable_id(content: &str) -> String {
    let mut h = Sha256::new();
    h.update(content.as_bytes());
    let bytes = h.finalize();
    // 取前 8 字节 = 16 hex,够稳够短。
    bytes.iter().take(8).map(|b| format!("{b:02x}")).collect()
}

fn to_todo_item(arg: &TodoArg) -> api::TodoItem {
    api::TodoItem {
        id: stable_id(&arg.content),
        title: arg.content.clone(),
        description: String::new(),
    }
}

fn is_completed_status(s: &str) -> bool {
    matches!(s, "completed" | "cancelled")
}

/// 合成两条 `Message::UpdateTodos`(创建新 list + mark completed)。
/// chat_stream 拦截 todowrite 时调用本函数,把返回 messages yield 出去。
pub fn build_update_todos_messages(
    args_str: &str,
    task_id: &str,
    request_id: &str,
) -> Result<Vec<api::Message>> {
    let parsed: Args =
        serde_json::from_str(args_str).map_err(|e| anyhow!("todowrite args parse error: {e}"))?;

    // 全部 todos 进 pending(顺序保持模型给的顺序),这是 CreateTodoList 的入口。
    let initial_todos: Vec<api::TodoItem> = parsed.todos.iter().map(to_todo_item).collect();
    // 然后 mark 那些 status=completed/cancelled 的 id 完成。
    let completed_ids: Vec<String> = parsed
        .todos
        .iter()
        .filter(|t| is_completed_status(&t.status))
        .map(|t| stable_id(&t.content))
        .collect();

    let mut messages = Vec::with_capacity(2);

    messages.push(make_update_todos_message(
        task_id,
        request_id,
        api::message::update_todos::Operation::CreateTodoList(api::CreateTodoList {
            initial_todos,
        }),
    ));

    if !completed_ids.is_empty() {
        messages.push(make_update_todos_message(
            task_id,
            request_id,
            api::message::update_todos::Operation::MarkTodosCompleted(api::MarkTodosCompleted {
                todo_ids: completed_ids,
            }),
        ));
    }

    Ok(messages)
}

fn make_update_todos_message(
    task_id: &str,
    request_id: &str,
    operation: api::message::update_todos::Operation,
) -> api::Message {
    api::Message {
        id: Uuid::new_v4().to_string(),
        task_id: task_id.to_owned(),
        server_message_data: String::new(),
        citations: vec![],
        message: Some(api::message::Message::UpdateTodos(
            api::message::UpdateTodos {
                operation: Some(operation),
            },
        )),
        request_id: request_id.to_owned(),
        timestamp: None,
    }
}

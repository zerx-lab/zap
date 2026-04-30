//! UI 信号 marker 类工具:执行即"通知前端做某事",result 是固定 ack。
//!
//! - `open_code_review`: 打开 Code Review 面板
//! - `init_project`: 触发项目初始化向导(创建 CLAUDE.md / .warp/rules.md 等)
//! - `transfer_shell_command_control_to_user`: 把长运行命令的 PTY 控制权交给用户
//!
//! 这些工具的 protobuf 字段都很少(空 message 或一个字段),executor 大多是
//! 直接转固定 result 的 marker 路径,client 端的实际副作用由 UI/Terminal
//! 监听对应 ToolCall message 后触发。

use anyhow::Result;
use serde::Deserialize;
use serde_json::{json, Value};
use warp_multi_agent_api as api;

use super::OpenAiTool;

// ---------------------------------------------------------------------------
// open_code_review
// ---------------------------------------------------------------------------

fn empty_parameters() -> Value {
    json!({
        "type": "object",
        "properties": {},
        "additionalProperties": false
    })
}

fn open_code_review_from_args(_args: &str) -> Result<api::message::tool_call::Tool> {
    Ok(api::message::tool_call::Tool::OpenCodeReview(
        api::message::tool_call::OpenCodeReview {},
    ))
}

fn open_code_review_result_to_json(result: &api::message::tool_call_result::Result) -> Option<Value> {
    use api::message::tool_call_result::Result as R;
    match result {
        R::OpenCodeReview(_) => Some(json!({ "status": "ok" })),
        _ => None,
    }
}

pub static OPEN_CODE_REVIEW: OpenAiTool = OpenAiTool {
    name: "open_code_review",
    description: "打开当前项目的 Code Review 面板(client UI 信号,无参数)。\
                  当用户明确要求开 code review,或上下文显示要开始审查阶段时使用。",
    parameters: empty_parameters,
    from_args: open_code_review_from_args,
    result_to_json: open_code_review_result_to_json,
};

// ---------------------------------------------------------------------------
// init_project
// ---------------------------------------------------------------------------

fn init_project_from_args(_args: &str) -> Result<api::message::tool_call::Tool> {
    Ok(api::message::tool_call::Tool::InitProject(
        api::message::tool_call::InitProject {},
    ))
}

fn init_project_result_to_json(result: &api::message::tool_call_result::Result) -> Option<Value> {
    use api::message::tool_call_result::Result as R;
    match result {
        R::InitProject(_) => Some(json!({ "status": "ok" })),
        _ => None,
    }
}

pub static INIT_PROJECT: OpenAiTool = OpenAiTool {
    name: "init_project",
    description: "触发当前 workspace 的项目初始化向导(client UI 信号,无参数)。\
                  当用户首次在新 repo 上启动 agent、或显式要求初始化项目时使用。",
    parameters: empty_parameters,
    from_args: init_project_from_args,
    result_to_json: init_project_result_to_json,
};

// ---------------------------------------------------------------------------
// transfer_shell_command_control_to_user
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct TransferArgs {
    /// 给用户看的解释:为什么要交还控制权。
    #[serde(default)]
    reason: String,
}

fn transfer_parameters() -> Value {
    json!({
        "type": "object",
        "properties": {
            "reason": {
                "type": "string",
                "description": "向用户解释为什么需要把控制权交还(例如「现在需要你手动登录交互」)。"
            }
        },
        "additionalProperties": false
    })
}

fn transfer_from_args(args: &str) -> Result<api::message::tool_call::Tool> {
    let parsed: TransferArgs = if args.trim().is_empty() {
        TransferArgs { reason: String::new() }
    } else {
        serde_json::from_str(args)?
    };
    Ok(api::message::tool_call::Tool::TransferShellCommandControlToUser(
        api::message::tool_call::TransferShellCommandControlToUser {
            reason: parsed.reason,
        },
    ))
}

fn transfer_result_to_json(result: &api::message::tool_call_result::Result) -> Option<Value> {
    use api::message::tool_call_result::Result as R;
    use api::transfer_shell_command_control_to_user_result::Result as TR;
    let r = match result {
        R::TransferShellCommandControlToUser(r) => r,
        _ => return None,
    };
    let value = match &r.result {
        Some(TR::LongRunningCommandSnapshot(s)) => json!({
            "status": "transferred",
            "command_id": s.command_id,
            "output": s.output,
            "is_alt_screen_active": s.is_alt_screen_active,
        }),
        Some(TR::CommandFinished(f)) => json!({
            "status": "completed",
            "command_id": f.command_id,
            "exit_code": f.exit_code,
            "output": f.output,
        }),
        Some(TR::Error(_)) => json!({ "status": "error", "message": "block_not_found" }),
        None => json!({ "status": "cancelled" }),
    };
    Some(value)
}

pub static TRANSFER_SHELL_CONTROL: OpenAiTool = OpenAiTool {
    name: "transfer_shell_command_control_to_user",
    description: "把当前长运行 shell 命令的 PTY 控制权交还给用户。\
                  适用场景:命令需要用户手动交互且场景不适合用 write_to_long_running_shell_command\
                  (如交互式登录、需要看终端实时回显才能决定下一步操作等)。\
                  reason 字段会展示给用户,用于解释为什么要交还。",
    parameters: transfer_parameters,
    from_args: transfer_from_args,
    result_to_json: transfer_result_to_json,
};

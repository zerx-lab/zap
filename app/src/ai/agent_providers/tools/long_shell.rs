//! 长运行 shell 命令的交互工具:
//! - `write_to_long_running_shell_command`: 给一个尚在运行的命令写 stdin/PTY
//! - `read_shell_command_output`: 拿一个尚在运行命令的当前输出快照
//!
//! 这两个工具的 `command_id` 来自 `run_shell_command` 的初始 snapshot
//! (`LongRunningShellCommandSnapshot.command_id`)。模型在调用前需要先看到一个
//! 长运行 shell 的 snapshot 拿到 id。

use anyhow::Result;
use serde::Deserialize;
use serde_json::{json, Value};
use warp_multi_agent_api as api;

use super::OpenAiTool;

// ---------------------------------------------------------------------------
// write_to_long_running_shell_command
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct WriteArgs {
    command_id: String,
    input: String,
    /// "raw" | "line" | "block",默认 "line"
    #[serde(default = "default_mode")]
    mode: String,
}

fn default_mode() -> String {
    "line".to_owned()
}

fn write_parameters() -> Value {
    json!({
        "type": "object",
        "properties": {
            "command_id": {
                "type": "string",
                "description": "之前 run_shell_command 返回的长运行命令 id。"
            },
            "input": {
                "type": "string",
                "description": "要写到 stdin/PTY 的文本。"
            },
            "mode": {
                "type": "string",
                "enum": ["raw", "line", "block"],
                "description": "raw=原始字节;line=作为一行(自动加换行);block=作为多行块。",
                "default": "line"
            }
        },
        "required": ["command_id", "input"],
        "additionalProperties": false
    })
}

fn write_from_args(args: &str) -> Result<api::message::tool_call::Tool> {
    let parsed: WriteArgs = serde_json::from_str(args)?;
    use api::message::tool_call::write_to_long_running_shell_command::mode::Mode as InnerMode;
    use api::message::tool_call::write_to_long_running_shell_command::Mode;
    let inner = match parsed.mode.as_str() {
        "raw" => InnerMode::Raw(()),
        "block" => InnerMode::Block(()),
        _ => InnerMode::Line(()),
    };
    Ok(api::message::tool_call::Tool::WriteToLongRunningShellCommand(
        api::message::tool_call::WriteToLongRunningShellCommand {
            command_id: parsed.command_id,
            input: parsed.input.into_bytes(),
            mode: Some(Mode { mode: Some(inner) }),
        },
    ))
}

fn write_result_to_json(result: &api::message::tool_call_result::Result) -> Option<Value> {
    use api::message::tool_call_result::Result as R;
    use api::write_to_long_running_shell_command_result::Result as WR;
    let r = match result {
        R::WriteToLongRunningShellCommand(r) => r,
        _ => return None,
    };
    let value = match &r.result {
        Some(WR::LongRunningCommandSnapshot(s)) => json!({
            "status": "running",
            "command_id": s.command_id,
            "output": s.output,
            "is_alt_screen_active": s.is_alt_screen_active,
        }),
        Some(WR::CommandFinished(f)) => json!({
            "status": "completed",
            "command_id": f.command_id,
            "exit_code": f.exit_code,
            "output": f.output,
        }),
        // ShellCommandError 现仅有 BlockNotFound 一个 variant
        Some(WR::Error(_)) => json!({
            "status": "error",
            "message": "block_not_found_or_command_id_invalid",
        }),
        None => json!({ "status": "cancelled" }),
    };
    Some(value)
}

pub static WRITE_TO_LONG_RUNNING_SHELL_COMMAND: OpenAiTool = OpenAiTool {
    name: "write_to_long_running_shell_command",
    description: "向一个长运行 shell 命令的 stdin/PTY 写文本(交互式输入或终止信号)。\
                  command_id 必须来自前一轮 run_shell_command(wait_until_complete=false)返回的 \
                  LongRunningCommandSnapshot.command_id;不能凭空构造、不能跨 session 复用。\
                  mode=line 自动加换行(默认,适合一般输入);\
                  mode=raw 原始字节(发 \\x03 终止进程、\\t 补全、方向键等控制序列必须用 raw);\
                  mode=block 多行块。\
                  返回 status=running 表示进程还在跑、completed 表示已退出、error 表示 command_id 失效。",
    parameters: write_parameters,
    from_args: write_from_args,
    result_to_json: write_result_to_json,
};

// ---------------------------------------------------------------------------
// read_shell_command_output
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct ReadArgs {
    command_id: String,
    /// "on_completion"(默认)或 number(秒数 → Duration)
    #[serde(default)]
    delay_seconds: Option<u64>,
}

fn read_parameters() -> Value {
    json!({
        "type": "object",
        "properties": {
            "command_id": {
                "type": "string",
                "description": "运行中命令的 id。"
            },
            "delay_seconds": {
                "type": "integer",
                "description": "可选: 在指定秒数后返回当前 snapshot;不填则等到命令完成才返回。",
                "minimum": 0
            }
        },
        "required": ["command_id"],
        "additionalProperties": false
    })
}

fn read_from_args(args: &str) -> Result<api::message::tool_call::Tool> {
    let parsed: ReadArgs = serde_json::from_str(args)?;
    use api::message::tool_call::read_shell_command_output::Delay;
    let delay = match parsed.delay_seconds {
        Some(secs) => Delay::Duration(prost_types::Duration {
            seconds: secs as i64,
            nanos: 0,
        }),
        None => Delay::OnCompletion(()),
    };
    Ok(api::message::tool_call::Tool::ReadShellCommandOutput(
        api::message::tool_call::ReadShellCommandOutput {
            command_id: parsed.command_id,
            delay: Some(delay),
        },
    ))
}

fn read_result_to_json(result: &api::message::tool_call_result::Result) -> Option<Value> {
    use api::message::tool_call_result::Result as R;
    use api::read_shell_command_output_result::Result as ReadR;
    let r = match result {
        R::ReadShellCommandOutput(r) => r,
        _ => return None,
    };
    let value = match &r.result {
        Some(ReadR::LongRunningCommandSnapshot(s)) => json!({
            "status": "running",
            "command": r.command,
            "command_id": s.command_id,
            "output": s.output,
            "is_alt_screen_active": s.is_alt_screen_active,
        }),
        Some(ReadR::CommandFinished(f)) => json!({
            "status": "completed",
            "command": r.command,
            "command_id": f.command_id,
            "exit_code": f.exit_code,
            "output": f.output,
        }),
        Some(ReadR::Error(_)) => json!({ "status": "error", "command": r.command }),
        None => json!({ "status": "cancelled", "command": r.command }),
    };
    Some(value)
}

pub static READ_SHELL_COMMAND_OUTPUT: OpenAiTool = OpenAiTool {
    name: "read_shell_command_output",
    description: "读取一个长运行 shell 命令的当前 stdout 快照。\
                  command_id 来源同 write_to_long_running_shell_command(必须先有 \
                  run_shell_command(wait_until_complete=false) 的 snapshot)。\
                  delay_seconds 不填 = 阻塞到命令自然结束才返回(谨慎使用,dev server 不会自己退出);\
                  delay_seconds=N(秒) = N 秒后返回当前 snapshot,不管命令有没有结束(轮询输出推荐用法)。\
                  返回 status=running 时 output 是当前已收到的 stdout 累积。",
    parameters: read_parameters,
    from_args: read_from_args,
    result_to_json: read_result_to_json,
};

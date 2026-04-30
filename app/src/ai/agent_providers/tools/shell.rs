//! `RunShellCommand` 适配。
//!
//! warp 中对应 `api::message::tool_call::Tool::RunShellCommand`,
//! 执行后 result 是 `ToolCallResultType::RunShellCommand(RunShellCommandResult)`。

use anyhow::Result;
use serde::Deserialize;
use serde_json::{json, Value};
use warp_multi_agent_api as api;

use super::OpenAiTool;

#[derive(Debug, Deserialize)]
struct Args {
    command: String,
    #[serde(default)]
    is_read_only: bool,
    #[serde(default)]
    uses_pager: bool,
    #[serde(default)]
    is_risky: bool,
    /// `None`(缺省 / true)= 等命令完成后再返回;`Some(false)` = 启动后立刻返回
    /// 一个 LongRunningCommandSnapshot,后续可用 read/write_to_long_running_*
    /// 工具继续交互(适合 dev server / tail -f 类持续运行命令)。
    #[serde(default)]
    wait_until_complete: Option<bool>,
}

fn parameters() -> Value {
    json!({
        "type": "object",
        "properties": {
            "command": {
                "type": "string",
                "description": "要执行的 shell 命令(完整命令行)。"
            },
            "is_read_only": {
                "type": "boolean",
                "description": "命令是否仅读取信息、不修改文件系统/外部状态(true 时无需用户确认)。",
                "default": false
            },
            "uses_pager": {
                "type": "boolean",
                "description": "命令是否会触发 pager(less/more 等)。建议 false,可附加 | cat 之类避免阻塞。",
                "default": false
            },
            "is_risky": {
                "type": "boolean",
                "description": "命令是否危险(rm -rf、改全局配置等)。设为 true 让用户更醒目地确认。",
                "default": false
            },
            "wait_until_complete": {
                "type": "boolean",
                "description": "默认 true(等命令结束才返回,适合一次性命令)。dev server / 后台进程 / tail -f / 交互 REPL 这类不会自然退出的命令必须设为 false,否则当前 turn 会卡死永远等不到结果。设 false 后会立刻返回 LongRunningCommandSnapshot,后续 turn 用 read/write_to_long_running_shell_command 继续交互。",
                "default": true
            }
        },
        "required": ["command"],
        "additionalProperties": false
    })
}

fn from_args(args: &str) -> Result<api::message::tool_call::Tool> {
    use api::message::tool_call::run_shell_command::WaitUntilCompleteValue;
    let parsed: Args = serde_json::from_str(args)?;
    // None 时显式默认成 true(等命令完成才返回),避免 controller 端的隐式默认行为
    // 在不同 warp 版本/路径下出现歧义。模型若想要长运行模式必须显式传 false。
    let wait_until_complete_value = Some(WaitUntilCompleteValue::WaitUntilComplete(
        parsed.wait_until_complete.unwrap_or(true),
    ));
    Ok(api::message::tool_call::Tool::RunShellCommand(
        api::message::tool_call::RunShellCommand {
            command: parsed.command,
            is_read_only: parsed.is_read_only,
            uses_pager: parsed.uses_pager,
            is_risky: parsed.is_risky,
            citations: vec![],
            wait_until_complete_value,
            risk_category: 0,
        },
    ))
}

fn result_to_json(result: &api::message::tool_call_result::Result) -> Option<Value> {
    use api::message::tool_call_result::Result as R;
    use api::run_shell_command_result::Result as ShellR;
    let r = match result {
        R::RunShellCommand(r) => r,
        _ => return None,
    };
    let value = match &r.result {
        Some(ShellR::CommandFinished(f)) => json!({
            "status": "completed",
            "command": r.command,
            "exit_code": f.exit_code,
            "output": f.output,
        }),
        // 长运行命令: 启动了但还没结束。把 snapshot 暴露给模型,这样模型可以
        // 决定是继续读 (read_shell_command_output) 还是写 (write_to_long_running_*)。
        Some(ShellR::LongRunningCommandSnapshot(s)) => json!({
            "status": "running",
            "command": r.command,
            "command_id": s.command_id,
            "output": s.output,
            "is_alt_screen_active": s.is_alt_screen_active,
        }),
        Some(ShellR::PermissionDenied(_)) => json!({
            "status": "permission_denied",
            "command": r.command,
        }),
        None => json!({ "status": "cancelled", "command": r.command }),
    };
    Some(value)
}

pub static RUN_SHELL_COMMAND: OpenAiTool = OpenAiTool {
    name: "run_shell_command",
    description: "在用户当前 shell 中执行命令并返回 stdout/stderr 与 exit code。\
                  【关键】对于 dev server / watcher / tail -f / 交互式 REPL 等持续进程,\
                  必须传 wait_until_complete=false,否则会一直挂起永远等不到结束。\
                  拿到 LongRunningCommandSnapshot 里的 command_id 之后:\
                  用 read_shell_command_output 读最新输出、\
                  用 write_to_long_running_shell_command 发输入(raw mode 写 \\x03 可发 Ctrl-C 终止)。\
                  只读命令(ls/cat/git status 等)保持默认 wait_until_complete=true 即可,\
                  is_read_only=true 时更易自动通过审批。\
                  命令需要用户审批后才会真正执行(除非配置了 auto-approve / allowlist)。",
    parameters,
    from_args,
    result_to_json,
};

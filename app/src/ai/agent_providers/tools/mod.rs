//! BYOP 模式下 OpenAI tool calling 的双向翻译注册表。
//!
//! 每个 warp 内置 tool(`api::message::tool_call::Tool` 的 variant)对应一个
//! [`OpenAiTool`] 描述: function name + JSON Schema + 反向解析 args + 把执行
//! result 序列化为给上游模型看的字符串。
//!
//! ## 当前实现的子集(Phase 3a 第一批)
//!
//! - `run_shell_command`
//! - `read_files`
//!
//! 后续轮次扩展:`grep` / `file_glob_v2` / `apply_file_diffs` / `call_mcp_tool` 等。
//!
//! ## 闭环说明
//!
//! 模型回 `tool_calls` → `from_args` 翻成 `tool_call::Tool` → 我们 emit
//! `Message::ToolCall { tool_call_id, tool }` → warp 自家 `convert_from.rs`
//! 自动翻成 `AIAgentAction` → executor 走 profile 权限/弹窗 → 执行 → result
//! 自动写回 conversation → 触发下一轮 byop request → 我们的 `result_to_json`
//! 把 result 序列化为 `role=tool, tool_call_id=...` 的 content 给上游。

pub mod ask;
pub mod codebase;
pub mod edit;
pub mod files;
pub mod long_shell;
pub mod mcp;
pub mod search;
pub mod shell;
pub mod skill;

use anyhow::Result;
use serde_json::Value;
use warp_multi_agent_api as api;

use crate::ai::agent::AIAgentActionResult;

/// 一条 tool 的双向适配描述。
pub struct OpenAiTool {
    /// 给上游 OpenAI 兼容 API 的 function name(LLM 在响应中按此名调用)。
    pub name: &'static str,
    /// 给 LLM 的描述。
    pub description: &'static str,
    /// 参数 JSON Schema(OpenAI 协议要求)。返回闭包以避免在 const 中构造 serde_json::Value。
    pub parameters: fn() -> Value,
    /// 反向解析: 上游模型返回的 args JSON 字符串 → warp 内部 `tool_call::Tool` variant。
    pub from_args: fn(args: &str) -> Result<api::message::tool_call::Tool>,
    /// 把 ToolCallResult 中对应该 tool 的 `Result` variant 转成给上游模型可读的 JSON。
    /// 没有匹配的 variant 时返回 `None`(让调用方 fallback 到 generic 序列化)。
    pub result_to_json: fn(&api::message::tool_call_result::Result) -> Option<Value>,
}

/// 注册表:全部已支持的 BYOP tool。
pub const REGISTRY: &[&OpenAiTool] = &[
    &shell::RUN_SHELL_COMMAND,
    &files::READ_FILES,
    &search::GREP,
    &search::FILE_GLOB_V2,
    &codebase::SEARCH_CODEBASE,
    &edit::APPLY_FILE_DIFFS,
    &long_shell::WRITE_TO_LONG_RUNNING_SHELL_COMMAND,
    &long_shell::READ_SHELL_COMMAND_OUTPUT,
    &ask::ASK_USER_QUESTION,
    &skill::READ_SKILL,
];

/// 按 OpenAI function name 反查注册表。
pub fn lookup(name: &str) -> Option<&'static OpenAiTool> {
    REGISTRY.iter().copied().find(|t| t.name == name)
}

/// 给定一条 ToolCallResult,优先在 REGISTRY 中找到对应的 tool 并用其 `result_to_json`
/// 序列化;找不到时尝试 MCP 通用序列化;再兜底到一个简短描述,避免 panic。
pub fn serialize_result(result: &api::message::ToolCallResult) -> String {
    let inner = match &result.result {
        Some(r) => r,
        None => return r#"{"status":"cancelled"}"#.to_owned(),
    };
    for t in REGISTRY {
        if let Some(json) = (t.result_to_json)(inner) {
            return serde_json::to_string(&json).unwrap_or_else(|_| "{}".to_owned());
        }
    }
    if let Some(json) = mcp::serialize_result(inner) {
        return serde_json::to_string(&json).unwrap_or_else(|_| "{}".to_owned());
    }
    // Fallback:不识别的 variant(用户后续轮次还没注册的 tool 也走这里)。
    r#"{"status":"unsupported_tool_result"}"#.to_owned()
}

/// 把 *当前轮 client 端执行* 完毕的 `AIAgentActionResult` 序列化为 JSON 字符串
/// 喂给上游模型(role=tool 的 content)。
///
/// ## 为什么不直接用 `AIAgentActionResultType::Display`
///
/// `Display` impl 把结构化结果(尤其是 `LongRunningCommandSnapshot`)渲染成
/// `"Command 'bun repl' is long-running"` 这类一行字符串,**完全丢弃 block_id
/// (=command_id)、grid_contents、is_alt_screen_active 等关键字段**,导致下一轮
/// 模型拿不到 command_id 没法继续 read/write_to_long_running_*,长运行命令完全废掉。
///
/// ## 工作原理
///
/// 1. 复用 `app/src/ai/agent/api/convert_to.rs` 中既有的 `TryFrom<AIAgentActionResult>
///    for api::request::input::user_inputs::user_input::Input`(覆盖全部 25+ ActionResult
///    variant),拿到 `Input::ToolCallResult { result, .. }`
/// 2. inner `*Result` 类型(如 `RunShellCommandResult`)与 `api::message::tool_call_result::Result`
///    共用同一个 protobuf message,只是外层 enum 的命名空间不同,所以可以重新包一次
///    外层 enum 复用 `tools::REGISTRY` 中既有的 per-tool `result_to_json`
///    (见 `shell.rs::result_to_json` 把 `LongRunningCommandSnapshot` 拍成完整 JSON
///    包含 command_id/output/is_alt_screen_active)
/// 3. 不识别的 variant 返回 `None`,调用方 fallback 到 Display
///
/// ## 维护注意
///
/// 新增 BYOP tool 时,**这里的 enum match 必须同步加 variant**,否则该 tool 的
/// 当前轮 ActionResult 会 fallback 到 Display,丢失结构化字段。
pub fn serialize_action_result(action: &AIAgentActionResult) -> Option<String> {
    use api::message::tool_call_result::Result as MsgR;
    use api::request::input::tool_call_result::Result as ReqR;
    use api::request::input::user_inputs::user_input::Input;

    let input: Input = action.clone().try_into().ok()?;
    let req_input: ReqR = match input {
        Input::ToolCallResult(tcr) => tcr.result?,
        _ => return None,
    };
    let msg_side = match req_input {
        ReqR::RunShellCommand(r) => MsgR::RunShellCommand(r),
        ReqR::WriteToLongRunningShellCommand(r) => MsgR::WriteToLongRunningShellCommand(r),
        ReqR::ReadShellCommandOutput(r) => MsgR::ReadShellCommandOutput(r),
        ReqR::ReadFiles(r) => MsgR::ReadFiles(r),
        ReqR::Grep(r) => MsgR::Grep(r),
        ReqR::FileGlobV2(r) => MsgR::FileGlobV2(r),
        ReqR::ApplyFileDiffs(r) => MsgR::ApplyFileDiffs(r),
        ReqR::SearchCodebase(r) => MsgR::SearchCodebase(r),
        ReqR::CallMcpTool(r) => MsgR::CallMcpTool(r),
        ReqR::ReadMcpResource(r) => MsgR::ReadMcpResource(r),
        ReqR::AskUserQuestion(r) => MsgR::AskUserQuestion(r),
        ReqR::ReadSkill(r) => MsgR::ReadSkill(r),
        _ => return None,
    };

    for t in REGISTRY {
        if let Some(json) = (t.result_to_json)(&msg_side) {
            return Some(serde_json::to_string(&json).unwrap_or_else(|_| "{}".to_owned()));
        }
    }
    if let Some(json) = mcp::serialize_result(&msg_side) {
        return Some(serde_json::to_string(&json).unwrap_or_else(|_| "{}".to_owned()));
    }
    None
}

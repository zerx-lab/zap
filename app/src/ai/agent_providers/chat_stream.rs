//! BYOP 模式下 chat completion + tool calling 适配层(基于 genai 0.5.3)。
//!
//! 把 `RequestParams` 翻译为 genai `ChatRequest`,通过 `Client::exec_chat_stream`
//! 调用用户配置的 provider,响应翻译回 `warp_multi_agent_api::ResponseEvent`,
//! controller 自家逻辑(权限/弹窗/执行/result 回写/触发下一轮)接管闭环。
//!
//! ## 5 种 API 协议显式路由
//!
//! 不再把所有 provider 当作 OpenAI 兼容硬塞,通过 `ServiceTargetResolver` 把
//! 用户在 settings UI 选的 `AgentProviderApiType` 一对一映射到 genai 的 `AdapterKind`:
//!
//! | ApiType        | AdapterKind  | 默认 endpoint                                  |
//! |----------------|--------------|------------------------------------------------|
//! | OpenAi         | OpenAI       | https://api.openai.com/v1                      |
//! | OpenAiResp     | OpenAIResp   | https://api.openai.com/v1 (走 /v1/responses)   |
//! | Gemini         | Gemini       | https://generativelanguage.googleapis.com/v1beta |
//! | Anthropic      | Anthropic    | https://api.anthropic.com                      |
//! | Ollama         | Ollama       | http://localhost:11434                         |
//!
//! 用户填的 `base_url` 始终覆盖默认。这样:
//! - DeepSeek / SiliconFlow / OpenRouter 等 OpenAI 兼容 provider 选 `OpenAi`,自定义 base_url
//! - 显式选定 adapter 完全绕过 genai 的"按模型名识别"默认行为,避免误识别
//!
//! ## 多轮 message 转换
//!
//! - system prompt: `ChatRequest::with_system()`(不进 messages 数组)
//! - user query: `ChatMessage::user(text)`
//! - assistant text: `ChatMessage::assistant(text)`
//! - assistant tool_calls: `ChatMessage::from(Vec<ToolCall>)`(自动 assistant role)
//! - tool result: `ChatMessage::from(ToolResponse::new(call_id, content))`(自动 tool role)
//!
//! ## 流式实现
//!
//! `Client::exec_chat_stream` 返回 `ChatStreamResponse`,其 `stream` 字段实现了
//! `futures_core::Stream<Item = Result<ChatStreamEvent>>`。事件:
//! - `Start` / `Chunk(text)` / `ReasoningChunk(text)` / `ToolCallChunk(tool_call)` / `End(StreamEnd)`
//!
//! 我们对 Chunk/ReasoningChunk 立即 emit `AppendToMessageContent`(打字机效果),
//! 对 ToolCallChunk 累积 buffer(按 call_id),流末统一 emit `Message::ToolCall`,
//! controller 自动接管。

use std::collections::HashMap;
use std::sync::Arc;

use futures::StreamExt;
use serde_json::{json, Value};
use uuid::Uuid;
use warp_multi_agent_api as api;

use genai::adapter::AdapterKind;
use genai::chat::{
    ChatMessage, ChatOptions, ChatRequest, ChatStreamEvent, ContentPart, MessageContent,
    Tool as GenaiTool, ToolCall, ToolResponse,
};
use genai::resolver::{AuthData, Endpoint, ServiceTargetResolver};
use genai::{Client, ModelIden, ServiceTarget};

use crate::ai::agent::api::{RequestParams, ResponseStream};
use crate::ai::agent::{AIAgentInput, RunningCommand};
use crate::server::server_api::AIApiError;
use crate::settings::AgentProviderApiType;
use ai::agent::convert::ConvertToAPITypeError;

use super::openai_compatible::OpenAiCompatibleError;
use super::tools;

// ---------------------------------------------------------------------------
// System prompt
// ---------------------------------------------------------------------------
// system prompt 由 `prompt_renderer::render_system` 通过 minijinja 模板生成,
// 按 LLMId 模型族选 system/{anthropic,gpt,beast,gemini,kimi,codex,trinity,default}.j2,
// 并把 warp 客户端已经收集好的 AIAgentContext(env / git / skills / project_rules / codebase / current_time)
// 渲染进 system,让 BYOP 路径也能拥有跟 warp 自家路径相当的环境信息。

use super::prompt_renderer;
use super::user_context;
use crate::ai::agent::AIAgentContext;

/// 从 input 中抽出最近一条 `UserQuery.context`(等价 warp `convert_to.rs::convert_input` 取的那条)。
fn latest_input_context(input: &[AIAgentInput]) -> &[AIAgentContext] {
    for i in input.iter().rev() {
        if let Some(ctx) = i.context() {
            return ctx;
        }
    }
    &[]
}

/// LRC tag-in 场景下渲染 `<attached_running_command>` XML 块,prepend 到 user message,
/// 让模型看到当前 PTY 的实际状态(命令、grid 内容、是否 alt-screen),从而正确选择
/// `write_to_long_running_shell_command` 工具发送对应键序列。
fn render_running_command_context(rc: &RunningCommand) -> String {
    format!(
        "<attached_running_command command_id=\"{}\" is_alt_screen_active=\"{}\">\n  \
         <command>{}</command>\n  \
         <snapshot>\n{}\n  </snapshot>\n  \
         <instructions>This command is already running in the user's terminal. \
         Use `read_shell_command_output` with this command_id to inspect it, and \
         `write_to_long_running_shell_command` with this command_id to operate the program \
         through its PTY (in raw mode, use tokens like `<ESC>` and `<ENTER>` for control \
         keys). This command_id is valid even if the process was started by the user \
         rather than by run_shell_command. Do NOT spawn a new shell to control the same TUI.\
         </instructions>\n\
         </attached_running_command>",
        xml_attr(rc.block_id.as_str()),
        rc.is_alt_screen_active,
        xml_text(&rc.command),
        xml_text(&rc.grid_contents),
    )
}

/// 简短回退版本:仅有 command_id(没拿到 RunningCommand 完整快照时),
/// 让模型至少知道目标 PTY 的 id,可以用 `read_shell_command_output` 自己取最新内容。
fn render_running_command_id_context(command_id: &str) -> String {
    format!(
        "<attached_running_command command_id=\"{}\">\n  \
         <instructions>This command is already running in the user's terminal. \
         Use `read_shell_command_output` with this command_id to inspect it, and \
         `write_to_long_running_shell_command` with this command_id to operate the program \
         through its PTY. Do NOT spawn a new shell to control the same TUI.</instructions>\n\
         </attached_running_command>",
        xml_attr(command_id),
    )
}

fn render_lrc_request_context(params: &RequestParams) -> Option<String> {
    params
        .lrc_running_command
        .as_ref()
        .map(render_running_command_context)
        .or_else(|| {
            params
                .lrc_command_id
                .as_deref()
                .map(render_running_command_id_context)
        })
}

/// OpenWarp:渲染 SSH 会话状态块,append 到 system prompt 末尾。
///
/// 触发条件:`SessionContext.is_legacy_ssh()` 为 true(用户在本地 PTY 手敲
/// `ssh xx@xx` 进入远端,远端没装 warp shell hook)。这种会话:
/// - `session_type` 仍是 `Local`
/// - 整段 system prompt 的 [Environment] 区块描述的是**本地客户端** OS / shell,
///   而 PTY 当前实际跑在**远端**
///
/// 不主动告知模型这一点,LLM 会按 system prompt 里的本地 OS 推断"目标在远端,
/// 我得先 ssh 过去",于是输出 `ssh xx@xx uname -a` 这种二次嵌套命令。
///
/// 注:warpified SSH(`SessionType::WarpifiedRemote`)不在这里处理 — 那条路径
/// 远端 shell hook 已重新 bootstrap,host_info / shell 都是远端真值,prompt 本身就对。
fn render_ssh_session_block(
    session_context: &crate::ai::blocklist::SessionContext,
) -> Option<String> {
    if !session_context.is_legacy_ssh() {
        return None;
    }
    let info = session_context.ssh_connection_info();
    let host = info
        .and_then(|i| i.host.as_deref())
        .map(xml_attr)
        .unwrap_or_else(|| "unknown".to_owned());
    let port = info
        .and_then(|i| i.port.as_deref())
        .map(xml_attr)
        .unwrap_or_else(|| "22".to_owned());

    Some(format!(
        "\n\n<ssh_session host=\"{host}\" port=\"{port}\">\n  \
         <fact>The active terminal PTY is currently inside an SSH session opened by the user from their local machine. \
         All shell commands you run via `run_shell_command` execute on the REMOTE host, not on the local client.</fact>\n  \
         <warning>The [Environment] block (OS / shell / working directory) above describes the LOCAL client and may not match the remote host. \
         If you need precise remote info, probe it directly (e.g. `uname -a`, `cat /etc/os-release`, `pwd`).</warning>\n  \
         <rules>\n    \
         - Run commands DIRECTLY (e.g. `uname -a`, `ls /`). Do NOT prepend `ssh {host} ...` — that opens a NESTED ssh session inside the current one.\n    \
         - Treat the working directory and home directory shown above with skepticism; they may reflect the local client.\n    \
         - When LRC tag-in mode is active (an `<attached_running_command>` block is present), prefer `write_to_long_running_shell_command` with that command_id to inject keystrokes into this same remote PTY. Spawning a new shell would create a separate local-side ssh client, not interact with the remote process the user is watching.\n  \
         </rules>\n\
         </ssh_session>"
    ))
}

/// XML 转义,同时 strip 所有非法/有问题的控制字符,避免 JSON 序列化失败。
///
/// `grid_contents`(从 `formatted_terminal_contents_for_input` 提取的 alt-screen 内容)
/// 可能含原始 ANSI escape 序列(0x1b)、CSI sequences、SGR codes、box-drawing chars 等。
/// 其中 `< 0x20` 的控制字符会让 Anthropic 解析 JSON 报 "invalid escaped character in string",
/// 因为 JSON RFC 7159 只接受 `\b \f \n \r \t \" \\ \/ \uXXXX` 这几种合法转义,
/// 其他 `\v` `\a` `\x1b` 之类直接 reject。
///
/// 处理:
/// - `\n` `\r` `\t` 保留(JSON 合法)
/// - 其它 `< 0x20` 控制字符替换成空格(纯展示给模型,不需要保留 ANSI 颜色等)
/// - `&` `<` `>` 转 XML entity
fn xml_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\n' | '\r' | '\t' => out.push(c),
            c if (c as u32) < 0x20 => out.push(' '),
            // DEL(0x7f)单独处理 — 也是控制字符
            '\u{7f}' => out.push(' '),
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
    out
}

fn xml_attr(s: &str) -> String {
    xml_text(s).replace('"', "&quot;")
}

// ---------------------------------------------------------------------------
// Multi-turn message 转换
// ---------------------------------------------------------------------------

/// 累积同一 assistant turn 的 text + tool_calls + reasoning,然后 flush 成一个或两个
/// `ChatMessage`(text 一个,tool_calls 一个 — genai 把它们建模为分开的 message)。
///
/// **thinking-mode reasoning_content 回传**:对 DeepSeek / Kimi 这类要求"每条
/// assistant 都必须回传 reasoning_content 字段"的 adapter,buf 持有 `force_echo=true`,
/// flush 时即使本 turn 没攒到 reasoning 也会挂一个非空占位 — genai 序列化层
/// (`adapter_shared.rs:368-373`) 只看 `ContentPart::ReasoningContent` 是否存在。
/// 其他 adapter(Anthropic / Gemini)`force_echo=false`,行为退化为旧逻辑(reasoning 非空才挂)。
const REASONING_ECHO_PLACEHOLDER: &str = " ";

#[derive(Default)]
struct AssistantBuffer {
    text: Option<String>,
    tool_calls: Vec<ToolCall>,
    /// 上一轮 AgentReasoning(thinking 链)。flush 时挂到对应 assistant message
    /// 的 reasoning_content 字段(genai 内部按 adapter 序列化:DeepSeek/Kimi 走 reasoning_content,
    /// Anthropic 走 thinking blocks)。
    reasoning: Option<String>,
    /// thinking-mode adapter 强制回传 reasoning_content(非空占位)。由
    /// `super::reasoning::model_requires_reasoning_echo` 决定。
    force_echo_reasoning: bool,
}

impl AssistantBuffer {
    fn new(force_echo_reasoning: bool) -> Self {
        Self {
            force_echo_reasoning,
            ..Default::default()
        }
    }

    fn flush_into(&mut self, messages: &mut Vec<ChatMessage>) {
        let reasoning = self.reasoning.take();
        let has_tool_calls = !self.tool_calls.is_empty();
        // 决定本次 flush 要挂到 tool_calls assistant 上的 reasoning 字符串:
        // - 有真实 reasoning 文本 → 用之
        // - 没有 + force_echo → 非空占位(满足 DeepSeek/Kimi 服务端"字段必须存在"的校验)
        // - 没有 + 不 force_echo → None(不挂字段,跟旧行为一致)
        let echo_reasoning: Option<String> = match reasoning {
            Some(r) if !r.is_empty() => Some(r),
            _ if self.force_echo_reasoning => Some(REASONING_ECHO_PLACEHOLDER.to_owned()),
            _ => None,
        };
        if let Some(t) = self.text.take() {
            let mut msg = ChatMessage::assistant(t);
            if has_tool_calls {
                // DeepSeek thinking mode 要求每条 assistant message 都带
                // reasoning_content。text + tool_calls 被 genai 建模成两条
                // assistant 时,text 这条也必须补占位。
                if self.force_echo_reasoning {
                    msg = msg.with_reasoning_content(Some(REASONING_ECHO_PLACEHOLDER.to_owned()));
                }
            } else if let Some(r) = echo_reasoning.clone() {
                msg = msg.with_reasoning_content(Some(r));
            }
            messages.push(msg);
        }
        if has_tool_calls {
            // genai `From<Vec<ToolCall>> for ChatMessage` 自动产 assistant role +
            // MessageContent::from_tool_calls。
            let mut msg = ChatMessage::from(std::mem::take(&mut self.tool_calls));
            if let Some(r) = echo_reasoning {
                msg = msg.with_reasoning_content(Some(r));
            }
            messages.push(msg);
        }
    }
}

/// 把 RequestParams 翻译为 genai `ChatRequest`(含 system + messages + tools)。
///
/// `force_echo_reasoning`:由 `super::reasoning::model_requires_reasoning_echo`
/// 决定。true 时所有 assistant message 强制挂 reasoning_content(空串占位),
/// 修复 DeepSeek-v4-flash / Kimi 等收紧校验的 thinking-mode endpoint。
fn build_chat_request(params: &RequestParams, force_echo_reasoning: bool) -> ChatRequest {
    let agent_ctx = latest_input_context(&params.input);
    let mut system_text = prompt_renderer::render_system(&params.model, agent_ctx);
    // OpenWarp:legacy SSH 会话画像补丁。`render_system` 走 AIAgentContext,
    // 拿到的 OS/shell 是本地客户端;legacy SSH 下 PTY 实际在远端,
    // 追加一段 SSH 状态块矫正 LLM 推断。
    if let Some(ssh_block) = render_ssh_session_block(&params.session_context) {
        system_text.push_str(&ssh_block);
    }
    // 注:LRC / 长命令的工具用法引导(write_to_long_running_shell_command + command_id +
    // 各种 mode 与 raw 字节序列)已经在 `prompts/system/default.j2:69-79` 完整覆盖。
    // 用户当前所处的具体 PTY 上下文(命令名 / alt-screen 标志 / grid 内容)通过
    // user message 前缀的 `<attached_running_command>` XML 块单独注入(见
    // `render_running_command_context` 与 build_chat_request 中的 UserQuery 分支)。
    // 不在 system 这层重复硬编码 TUI 退出键之类,避免与 default.j2 的标准引导冲突或冗余。

    let mut messages: Vec<ChatMessage> = Vec::new();

    // 收集所有 task 的 messages,按时间戳排序。
    let mut all_msgs: Vec<&api::Message> = params
        .tasks
        .iter()
        .flat_map(|t| t.messages.iter())
        .collect();
    all_msgs.sort_by_key(|m| {
        m.timestamp
            .as_ref()
            .map(|ts| (ts.seconds, ts.nanos))
            .unwrap_or((0, 0))
    });

    let mut buf = AssistantBuffer::new(force_echo_reasoning);
    // OpenWarp:历史里被 skip 掉的 subagent ToolCall 对应的 call_id —— 它们的
    // ToolCallResult 也必须 skip,否则会成为孤儿 tool_response,Anthropic 直接 400
    // `unexpected tool_use_id ... no corresponding tool_use block`。
    let mut skipped_subagent_call_ids: std::collections::HashSet<String> =
        std::collections::HashSet::new();

    for msg in all_msgs {
        let Some(inner) = &msg.message else {
            continue;
        };
        match inner {
            api::message::Message::UserQuery(u) => {
                buf.flush_into(&mut messages);
                // 历史轮的 user query 没有 AIAgentContext 数据(InputContext 在 warp 协议
                // 是单 Request 级 payload,不持久化到 message),只送 query 文本。
                // 这跟 warp 自家路径行为一致 — 历史轮不重发附件 context。
                messages.push(ChatMessage::user(u.query.clone()));
            }
            api::message::Message::AgentReasoning(r) => {
                // 把上一轮的 reasoning 挂到下一个要 flush 的 assistant message 上。
                // genai 0.6 的 with_reasoning_content 会按当前 adapter 序列化:
                // DeepSeek/Kimi → reasoning_content 字段;Anthropic → thinking blocks。
                // 多段 AgentReasoning 累加(同一 turn 可能 stream 出多个 reasoning chunk
                // 落地为多条 AgentReasoning)。
                let next = r.reasoning.clone();
                if !next.is_empty() {
                    match buf.reasoning.as_mut() {
                        Some(existing) => existing.push_str(&next),
                        None => buf.reasoning = Some(next),
                    }
                }
            }
            api::message::Message::AgentOutput(a) => {
                if buf.text.is_some() || !buf.tool_calls.is_empty() {
                    buf.flush_into(&mut messages);
                }
                buf.text = Some(a.text.clone());
            }
            api::message::Message::ToolCall(tc) => {
                // OpenWarp BYOP:**虚拟 subagent tool_call 不发给上游模型**。
                // LRC tag-in 场景下,我们在 chat_stream 流头合成 `Tool::Subagent { metadata: Cli }`
                // 写入 root.task.messages,只用于触发 conversation 创建 cli subtask + spawn 浮窗,
                // 它不是模型实际产出的工具调用,模型看到会 confused(多余 tool call + 没法回应)。
                // 同样它对应的 placeholder ToolResponse(由 sanitize_tool_call_pairs 补的)
                // 也要由下面 ToolCallResult 分支的 skip 逻辑配合过滤,避免出现
                // "tool_response 找不到匹配的 tool_call" 的不平衡。
                use crate::ai::agent::task::helper::ToolCallExt;
                if tc.subagent().is_some() {
                    skipped_subagent_call_ids.insert(tc.tool_call_id.clone());
                    continue;
                }
                let (name, args_json) = serialize_outgoing_tool_call(
                    tc,
                    params.mcp_context.as_ref(),
                    &msg.server_message_data,
                );
                buf.tool_calls.push(ToolCall {
                    call_id: tc.tool_call_id.clone(),
                    fn_name: name,
                    fn_arguments: args_json,
                    thought_signatures: None,
                });
            }
            api::message::Message::ToolCallResult(tcr) => {
                buf.flush_into(&mut messages);
                // OpenWarp:对应 ToolCall 已被 skip(subagent 虚拟 call)→ result 也 skip,
                // 否则留下孤儿 tool_response 导致上游 400。
                if skipped_subagent_call_ids.contains(&tcr.tool_call_id) {
                    continue;
                }
                // BYOP 持久化的 ToolCallResult 走 server_message_data(content 已是 JSON 字符串);
                // server 端 emit 走 result oneof 结构化 variant — 兼容两路。
                let content = if tcr.result.is_some() {
                    tools::serialize_result(tcr)
                } else if !msg.server_message_data.is_empty() {
                    msg.server_message_data.clone()
                } else {
                    r#"{"status":"empty"}"#.to_owned()
                };
                messages.push(ChatMessage::from(ToolResponse::new(
                    tcr.tool_call_id.clone(),
                    content,
                )));
            }
            _ => {
                // 其他 message 类型(SystemQuery/UpdateTodos/...)BYOP 暂不送上游。
            }
        }
    }
    buf.flush_into(&mut messages);

    // 当前轮新输入 → 追加。
    for input in &params.input {
        match input {
            AIAgentInput::UserQuery {
                query,
                context,
                running_command,
                ..
            } => {
                // 当前轮 UserQuery 自带的附件类 context(Block / SelectedText / File / Image)
                // 严格对齐 warp 自家路径走 `api::InputContext.executed_shell_commands` 等字段
                // 上行后由后端注入 prompt 的语义。BYOP 没有后端这层,直接 prepend 到 user message。
                // 环境型 context(env / git / skills / ...)由 prompt_renderer 渲染进 system,
                // 与本路径不重叠。
                //
                // OpenWarp:LRC tag-in 场景下,`running_command: Some(...)` 含完整 PTY 上下文
                // (alt-screen grid_contents + command + is_alt_screen_active 标志),用
                // `render_running_command_context` 渲成 `<attached_running_command>` XML 块
                // prepend 到 user message,模型据此决定调 write_to_long_running_shell_command。
                // 没填(普通对话或 controller 没注入)时回退到 `lrc_command_id` 简短上下文。
                let mut prefixes: Vec<String> = Vec::new();
                let request_running_command = running_command
                    .as_ref()
                    .or(params.lrc_running_command.as_ref());
                if let Some(rc) = request_running_command {
                    prefixes.push(render_running_command_context(rc));
                } else if let Some(command_id) = params.lrc_command_id.as_deref() {
                    prefixes.push(render_running_command_id_context(command_id));
                }
                if let Some(attachments_prefix) = user_context::render_user_attachments(context) {
                    prefixes.push(attachments_prefix);
                }
                let full_text = if prefixes.is_empty() {
                    query.clone()
                } else {
                    format!("{}\n\n{query}", prefixes.join("\n\n"))
                };
                log::info!(
                    "[byop-diag] build_chat_request UserQuery: query_len={} \
                     running_command={} prefixes={} full_text_len={}",
                    query.len(),
                    match request_running_command {
                        Some(rc) => format!(
                            "Some(grid_len={} alt={})",
                            rc.grid_contents.len(),
                            rc.is_alt_screen_active
                        ),
                        None => "None".to_owned(),
                    },
                    prefixes.len(),
                    full_text.len()
                );
                messages.push(ChatMessage::user(full_text));
            }
            AIAgentInput::ActionResult { result, .. } => {
                // 上一轮模型回了 tool_calls,client 端执行完后 result 走 `params.input`
                // 而不是 `params.tasks` 历史。必须在这里序列化为 ToolResponse,否则
                // genai/上游会因 tool_call_id 配对失败 400。
                let tool_call_id = result.id.to_string();
                let content = tools::serialize_action_result(result).unwrap_or_else(|| {
                    serde_json::json!({ "result": result.result.to_string() }).to_string()
                });
                messages.push(ChatMessage::from(ToolResponse::new(tool_call_id, content)));
            }
            AIAgentInput::InvokeSkill {
                skill, user_query, ..
            } => {
                let mut composed = format!(
                    "请按下面的技能 \"{}\" 指引执行任务:\n\n{}\n\n---\n",
                    skill.name, skill.content,
                );
                if let Some(uq) = user_query {
                    composed.push_str(&format!("用户进一步指令: {}", uq.query));
                }
                messages.push(ChatMessage::user(composed));
            }
            AIAgentInput::ResumeConversation { context } => {
                // BYOP 没有 server 端 resume prompt 注入层。LRC auto-resume 时必须显式
                // 重带当前 PTY 上下文,否则错误恢复轮会退化成普通对话并重新选择 shell 工具。
                let mut prefixes: Vec<String> = Vec::new();
                if let Some(lrc_prefix) = render_lrc_request_context(params) {
                    prefixes.push(lrc_prefix);
                }
                if let Some(attachments_prefix) = user_context::render_user_attachments(context) {
                    prefixes.push(attachments_prefix);
                }
                if !prefixes.is_empty() {
                    messages.push(ChatMessage::user(format!(
                        "{}\n\nContinue.",
                        prefixes.join("\n\n")
                    )));
                }
            }
            AIAgentInput::AutoCodeDiffQuery { .. }
            | AIAgentInput::CreateNewProject { .. }
            | AIAgentInput::CodeReview { .. }
            | AIAgentInput::SummarizeConversation { .. } => {
                // 暂时忽略
            }
            _ => {}
        }
    }

    // 防御性 sanitize: 确保每个 assistant tool_calls 后面跟着对应每个 call_id 的
    // ToolResponse。warp 自家协议有时把 tool result 消化成下一轮 AgentOutput,
    // 上游若未保留 ToolCallResult,会让 tool_calls 配对失败。
    sanitize_tool_call_pairs(&mut messages);

    // 防御性 sanitize: 确保 messages 末尾不是 assistant。
    // Anthropic / 部分网关不接受末尾为 assistant 的请求(prefill 仅特定模型支持),
    // 而 warp 的 `AIAgentInput::ResumeConversation`(handoff/auto-resume after error 等)
    // 不附加新 user 消息,会让序列末尾停在历史 assistant 上。
    // 这里统一兜底:末尾若是 assistant,追加一条隐式 user 消息让上游继续。
    ensure_ends_with_user(&mut messages);

    let tools_array = build_tools_array(params);

    // OpenWarp:整体 sanitize system / messages / tools 中所有会进入 JSON body 的字符串,
    // 移除 < 0x20 / DEL 控制字符(除 \n \r \t),并把 `\xNN` 这类危险字面量替换为
    // 普通文字,避免 Anthropic 或中间代理把它们误当成 JSON escape 后 400。
    // nvim 等 alt-screen TUI 的 grid_contents、tool result、工具描述和 schema description
    // 都可能带这些片段,所以不能只清理 user message 的 first_text。
    let system_text = sanitize_text_for_json(&system_text);
    let messages: Vec<ChatMessage> = messages
        .into_iter()
        .map(sanitize_chat_message_for_request)
        .collect();
    let tools_array: Vec<GenaiTool> = tools_array
        .into_iter()
        .map(sanitize_tool_for_request)
        .collect();

    let mut req = ChatRequest::from_messages(messages).with_system(system_text);
    if !tools_array.is_empty() {
        req = req.with_tools(tools_array);
    }
    req
}

/// 移除字符串中所有可能让 JSON 序列化产生非法转义的字符:
/// - 所有 ASCII 控制字符替换成空格,包括换行、回车和 tab
/// - DEL(0x7f)替换成空格
/// - 反斜杠替换成 `/`,双引号替换成单引号
///
/// 用途:防 ANSI escape 序列、Windows 路径、换行、字符串内引号等内容透到 BYOP 请求体。
/// 标准 JSON 允许这些 escape,但部分 Anthropic 兼容代理会在转发时把 escape
/// 处理坏并返回 `invalid escaped character in string`,因此这里统一压平成安全字符。
fn sanitize_text_for_json(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            c if (c as u32) < 0x20 => out.push(' '),
            '\u{7f}' => out.push(' '),
            '\\' => out.push('/'),
            '"' => out.push('\''),
            _ => out.push(c),
        }
    }
    replace_dangerous_escape_literals(out)
}

fn replace_dangerous_escape_literals(mut text: String) -> String {
    for (from, to) in [
        ("\\n", " "),
        ("\\r", " "),
        ("\\t", " "),
        ("\\x1b", "ESC"),
        ("\\x1B", "ESC"),
        ("\\x03", "Ctrl-C"),
        ("\\x04", "Ctrl-D"),
        ("\\x07", "BEL"),
        ("\\a", "BEL"),
        ("\\v", "vertical tab"),
    ] {
        text = text.replace(from, to);
    }
    text
}

fn sanitize_json_value_for_request(value: Value) -> Value {
    match value {
        Value::String(s) => Value::String(sanitize_text_for_json(&s)),
        Value::Array(values) => Value::Array(
            values
                .into_iter()
                .map(sanitize_json_value_for_request)
                .collect(),
        ),
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(key, value)| (key, sanitize_json_value_for_request(value)))
                .collect(),
        ),
        other => other,
    }
}

fn sanitize_chat_message_for_request(mut message: ChatMessage) -> ChatMessage {
    let parts = message
        .content
        .into_parts()
        .into_iter()
        .map(|part| match part {
            ContentPart::Text(text) => ContentPart::Text(sanitize_text_for_json(&text)),
            // ToolResponse.content 与 ToolCall.fn_arguments 本身就是
            // `serde_json::to_string` / `serde_json::json!` 产出的合法 JSON,
            // 让模型按 JSON 协议解析。再过一遍 sanitize_text_for_json 会把
            // `"` → `'`、`\` → `/`、控制字符压平,把合法 JSON 变成 Python-like
            // repr,模型彻底无法解析 retry 提示,陷入死循环改格式。
            // sanitize 仅对 prose(Text / Reasoning / ThoughtSignature)生效,
            // 结构化字段一律直通。
            ContentPart::ToolResponse(response) => ContentPart::ToolResponse(response),
            ContentPart::ToolCall(call) => ContentPart::ToolCall(call),
            ContentPart::ThoughtSignature(signature) => {
                ContentPart::ThoughtSignature(sanitize_text_for_json(&signature))
            }
            ContentPart::ReasoningContent(reasoning) => {
                ContentPart::ReasoningContent(sanitize_text_for_json(&reasoning))
            }
            ContentPart::Custom(mut custom) => {
                custom.data = sanitize_json_value_for_request(custom.data);
                ContentPart::Custom(custom)
            }
            other => other,
        })
        .collect::<Vec<_>>();
    message.content = MessageContent::from_parts(parts);
    message
}

fn sanitize_tool_for_request(mut tool: GenaiTool) -> GenaiTool {
    tool.description = tool
        .description
        .map(|description| sanitize_text_for_json(&description));
    tool.schema = tool.schema.map(sanitize_json_value_for_request);
    tool
}

/// 重排 messages 中所有 Tool 消息,确保:
/// 每个含 tool_calls 的 Assistant message 后面紧跟**且仅紧跟**一条 Tool message,
/// 内含该 Assistant **每个** call_id 的 ToolResponse(按 tool_calls 顺序,缺失补 placeholder)。
///
/// **为什么需要重排,而不是只补 placeholder / 剔孤儿**:
/// `build_chat_request` 按时间戳 chronological 排序合并所有 task 的历史 messages。
/// 当模型在一轮内发起多个 tool_call,且这些 tool 的执行时长差异较大时(如 read_skill
/// 立即返回错误,而 git/PowerShell 命令稍慢),后到的 ToolCallResult 时间戳可能晚于
/// 模型紧接着发起的**下一轮** Assistant tool_calls,导致历史 messages 被排成:
///
/// ```text
/// Asst-X(tc_a, tc_b, tc_c)
/// Tool(tc_c real)         ← read_skill 错误,快
/// Asst-Y(tc_d, tc_e)      ← 模型基于 tc_c 错误立刻发了下一轮
/// Tool(tc_a real)         ← git 命令慢,落到 Asst-Y 后面
/// Tool(tc_b real)
/// ```
///
/// Anthropic API 把连续 Tool block 合并视作"上一条 Assistant 的 tool_results",
/// 于是 Asst-Y 后面的 Tool block 含 tc_a/tc_b 这种 Asst-Y 不认识的 call_id → 400
/// `unexpected tool_use_id ... no corresponding tool_use block in the previous message`。
///
/// 旧实现只做"剔孤儿(整序列匹配)+补 placeholder(向前看相邻 Tool)",前者不会剔除
/// 这类**位置错误但 call_id 合法**的 ToolResponse,后者也不会重定位 — 所以 400 重现。
///
/// 新实现:抽出所有 ToolResponse 进 `call_id → response` 表,然后按每个 Assistant
/// tool_calls 的 call_id 顺序重新组装紧随其后的 Tool message。剩余未消费的 ToolResponse
/// (call_id 完全不在历史 Assistant tool_calls 里)即真孤儿,丢弃。
fn sanitize_tool_call_pairs(messages: &mut Vec<ChatMessage>) {
    use std::collections::HashMap;

    // 抽取所有 ToolResponse(同 call_id 后到的覆盖前面,符合"最新结果优先"语义)。
    let mut response_by_call_id: HashMap<String, ToolResponse> = HashMap::new();
    let original = std::mem::take(messages);
    let mut non_tool_msgs: Vec<ChatMessage> = Vec::with_capacity(original.len());
    for msg in original {
        if msg.role == genai::chat::ChatRole::Tool {
            for resp in msg.content.tool_responses() {
                response_by_call_id.insert(resp.call_id.clone(), (*resp).clone());
            }
        } else {
            non_tool_msgs.push(msg);
        }
    }

    // 重组:每个 Assistant 含 tool_calls 后紧跟一条 Tool message,按 call_id 顺序绑定。
    let mut placeholders_inserted: Vec<String> = Vec::new();
    for msg in non_tool_msgs {
        let call_ids: Vec<String> = msg
            .content
            .tool_calls()
            .iter()
            .map(|tc| tc.call_id.clone())
            .collect();
        let is_assistant = msg.role == genai::chat::ChatRole::Assistant;
        messages.push(msg);

        if is_assistant && !call_ids.is_empty() {
            let bundled: Vec<ToolResponse> = call_ids
                .iter()
                .map(|cid| {
                    response_by_call_id.remove(cid).unwrap_or_else(|| {
                        placeholders_inserted.push(cid.clone());
                        ToolResponse::new(cid.clone(), "(tool 执行结果未保留)".to_owned())
                    })
                })
                .collect();
            messages.push(ChatMessage::from(bundled));
        }
    }

    // 剩余 response_by_call_id 是真孤儿(没有任何 Assistant tool_call 与之配对),丢弃。
    if !response_by_call_id.is_empty() {
        let orphan_ids: Vec<&String> = response_by_call_id.keys().collect();
        log::warn!(
            "[byop-diag] sanitize_tool_call_pairs: 丢弃 {} 个孤儿 ToolResponse: \
             orphan_call_ids={:?}",
            response_by_call_id.len(),
            orphan_ids
        );
    }
    if !placeholders_inserted.is_empty() {
        log::warn!(
            "[byop-diag] sanitize_tool_call_pairs: 给 {} 个 ToolCall 补 placeholder \
             ToolResponse: missing_call_ids={:?}",
            placeholders_inserted.len(),
            placeholders_inserted
        );
    }
}

/// 兜底:确保 messages 末尾是 user(或 tool 响应)。
///
/// 触发场景:`AIAgentInput::ResumeConversation` 不附加新 user 消息,直接重发历史。
/// Anthropic 原生 API 拒绝末尾为 assistant 的请求(`This model does not support
/// assistant message prefill. The conversation must end with a user message.`),
/// 重试 3 次都同 payload → UI 渲染 error block 触发 flex panic。
///
/// 末尾是 assistant 时追加 `ChatMessage::user("Continue.")`,提示模型继续即可。
/// Tool 角色作为 user 输入的一种(模型会把 tool 响应当作下一轮起点)不动。
/// 空 messages 不触发,避免给空对话凭空塞内容。
fn ensure_ends_with_user(messages: &mut Vec<ChatMessage>) {
    use genai::chat::ChatRole;
    if let Some(last) = messages.last() {
        if last.role == ChatRole::Assistant {
            messages.push(ChatMessage::user("Continue."));
        }
    }
}

/// 反向: 把内部 `tool_call::Tool` variant 序列化成 (function name, arguments JSON Value)
/// 用于多轮历史回放。这里的 (name, args) 必须与 `tools::REGISTRY` 中各 tool 的 `name`
/// 与 `from_args` 期望的 schema 严格对齐。
fn serialize_outgoing_tool_call(
    tc: &api::message::ToolCall,
    mcp_ctx: Option<&crate::ai::agent::MCPContext>,
    server_message_data: &str,
) -> (String, Value) {
    use api::message::tool_call::Tool;

    // BYOP from_args 解析失败 carrier 还原:由 make_tool_call_carrier_message 写入,
    // tool oneof = None,原始 `<fn_name>\n<args_str>` 编码在 server_message_data。
    // 必须在主 match 之前优先识别,否则会落到下面 None=>"warp_internal_empty",
    // 上游模型看到一个不存在的工具名会更困惑、也不知道是哪个 call 失败了。
    if tc.tool.is_none() {
        if let Some((fn_name, raw_args)) = server_message_data.split_once('\n') {
            if !fn_name.is_empty() {
                let args_value = serde_json::from_str(raw_args)
                    .unwrap_or_else(|_| Value::String(raw_args.to_owned()));
                return (fn_name.to_owned(), args_value);
            }
        }
    }

    // 大多数旧实现返回 (String, String);这里改成 (String, Value),把字符串再 parse 一次。
    let (name, args_str) = match &tc.tool {
        Some(Tool::CallMcpTool(c)) => tools::mcp::serialize_outgoing_call(c, mcp_ctx),
        Some(Tool::ReadMcpResource(r)) => tools::mcp::serialize_outgoing_read_resource(r, mcp_ctx),
        Some(Tool::RunShellCommand(c)) => (
            "run_shell_command".to_owned(),
            json!({
                "command": c.command,
                "is_read_only": c.is_read_only,
                "uses_pager": c.uses_pager,
                "is_risky": c.is_risky,
            })
            .to_string(),
        ),
        Some(Tool::ReadFiles(r)) => {
            let files: Vec<Value> = r
                .files
                .iter()
                .map(|f| {
                    json!({
                        "path": f.name,
                        "line_ranges": f.line_ranges.iter().map(|lr| json!({
                            "start": lr.start, "end": lr.end
                        })).collect::<Vec<_>>(),
                    })
                })
                .collect();
            (
                "read_files".to_owned(),
                json!({ "files": files }).to_string(),
            )
        }
        Some(Tool::Grep(g)) => (
            "grep".to_owned(),
            json!({ "queries": g.queries, "path": g.path }).to_string(),
        ),
        Some(Tool::SearchCodebase(s)) => (
            "search_codebase".to_owned(),
            json!({
                "query": s.query,
                "path_filters": s.path_filters,
                "codebase_path": s.codebase_path,
            })
            .to_string(),
        ),
        Some(Tool::AskUserQuestion(a)) => {
            let questions: Vec<Value> = a
                .questions
                .iter()
                .map(|q| {
                    let (options, recommended_index, multi_select, supports_other) =
                        match &q.question_type {
                            Some(
                                api::ask_user_question::question::QuestionType::MultipleChoice(mc),
                            ) => (
                                mc.options
                                    .iter()
                                    .map(|o| o.label.clone())
                                    .collect::<Vec<_>>(),
                                mc.recommended_option_index,
                                mc.is_multiselect,
                                mc.supports_other,
                            ),
                            None => (vec![], 0, false, false),
                        };
                    json!({
                        "question": q.question,
                        "options": options,
                        "recommended_index": recommended_index,
                        "multi_select": multi_select,
                        "supports_other": supports_other,
                    })
                })
                .collect();
            (
                "ask_user_question".to_owned(),
                json!({ "questions": questions }).to_string(),
            )
        }
        Some(Tool::FileGlobV2(g)) => (
            "file_glob".to_owned(),
            json!({
                "patterns": g.patterns,
                "search_dir": g.search_dir,
                "limit": g.max_matches,
            })
            .to_string(),
        ),
        Some(Tool::ApplyFileDiffs(a)) => {
            let mut operations: Vec<Value> = Vec::new();
            for d in &a.diffs {
                operations.push(json!({
                    "op": "edit",
                    "file_path": d.file_path,
                    "search": d.search,
                    "replace": d.replace,
                }));
            }
            for f in &a.new_files {
                operations.push(json!({
                    "op": "create",
                    "file_path": f.file_path,
                    "content": f.content,
                }));
            }
            for f in &a.deleted_files {
                operations.push(json!({
                    "op": "delete",
                    "file_path": f.file_path,
                }));
            }
            (
                "apply_file_diffs".to_owned(),
                json!({ "summary": a.summary, "operations": operations }).to_string(),
            )
        }
        Some(Tool::WriteToLongRunningShellCommand(w)) => {
            use api::message::tool_call::write_to_long_running_shell_command::mode::Mode as M;
            let mode = match w.mode.as_ref().and_then(|m| m.mode.as_ref()) {
                Some(M::Raw(_)) => "raw",
                Some(M::Block(_)) => "block",
                _ => "line",
            };
            (
                "write_to_long_running_shell_command".to_owned(),
                json!({
                    "command_id": w.command_id,
                    "input": String::from_utf8_lossy(&w.input).to_string(),
                    "mode": mode,
                })
                .to_string(),
            )
        }
        Some(Tool::ReadDocuments(r)) => {
            let docs: Vec<Value> = r
                .documents
                .iter()
                .map(|d| {
                    json!({
                        "document_id": d.document_id,
                        "line_ranges": d.line_ranges.iter().map(|lr| json!({
                            "start": lr.start, "end": lr.end
                        })).collect::<Vec<_>>(),
                    })
                })
                .collect();
            (
                "read_documents".to_owned(),
                json!({ "documents": docs }).to_string(),
            )
        }
        Some(Tool::EditDocuments(e)) => {
            let diffs: Vec<Value> = e
                .diffs
                .iter()
                .map(|d| {
                    json!({
                        "document_id": d.document_id,
                        "search": d.search,
                        "replace": d.replace,
                    })
                })
                .collect();
            (
                "edit_documents".to_owned(),
                json!({ "diffs": diffs }).to_string(),
            )
        }
        Some(Tool::CreateDocuments(c)) => {
            let new_documents: Vec<Value> = c
                .new_documents
                .iter()
                .map(|d| json!({ "title": d.title, "content": d.content }))
                .collect();
            (
                "create_documents".to_owned(),
                json!({ "new_documents": new_documents }).to_string(),
            )
        }
        Some(Tool::SuggestNewConversation(s)) => (
            "suggest_new_conversation".to_owned(),
            json!({ "message_id": s.message_id }).to_string(),
        ),
        Some(Tool::SuggestPrompt(s)) => {
            use api::message::tool_call::suggest_prompt::DisplayMode;
            let (prompt, label) = match &s.display_mode {
                Some(DisplayMode::PromptChip(c)) => (c.prompt.clone(), c.label.clone()),
                Some(DisplayMode::InlineQueryBanner(b)) => (b.query.clone(), b.title.clone()),
                None => (String::new(), String::new()),
            };
            (
                "suggest_prompt".to_owned(),
                json!({ "prompt": prompt, "label": label }).to_string(),
            )
        }
        Some(Tool::OpenCodeReview(_)) => ("open_code_review".to_owned(), "{}".to_owned()),
        Some(Tool::InitProject(_)) => ("init_project".to_owned(), "{}".to_owned()),
        Some(Tool::TransferShellCommandControlToUser(t)) => (
            "transfer_shell_command_control_to_user".to_owned(),
            json!({ "reason": t.reason }).to_string(),
        ),
        Some(Tool::ReadSkill(r)) => {
            use api::message::tool_call::read_skill::SkillReference;
            let path = match &r.skill_reference {
                Some(SkillReference::SkillPath(p)) => p.clone(),
                Some(SkillReference::BundledSkillId(id)) => format!("bundled:{id}"),
                None => String::new(),
            };
            (
                "read_skill".to_owned(),
                json!({ "skill_path": path }).to_string(),
            )
        }
        Some(Tool::ReadShellCommandOutput(r)) => {
            use api::message::tool_call::read_shell_command_output::Delay;
            let delay_seconds = match &r.delay {
                Some(Delay::Duration(d)) => Some(d.seconds),
                Some(Delay::OnCompletion(_)) | None => None,
            };
            let mut args = json!({ "command_id": r.command_id });
            if let Some(s) = delay_seconds {
                args["delay_seconds"] = json!(s);
            }
            ("read_shell_command_output".to_owned(), args.to_string())
        }
        Some(other) => {
            let variant_name = format!("{other:?}")
                .split('(')
                .next()
                .unwrap_or("UnknownVariant")
                .to_owned();
            (format!("warp_internal_{}", variant_name), "{}".to_owned())
        }
        None => ("warp_internal_empty".to_owned(), "{}".to_owned()),
    };
    let args_value: Value =
        serde_json::from_str(&args_str).unwrap_or(Value::Object(Default::default()));
    (name, args_value)
}

// ---------------------------------------------------------------------------
// Tools 数组
// ---------------------------------------------------------------------------

fn build_tools_array(params: &RequestParams) -> Vec<GenaiTool> {
    // OpenWarp A2:LRC tag-in 场景剔除 `run_shell_command`,迫使模型选 PTY 操作类工具。
    //
    // 在 alt-screen 长命令(nvim/htop)+ 用户 tag-in 状态下,**模型最容易犯的错**是
    // 调 `run_shell_command` 跑 `taskkill nvim` / `Stop-Process nvim`(开新进程),
    // 这跟当前正在跑的 PTY 没关系,杀不到目标。**正确做法**是
    // `write_to_long_running_shell_command(command_id, input=":q\n", mode=raw)`,
    // 直接给当前 PTY 发指令。
    //
    // 实测带 system prompt 引导 + RunningCommand context prefix 都不够强,
    // 模型仍然倾向 run_shell_command(更简单)。最干净的硬约束就是从 tools 列表
    // 直接移除该工具,模型只能在 PTY 操作类工具中选。
    //
    // 其他工具保留(read_files/grep/ask_user_question 等),允许模型做必要的
    // 信息收集和反问。
    let is_lrc = params.lrc_command_id.is_some();
    let mut out: Vec<GenaiTool> = tools::REGISTRY
        .iter()
        .filter(|t| {
            if is_lrc && t.name == "run_shell_command" {
                return false;
            }
            true
        })
        .map(|t| t.to_genai_tool())
        .collect();

    if let Some(ctx) = params.mcp_context.as_ref() {
        for (name, description, parameters) in tools::mcp::build_mcp_tool_defs(ctx) {
            out.push(
                GenaiTool::new(name)
                    .with_description(description)
                    .with_schema(parameters),
            );
        }
    }
    if is_lrc {
        log::info!(
            "[byop] LRC tag-in: tools array filtered (removed run_shell_command), \
             total tools={}",
            out.len()
        );
    }
    out
}

// ---------------------------------------------------------------------------
// Client / 路由
// ---------------------------------------------------------------------------

/// 把 `AgentProviderApiType` 一对一映射到 genai `AdapterKind`。
fn adapter_kind_for(api_type: AgentProviderApiType) -> AdapterKind {
    match api_type {
        AgentProviderApiType::OpenAi => AdapterKind::OpenAI,
        AgentProviderApiType::OpenAiResp => AdapterKind::OpenAIResp,
        AgentProviderApiType::Gemini => AdapterKind::Gemini,
        AgentProviderApiType::Anthropic => AdapterKind::Anthropic,
        AgentProviderApiType::Ollama => AdapterKind::Ollama,
        AgentProviderApiType::DeepSeek => AdapterKind::DeepSeek,
    }
}

/// 规范化用户填写的 `base_url`,产出供 genai adapter 拼接 service path 的 endpoint URL。
///
/// genai 0.6.x 所有 adapter 都假设 endpoint 以 `/` 结尾、且已经包含版本路径段:
/// - Anthropic:`format!("{base_url}messages")` 期望 `…/v1/`
/// - Gemini:`format!("{base_url}models/{m}:streamGenerateContent")` 期望 `…/v1beta/`
/// - OpenAI / OpenAIResp / DeepSeek:`Url::join("chat/completions" 或 "responses")` 期望 `…/v1/`
/// - Ollama:`format!("{base_url}api/chat")` 期望根路径 `…/`
///
/// 用户实际三种填法:
/// 1. 纯 host(`https://ai.zerx.dev`)— 早期默认行为只补尾 `/` 会拼成 `https://ai.zerx.dev/messages`
///    漏掉 `/v1/` 导致 404。**这里按 api_type 自动追加默认版本路径段**(Anthropic/OpenAI 系→`/v1/`,
///    Gemini→`/v1beta/`,Ollama 不补)。
/// 2. 完整带版本路径(`https://ai.zerx.dev/v1`)— 仅补尾 `/`,不动 path。
/// 3. 留空 — 用 [`AgentProviderApiType::default_base_url`]。
fn normalize_endpoint_url(api_type: AgentProviderApiType, base_url: &str) -> String {
    let trimmed = base_url.trim();
    if trimmed.is_empty() {
        return api_type.default_base_url().to_owned();
    }

    // 解析失败(用户填了畸形 URL)→ 退化到原"补尾 /"行为,让上游报错而不是这里 panic。
    let parsed = match url::Url::parse(trimmed) {
        Ok(u) => u,
        Err(_) => {
            let stripped = trimmed.trim_end_matches('/');
            return format!("{stripped}/");
        }
    };

    // path == "/" 或为空 → 用户只填了 host,自动补上 api_type 默认版本路径段。
    if parsed.path() == "/" || parsed.path().is_empty() {
        // 从 default_base_url 抽 path 部分(如 "/v1/" / "/v1beta/" / "/")。
        let default_path = url::Url::parse(api_type.default_base_url())
            .ok()
            .map(|u| u.path().to_owned())
            .unwrap_or_else(|| "/".to_owned());
        let host_part = trimmed.trim_end_matches('/');
        return format!("{host_part}{default_path}");
    }

    // 用户已自带 path → 仅确保尾随 `/`(genai format!/Url::join 都依赖)。
    let stripped = trimmed.trim_end_matches('/');
    format!("{stripped}/")
}

/// 构造 genai Client。每次请求新建(开销低 — Client 内部只是 reqwest::Client + adapter 表)。
/// `ServiceTargetResolver` capture 当前请求的 endpoint/key/api_type 后,把每次 exec_chat_stream
/// 都强制路由到指定 AdapterKind,完全绕过 genai 默认的"按模型名识别"。
pub(super) fn build_client(
    api_type: AgentProviderApiType,
    base_url: String,
    api_key: String,
) -> Client {
    let adapter_kind = adapter_kind_for(api_type);
    let endpoint_url = normalize_endpoint_url(api_type, &base_url);
    log::info!("[byop] build_client: adapter={adapter_kind:?} endpoint_url={endpoint_url}");
    let key_for_resolver = api_key.clone();
    let resolver = ServiceTargetResolver::from_resolver_fn(
        move |service_target: ServiceTarget| -> Result<ServiceTarget, genai::resolver::Error> {
            let ServiceTarget { model, .. } = service_target;
            let endpoint = Endpoint::from_owned(endpoint_url.clone());
            let auth = AuthData::from_single(key_for_resolver.clone());
            // 用我们指定的 AdapterKind 覆盖 genai 的"按模型名"识别结果,
            // 但保留 model_name 以便上游服务正确寻址模型。
            let model = ModelIden::new(adapter_kind, model.model_name);
            Ok(ServiceTarget {
                endpoint,
                auth,
                model,
            })
        },
    );
    Client::builder()
        .with_service_target_resolver(resolver)
        .build()
}

fn build_chat_options(
    api_type: AgentProviderApiType,
    model_id: &str,
    effort_setting: crate::settings::ReasoningEffortSetting,
) -> ChatOptions {
    let mut opts = ChatOptions::default()
        .with_capture_content(true)
        .with_capture_tool_calls(true)
        .with_capture_reasoning_content(true)
        .with_capture_usage(true)
        // 让 genai 把 DeepSeek-style 模型在 content 中夹带的 <think>...</think>
        // 段抽出来归到 reasoning chunk,UI 显示更干净。仅对支持该格式的 adapter 生效。
        .with_normalize_reasoning_content(true);

    // 仅在用户显式选了非 Auto 档位 **且** 模型支持 reasoning 时才注入。
    // - Auto:不传,让 genai 走"模型名后缀推断"(OpenAI/Anthropic adapter 内部)。
    // - 非 Auto + 模型不支持:也不传,避免向 claude-3-5-haiku / gpt-4o / gemini-1.5-pro
    //   等老模型注入 thinking 参数被上游 400 拒绝。
    if let Some(effort) = effort_setting.to_genai() {
        if super::reasoning::model_supports_reasoning(api_type, model_id) {
            log::info!(
                "[byop] reasoning_effort injected: model={model_id} setting={effort_setting:?}"
            );
            opts = opts.with_reasoning_effort(effort);
        } else {
            log::info!(
                "[byop] reasoning_effort SKIPPED: model={model_id} not in capability list \
                 (api_type={api_type:?} setting={effort_setting:?}); request sent without thinking params"
            );
        }
    }
    opts
}

fn map_genai_error(err: genai::Error) -> OpenAiCompatibleError {
    use genai::Error as G;
    match err {
        // 真·解析失败:JSON 反序列化阶段
        G::StreamParse { .. }
        | G::SerdeJson(_)
        | G::JsonValueExt(_)
        | G::InvalidJsonResponseElement { .. } => OpenAiCompatibleError::Decode(format!("{err}")),

        // 网络/流式发送阶段失败(reqwest 连接、TLS、DNS、超时、流中断等)
        G::WebStream { .. } | G::WebAdapterCall { .. } | G::WebModelCall { .. } => {
            OpenAiCompatibleError::Stream(format!("{err}"))
        }

        // 服务端返回的 HTTP 错误状态
        G::HttpError {
            status,
            body,
            canonical_reason,
        } => OpenAiCompatibleError::Status {
            status: status.as_u16(),
            body: if canonical_reason.is_empty() {
                body
            } else {
                format!("{canonical_reason}: {body}")
            },
        },

        // 其余(请求构造、鉴权、能力不支持等)归为通用错误,避免误导成"解析失败"
        other => OpenAiCompatibleError::Other(format!("{other}")),
    }
}

// ---------------------------------------------------------------------------
// 主流程
// ---------------------------------------------------------------------------

/// 标题生成所需的 BYOP 配置。可能与主请求同 provider 也可能不同(用户在 Profile
/// Editor 里独立选了 title_model)。
pub struct TitleGenInput {
    pub base_url: String,
    pub api_key: String,
    pub model_id: String,
    pub api_type: AgentProviderApiType,
    pub reasoning_effort: crate::settings::ReasoningEffortSetting,
}

/// `task_id`: conversation 的 root task id(controller 端从 history model 取)。
/// `target_task_id`: 本轮模型输出应该写入的 task id;普通对话等于 root,
/// CLI subagent 后续轮为已有 subtask。
/// `needs_create_task`: 仅首轮(root 还是 Optimistic)需要 emit `CreateTask`。
pub async fn generate_byop_output(
    params: RequestParams,
    base_url: String,
    api_key: String,
    model_id: String,
    api_type: AgentProviderApiType,
    reasoning_effort: crate::settings::ReasoningEffortSetting,
    task_id: String,
    target_task_id: String,
    needs_create_task: bool,
    // LRC 场景绑定的 CLI subagent `command_id`(= LRC block id 字符串)。
    lrc_command_id: Option<String>,
    // 仅 tag-in 首轮为 true:流头会合成虚拟 `tool_call::Subagent` + CreateTask,
    // 用 server subtask 升级 master 路径已经创建的 optimistic CLI subtask。
    lrc_should_spawn_subagent: bool,
    // 选中模型的 context window(tokens)。Some 时:流末用 genai captured_usage
    // 计算 (prompt_tokens + completion_tokens) / context_window 写回
    // ConversationUsageMetadata,驱动 footer 的 "X% context remaining" 实时更新。
    // None ⇒ 跳过(用户未填 + catalog 无),UI 维持 100% 占位。
    context_window: Option<u32>,
    _cancellation_rx: futures::channel::oneshot::Receiver<()>,
) -> Result<ResponseStream, ConvertToAPITypeError> {
    let force_echo_reasoning = super::reasoning::model_requires_reasoning_echo(api_type, &model_id);
    let chat_req = build_chat_request(&params, force_echo_reasoning);
    let chat_opts = build_chat_options(api_type, &model_id, reasoning_effort);
    let client = build_client(api_type, base_url, api_key);
    let conversation_id = params
        .conversation_token
        .as_ref()
        .map(|t| t.as_str().to_string())
        .unwrap_or_default();
    let request_id = Uuid::new_v4().to_string();
    let mcp_context = params.mcp_context.clone();

    // ⚠️ BYOP 持久化关键:warp 自家路径下,以下 ClientAction 都是 server 端 emit
    // 让 client 端把 UserQuery / ToolCallResult 等"非模型产出"的 message
    // 写回 task.messages,从而让下一轮请求的 `params.tasks` snapshot 完整。
    //
    // BYOP 去云化客户端自管,server 端不存在,必须我们自己 emit 这些写回事件,
    // 否则下一轮 `compute_active_tasks` 只看到模型产出(reasoning/output/tool_call),
    // 缺失对应的 user_query 和 tool_call_result,模型 context 严重断裂。
    //
    // 这里在流开始后 emit 两类事件:
    //   1. AddMessagesToTask{UserQuery}    ← 当前轮所有 UserQuery input
    //   2. AddMessagesToTask{ToolCallResult} ← 当前轮所有 ActionResult input
    //
    // emit 时机必须在 CreateTask 之后(任务已升级为 Server 状态),
    // 在模型响应开始之前(UI 顺序:user 显示 → thinking/answer)。
    let pending_user_queries: Vec<String> = params
        .input
        .iter()
        .filter_map(|i| match i {
            AIAgentInput::UserQuery { query, .. } => Some(query.clone()),
            _ => None,
        })
        .collect();
    // ToolCallResult 持久化:用 `tools::serialize_action_result` 把 ActionResult
    // 序列化为 JSON 字符串,装进 Message 的 server_message_data 字段
    // (warp protobuf 的 `tool_call_result.result` oneof 都是结构化 variant,
    // 没有通用 string 兜底;但 server_message_data 是自由字符串字段,刚好够用)。
    // 下一轮 build_chat_request 在 ToolCallResult 分支:result=None 时从
    // server_message_data 读 content,result=Some 时走 tools::serialize_result。
    let pending_tool_results: Vec<(String, String)> = params
        .input
        .iter()
        .filter_map(|i| match i {
            AIAgentInput::ActionResult { result, .. } => {
                let id = result.id.to_string();
                let content = tools::serialize_action_result(result).unwrap_or_else(|| {
                    serde_json::json!({ "result": result.result.to_string() }).to_string()
                });
                Some((id, content))
            }
            _ => None,
        })
        .collect();

    // INFO 级别一行总览 + 每条 message 一行简报(role + 文本长度 + tool 计数 + reasoning 标记),
    // 默认日志配置即可看到,便于诊断"历史是否完整传上去"等问题。
    log::info!(
        "[byop] adapter={:?} model={} system_len={} messages={} tools={}",
        adapter_kind_for(api_type),
        model_id,
        chat_req.system.as_deref().map(str::len).unwrap_or(0),
        chat_req.messages.len(),
        chat_req.tools.as_ref().map(|t| t.len()).unwrap_or(0),
    );
    for (idx, m) in chat_req.messages.iter().enumerate() {
        let role = format!("{:?}", m.role);
        let text_len = m.content.first_text().map(str::len).unwrap_or(0);
        let tc_count = m.content.tool_calls().len();
        let tr_count = m.content.tool_responses().len();
        // reasoning_content 检测 — genai 把它存为 ContentPart::ReasoningContent,
        // 没有公开 getter,这里通过 size() 与 first_text+tool_count 的和差异粗判。
        log::info!(
            "[byop]  [{idx}] role={role} text_len={text_len} tool_calls={tc_count} tool_responses={tr_count}"
        );
    }

    // 诊断:构造包含 system / messages / tools 的完整 ChatRequest JSON dump,保存到
    // stream 闭包。真实 Anthropic wire body 会由 genai adapter 再转换一层,但这里已经
    // 覆盖所有传入 BYOP 的原始字符串,足够定位非法 escape 来自 prompt、工具描述、
    // schema 还是 tool result。
    let diag_body_json = serde_json::to_string(&json!({
        "model": &model_id,
        "chat_request": &chat_req,
    }))
    .unwrap_or_default();
    log::info!("[byop] diag_body_approx_len={}", diag_body_json.len());
    log::info!("[byop-diag] full_request_json={diag_body_json}");

    // 主动扫描原始文本里的"可疑反斜杠序列":虽然 serde_json 序列化时会把
    // `\` escape 成 `\\`(合法 JSON),但若出现在 base64 / 已 escape 字符串等场景,
    // 上游 proxy 若做"把 `\\u` 误解析回 `\u`"的转换,就会触发 invalid escape。
    // 这里只扫描 raw 字符串里的 `\n` / `\u` / `\x` / `\0`-`\9` 等模式,辅助定位嫌疑文本。
    fn scan_suspicious_backslash(label: &str, s: &str) {
        let bytes = s.as_bytes();
        let mut bs_hits: Vec<(usize, String)> = Vec::new();
        let mut ctrl_hits: Vec<(usize, u8)> = Vec::new();
        for (i, &b) in bytes.iter().enumerate() {
            // 1) 字面 `\u` `\x` `\a` `\v` 序列(serde_json 会输出 `\\X`,合法,
            //    但 proxy 若做"`\\u` 还原 `\u`"误处理会触发 invalid escape)
            if b == b'\\' && i + 1 < bytes.len() {
                let next = bytes[i + 1];
                if matches!(next, b'n' | b'r' | b't' | b'u' | b'x' | b'a' | b'v') {
                    let end = (i + 8).min(bytes.len());
                    let snippet = String::from_utf8_lossy(&bytes[i..end]).to_string();
                    if bs_hits.len() < 5 {
                        bs_hits.push((i, snippet));
                    }
                }
            }
            // 2) raw 控制字符(byte 0x00-0x08, 0x0B-0x0C, 0x0E-0x1F)
            //    serde_json 会 escape 为 `\u00XX`,合法 JSON;但部分 strict proxy
            //    或经过 base64 / 中间编码层时这些字节最容易出错。
            if (b < 0x20 && !matches!(b, b'\t' | b'\n' | b'\r')) && ctrl_hits.len() < 10 {
                ctrl_hits.push((i, b));
            }
        }
        if !bs_hits.is_empty() {
            log::warn!("[byop] {label} suspicious literal '\\X' patterns: {bs_hits:?}");
        }
        if !ctrl_hits.is_empty() {
            log::warn!("[byop] {label} contains raw control chars (offset, byte): {ctrl_hits:?}");
        }
    }
    scan_suspicious_backslash("full_request_json", &diag_body_json);
    if let Some(sys) = chat_req.system.as_deref() {
        scan_suspicious_backslash("system", sys);
    }
    for (idx, m) in chat_req.messages.iter().enumerate() {
        if let Some(t) = m.content.first_text() {
            scan_suspicious_backslash(&format!("msg[{idx}]"), t);
        }
    }

    let stream = async_stream::stream! {
        // 1) StreamInit — 始终先发,UI 能立刻显示 "thinking..."
        yield Ok(api::ResponseEvent {
            r#type: Some(api::response_event::Type::Init(
                api::response_event::StreamInit {
                    request_id: request_id.clone(),
                    conversation_id,
                    run_id: String::new(),
                },
            )),
        });

        // 2) 首轮:CreateTask 升级 Optimistic root → Server。
        if needs_create_task {
            yield Ok(create_task_event(&task_id));
        }

        // 3) 持久化 input 里的 UserQuery / ToolCallResult 到 task.messages。
        //    (warp server 路径由后端 emit;BYOP 客户端必须自己 emit,见上方注释。)
        //    tag-in 首轮先写 root,再由下面的 spawn 分支复制到新 subtask;已有 CLI
        //    subagent 的后续轮直接写 target_task_id。
        let persistence_task_id = if lrc_should_spawn_subagent {
            task_id.as_str()
        } else {
            target_task_id.as_str()
        };
        let mut persistence_messages: Vec<api::Message> = Vec::new();
        for q in &pending_user_queries {
            persistence_messages.push(make_user_query_message(
                persistence_task_id,
                &request_id,
                q.clone(),
            ));
        }
        for (call_id, content) in &pending_tool_results {
            persistence_messages.push(make_tool_call_result_message(
                persistence_task_id,
                &request_id,
                call_id.clone(),
                content.clone(),
            ));
        }
        if !persistence_messages.is_empty() {
            yield Ok(make_add_messages_event(persistence_task_id, persistence_messages));
        }

        // 3.5) LRC subagent spawn(对齐上游云端的 cli subagent 注入路径)。
        //
        // 当请求来自 alt-screen + agent tagged-in 状态时,`lrc_command_id` 携带当前 LRC
        // block 的 id 字符串。此处客户端合成两条事件:
        //   a) AddMessagesToTask(root, [<虚拟 subagent tool_call>])
        //      在 root.messages 里挂一条 ToolCall::Subagent { task_id=<新 subtask>,
        //      metadata: Cli { command_id }, payload: "" }。
        //      conversation `Task::new_subtask` 会从 parent.messages 里按 task_id 匹配
        //      这条 subagent_call,提取出 SubagentParams 挂到 subtask。
        //   b) CreateTask(api::Task { id=<新 subtask>, dependencies.parent_task_id=root })
        //      触发 `apply_client_action::CreateTask`,因 parent_id 非空走 `new_subtask`,
        //      接着 emit `BlocklistAIHistoryEvent::CreatedSubtask` →
        //      `cli_controller::handle_history_model_event` 看到 cli_subagent_block_id
        //      非空,emit `CLISubagentEvent::SpawnedSubagent` → terminal_view 创建
        //      `CLISubagentView` 浮窗,挂进 `cli_subagent_views` map。
        //
        // 切换后续 chunk emit 的 task_id 到 subtask_id,让模型 reasoning/output/tool_call
        // 全部进 subtask,subagent_view 据此渲染浮窗内容。
        //
        // 时序约束:必须在 root CreateTask + UserQuery 持久化之后,模型流之前。
        // 否则 conversation 找不到 root task / 找不到 user query 引用对。
        let mut current_task_id = if lrc_should_spawn_subagent {
            task_id.clone()
        } else {
            target_task_id.clone()
        };
        if lrc_should_spawn_subagent {
            let Some(command_id) = lrc_command_id.clone() else {
                log::warn!("[byop] LRC spawn requested without command_id");
                yield Err(Arc::new(AIApiError::Other(anyhow::anyhow!(
                    "BYOP LRC spawn requested without command_id"
                ))));
                return;
            };
            let subtask_id = Uuid::new_v4().to_string();
            let tool_call_id = Uuid::new_v4().to_string();
            log::info!(
                "[byop] LRC tag-in: spawning cli subagent subtask={subtask_id} \
                 command_id={command_id} parent={task_id}"
            );

            let subagent_tool = api::message::tool_call::Tool::Subagent(
                api::message::tool_call::Subagent {
                    task_id: subtask_id.clone(),
                    payload: String::new(),
                    metadata: Some(
                        api::message::tool_call::subagent::Metadata::Cli(
                            api::message::tool_call::subagent::CliSubagent {
                                command_id,
                            },
                        ),
                    ),
                },
            );
            let subagent_msg = make_tool_call_message(
                &task_id,
                &request_id,
                &tool_call_id,
                subagent_tool,
            );
            // a) 把 subagent tool_call 挂到 root.messages,供 new_subtask 反查 SubagentParams。
            yield Ok(make_add_messages_event(&task_id, vec![subagent_msg]));
            // b) 创建带 parent_task_id 的 subtask;conversation 检测 parent_id 非空 →
            //    走 `Task::new_subtask` 路径,自动绑定 SubagentParams。
            yield Ok(create_subtask_event(&subtask_id, &task_id));

            // c) OpenWarp A1:把当前轮的 UserQuery 也复制一份到 subtask,初始化 subtask 的
            //    exchange.output.messages。否则 CLISubagentView 渲染时 subtask 的 exchanges
            //    output 为空,浮窗永远只显示 49.6 高度的空对话框,看不到任何内容。
            //    上游云端在 cli subagent 任务上有完整 ClientAction 序列填 exchange.output,
            //    BYOP 客户端自管必须显式注入。
            //
            //    只复制本轮 UserQuery(`pending_user_queries`),不动 root 的副本(root
            //    保留 user query 引用以避免 exchange.input 为空导致状态机错乱)。
            //    后续模型 chunks 走 `current_task_id = subtask_id`,append 到这个起点之后。
            if !pending_user_queries.is_empty() {
                let mut subtask_messages: Vec<api::Message> = Vec::new();
                for q in &pending_user_queries {
                    subtask_messages.push(make_user_query_message(
                        &subtask_id,
                        &request_id,
                        q.clone(),
                    ));
                }
                yield Ok(make_add_messages_event(&subtask_id, subtask_messages));
            }

            // 后续 chunk emit 切到 subtask。
            current_task_id = subtask_id;
        }

        log::info!("[byop] opening stream: model={model_id}");
        let mut sdk_stream = match client
            .exec_chat_stream(&model_id, chat_req, Some(&chat_opts))
            .await
        {
            Ok(resp) => {
                log::info!("[byop] stream opened OK (HTTP request accepted)");
                resp.stream
            }
            Err(e) => {
                let mapped = map_genai_error(e);
                log::error!("[byop] open stream failed: {mapped:#}");
                yield Err(Arc::new(AIApiError::Other(anyhow::anyhow!(
                    "BYOP open stream failed: {mapped}"
                ))));
                return;
            }
        };

        // 流式状态:文本 / 推理各自的 message id 在第一次 chunk 到达时生成,
        // 之后的 chunk 走 AppendToMessageContent 增量追加。
        let mut text_msg_id: Option<String> = None;
        let mut reasoning_msg_id: Option<String> = None;
        // tool_call 按 call_id 累积 — genai 流式发的 ToolCallChunk 已带完整 ToolCall
        // (since 0.4.0 行为),但跨 chunk 同一 call_id 可能多次出现 args 增量,
        // 用 HashMap 按 id 累积后在流末统一 emit。
        let mut tool_bufs: HashMap<String, ToolCall> = HashMap::new();
        // 诊断:统计 stream 各类事件计数,流末打 INFO log。
        // 用于排查"消息静默消失"——如果 chunk_count=0 且 tool_count=0,说明上游返回空内容。
        let mut start_count: u32 = 0;
        let mut chunk_count: u32 = 0;
        let mut chunk_bytes: usize = 0;
        let mut reasoning_count: u32 = 0;
        let mut reasoning_bytes: usize = 0;
        let mut tool_chunk_count: u32 = 0;
        let mut end_count: u32 = 0;
        let mut other_count: u32 = 0;
        // 累积本轮 token 使用量。genai 在 ChatStreamEvent::End 事件里携带
        // captured_usage(Option<Usage>),其 prompt_tokens 是本轮整段 history
        // (Anthropic / OpenAI 都按"完整请求 prompt"计),completion_tokens 是模型输出。
        // 二者相加除以 context_window 即为"context 占用率",和 warp 自家 server 路径语义一致。
        let mut captured_prompt_tokens: i32 = 0;
        let mut captured_completion_tokens: i32 = 0;

        while let Some(item) = sdk_stream.next().await {
            let event = match item {
                Ok(ev) => ev,
                Err(e) => {
                    let mapped = map_genai_error(e);
                    let err_text = format!("{mapped:#}");
                    log::error!("[byop] stream chunk error: {err_text}");
                    log::error!("[byop-diag] full_request_json_on_error={diag_body_json}");
                    // 从错误消息里 parse "column N",dump diag_body_json 该位置 ±200 char 上下文 + 字节 hex。
                    if let Some(col) = err_text
                        .split("column ")
                        .nth(1)
                        .and_then(|s| s.chars().take_while(|c| c.is_ascii_digit()).collect::<String>().parse::<usize>().ok())
                    {
                        let body = &diag_body_json;
                        let byte_len = body.len();
                        let start = col.saturating_sub(200).min(byte_len);
                        let end = (col + 200).min(byte_len);
                        let context = body.get(start..end).unwrap_or("(slice failed: 非 char 边界)");
                        log::error!(
                            "[byop] error column={col} diag_body_len={byte_len} context[{start}..{end}]={context:?}"
                        );
                        let hex_start = col.saturating_sub(20).min(byte_len);
                        let hex_end = (col + 20).min(byte_len);
                        if let Some(slice) = body.as_bytes().get(hex_start..hex_end) {
                            log::error!("[byop] error bytes[{hex_start}..{hex_end}] hex={slice:02x?}");
                        }
                    }
                    yield Err(Arc::new(AIApiError::Other(anyhow::anyhow!(
                        "BYOP stream error: {mapped}"
                    ))));
                    return;
                }
            };

            match event {
                ChatStreamEvent::Start => {
                    // unit event;UI 已通过 StreamInit 显示 thinking,这里 no-op
                    start_count += 1;
                }
                ChatStreamEvent::Chunk(c) if !c.content.is_empty() => {
                    chunk_count += 1;
                    chunk_bytes += c.content.len();
                    if let Some(id) = text_msg_id.clone() {
                        yield Ok(make_append_event(&current_task_id, &id, AppendKind::Text(c.content)));
                    } else {
                        let new_id = Uuid::new_v4().to_string();
                        let mut msg = make_agent_output_message(&current_task_id, &request_id, c.content);
                        msg.id = new_id.clone();
                        text_msg_id = Some(new_id);
                        yield Ok(make_add_messages_event(&current_task_id, vec![msg]));
                    }
                }
                ChatStreamEvent::Chunk(_) => {}
                ChatStreamEvent::ReasoningChunk(c) if !c.content.is_empty() => {
                    reasoning_count += 1;
                    reasoning_bytes += c.content.len();
                    if let Some(id) = reasoning_msg_id.clone() {
                        yield Ok(make_append_event(&current_task_id, &id, AppendKind::Reasoning(c.content)));
                    } else {
                        let new_id = Uuid::new_v4().to_string();
                        let mut msg = make_reasoning_message(&current_task_id, &request_id, c.content);
                        msg.id = new_id.clone();
                        reasoning_msg_id = Some(new_id);
                        yield Ok(make_add_messages_event(&current_task_id, vec![msg]));
                    }
                }
                ChatStreamEvent::ReasoningChunk(_) => {}
                ChatStreamEvent::ToolCallChunk(tc) => {
                    tool_chunk_count += 1;
                    let mut call = tc.tool_call;
                    // 极个别 provider(自建 ollama 代理等)不发 call_id,本地 uuid 兜底。
                    if call.call_id.is_empty() {
                        call.call_id = Uuid::new_v4().to_string();
                    }
                    // 同一 call_id 多次 chunk:后到的覆盖(genai 已合并 args)。
                    tool_bufs.insert(call.call_id.clone(), call);
                }
                ChatStreamEvent::End(end) => {
                    end_count += 1;
                    // genai >= 0.4.0 的 captured_content 含 tool_calls。
                    // 优先用 captured_content 里的 tool_calls(更完整),
                    // 否则用 streaming 中累积的 tool_bufs。
                    if let Some(content) = end.captured_content.as_ref() {
                        for call in content.tool_calls() {
                            tool_bufs.entry(call.call_id.clone()).or_insert_with(|| call.clone());
                        }
                    }
                    if let Some(usage) = end.captured_usage.as_ref() {
                        // 多次 End 取最大值兜底(理论上单次 stream 只有一次 End)。
                        if let Some(p) = usage.prompt_tokens {
                            captured_prompt_tokens = captured_prompt_tokens.max(p);
                        }
                        if let Some(c) = usage.completion_tokens {
                            captured_completion_tokens = captured_completion_tokens.max(c);
                        }
                    }
                }
                _ => {
                    other_count += 1;
                    // ThoughtSignatureChunk 等暂不处理(Gemini 3 thoughts 需要回传给后续轮次,
                    // 当前 BYOP 不持久化 thought_signatures,接受降级)
                }
            }
        }

        // 流统计 INFO log。chunk_count=0 && tool_count=0 时上游返回为空,
        // 大概率是 model_id 不被识别 / max_tokens 缺失 / Anthropic API 兼容代理返回 200 但 body 空。
        let total_tools = tool_bufs.len();
        log::info!(
            "[byop] stream stats: start={start_count} chunks={chunk_count} ({chunk_bytes}B) \
             reasoning={reasoning_count} ({reasoning_bytes}B) tool_chunks={tool_chunk_count} \
             ends={end_count} other={other_count} captured_tools={total_tools}"
        );
        if chunk_count == 0 && reasoning_count == 0 && total_tools == 0 {
            log::warn!(
                "[byop] stream returned 0 content / 0 reasoning / 0 tool_calls — \
                 上游可能返回空响应(model_id 错? max_tokens 缺? proxy 异常?)"
            );
        }

        // 流结束:把累积的 tool_calls 一次性发出。
        let mut final_messages: Vec<api::Message> = Vec::new();
        for call in tool_bufs.into_values() {
            // 诊断:dump 模型实际发的 tool_call raw payload
            // (call_id / fn_name / fn_arguments JSON 原文 + 类型标注),
            // 便于核对模型是否按 schema 出入参(常见问题:bool 字段被字符串化、
            // 数字被加引号、嵌套对象塌成字符串等)。
            let args_repr = if call.fn_arguments.is_string() {
                format!("string({:?})", call.fn_arguments.as_str().unwrap_or(""))
            } else {
                format!(
                    "{}({})",
                    match &call.fn_arguments {
                        Value::Object(_) => "object",
                        Value::Array(_) => "array",
                        Value::Bool(_) => "bool",
                        Value::Number(_) => "number",
                        Value::Null => "null",
                        Value::String(_) => "string",
                    },
                    call.fn_arguments
                )
            };
            log::info!(
                "[byop] tool_call_in: name={} call_id={} args={}",
                call.fn_name,
                call.call_id,
                args_repr,
            );
            match parse_incoming_tool_call(&call, mcp_context.as_ref()) {
                Ok(warp_tool) => {
                    final_messages.push(make_tool_call_message(
                        &current_task_id,
                        &request_id,
                        &call.call_id,
                        warp_tool,
                    ));
                }
                Err(e) => {
                    // 关键:不再把 from_args 失败吞成纯文本(原实现:emit AgentOutput),
                    // 因为模型那一轮以为自己调了 tool 在等 result,看到一段中文 assistant 文字
                    // 完全不知道是参数类型错,无法定向修正重试。
                    // 改成 emit 一对 ToolCall(carrier) + ToolCallResult(error JSON),
                    // 让模型在下一轮看到标准 tool_result error,可以按惯例改 args 重试或换工具。
                    //
                    // ToolCall 的 `tool` oneof 留 None(没有合适的结构化 variant),原始
                    // fn_name + args_str 通过 server_message_data 携带,
                    // serialize_outgoing_tool_call 的 carrier 分支会优先还原。
                    let args_str = if call.fn_arguments.is_string() {
                        call.fn_arguments.as_str().unwrap_or("").to_owned()
                    } else {
                        call.fn_arguments.to_string()
                    };
                    log::warn!(
                        "[byop] tool_call parse failed → emit synthetic error tool_result: \
                         tool={} call_id={} err={e:#}",
                        call.fn_name,
                        call.call_id
                    );
                    let error_payload = serde_json::json!({
                        "error": "invalid_arguments",
                        "detail": e.to_string(),
                        "tool": call.fn_name,
                        "received_args": &args_str,
                        "hint": "Arguments did not match the tool's JSON Schema. \
                                 Re-emit the tool call with corrected types / required fields, \
                                 or pick a different tool.",
                    });
                    let error_content = serde_json::to_string(&error_payload)
                        .unwrap_or_else(|_| r#"{"error":"invalid_arguments"}"#.to_owned());
                    final_messages.push(make_tool_call_carrier_message(
                        &current_task_id,
                        &request_id,
                        &call.call_id,
                        &call.fn_name,
                        &args_str,
                    ));
                    final_messages.push(make_tool_call_result_message(
                        &current_task_id,
                        &request_id,
                        call.call_id.clone(),
                        error_content,
                    ));
                }
            }
        }
        if !final_messages.is_empty() {
            yield Ok(make_add_messages_event(&current_task_id, final_messages));
        }

        // 把 captured token usage 折算成 ConversationUsageMetadata.context_window_usage
        // 注入 StreamFinished — controller 的 handle_response_stream_finished 会把它写到
        // conversation.conversation_usage_metadata,footer 监听 UpdatedStreamingExchange/
        // AppendedExchange 事件即在每轮末实时刷新 "X% context remaining" 工具提示。
        let usage_metadata = context_window.and_then(|cw| {
            if cw == 0 || (captured_prompt_tokens == 0 && captured_completion_tokens == 0) {
                return None;
            }
            let used = (captured_prompt_tokens + captured_completion_tokens).max(0) as f32;
            let pct = (used / cw as f32).clamp(0.0, 1.0);
            log::info!(
                "[byop] context usage: prompt={} completion={} window={} → {:.1}%",
                captured_prompt_tokens,
                captured_completion_tokens,
                cw,
                pct * 100.0
            );
            Some(api::response_event::stream_finished::ConversationUsageMetadata {
                context_window_usage: pct,
                summarized: false,
                credits_spent: 0.0,
                #[allow(deprecated)]
                token_usage: Vec::new(),
                tool_usage_metadata: None,
                warp_token_usage: std::collections::HashMap::new(),
                byok_token_usage: std::collections::HashMap::new(),
            })
        });
        yield Ok(make_finished_done(usage_metadata));
    };

    Ok(Box::pin(stream))
}

/// 用独立 BYOP 配置发一个短的非工具请求,让模型对首条 user query 生成会话标题。
/// 所有错误吞掉(返回 Err 让上游打 warn log,不影响主流程)。
///
/// 实现委托给 `oneshot::byop_oneshot_completion`,这里只负责拼 prompt 和清洗输出。
///
/// ## prompt 设计
///
/// - **system**: 见 `prompts/tasks/title_system.md`,结构化 task/rules/examples,
///   覆盖中英双语示例,显式禁止 "回答用户问题 / 拒绝 / 加引号"。
/// - **user**: 把原始 `user_query` 包在 `<user>...</user>` 里,前置一句明确的
///   "Generate a title for this conversation:",避免弱模型把 user 当主指令直接答复
///   (典型坏 case:user="你是谁" → 模型答"我是 Claude"被当作标题)。
/// - **temperature**: 0.3 — opencode title agent 用 0.5,这里更保守,降低跑题。
pub(crate) async fn generate_title_via_byop(
    tg: &TitleGenInput,
    user_query: &str,
) -> Result<Option<String>, anyhow::Error> {
    let cfg = super::oneshot::OneshotConfig {
        base_url: tg.base_url.clone(),
        api_key: tg.api_key.clone(),
        model_id: tg.model_id.clone(),
        api_type: tg.api_type,
        reasoning_effort: tg.reasoning_effort,
    };
    let system = include_str!("prompts/tasks/title_system.md");
    let user_prompt = format!(
        "Generate a title for this conversation:\n<user>{}</user>",
        user_query
    );
    let opts = super::oneshot::OneshotOptions {
        max_chars: Some(1000),
        temperature: Some(0.3),
        ..Default::default()
    };
    let raw = super::oneshot::byop_oneshot_completion(&cfg, system, &user_prompt, &opts).await?;
    Ok(sanitize_title(&raw))
}

/// 清洗 title 文本。空字符串 → None(让上游跳过 emit)。
///
/// 处理顺序:
/// 1. 剥 `<think>...</think>` / `<reasoning>...</reasoning>` 思考块(reasoning 模型常见前缀)。
/// 2. 取首行非空内容(模型常前置"好的,标题是:"再换行给标题)。
/// 3. 剥 `Title:` / `标题:` / `Thread:` / `Subject:` 等前缀(大小写不敏感)。
/// 4. 剥首尾引号 / 反引号(中英文)。
/// 5. 去尾标点。
/// 6. 50 字符截断(按 char,保护 CJK),超过则尾部加 `…`。
fn sanitize_title(raw: &str) -> Option<String> {
    // 1. 剥 reasoning 标签(可能有多个,DOTALL 模式)。
    let mut s = raw.to_owned();
    for tag in &["think", "reasoning", "thought", "scratchpad"] {
        let open = format!("<{}>", tag);
        let close = format!("</{}>", tag);
        while let (Some(start), Some(end_rel)) = (
            s.find(&open),
            s.find(&close).map(|e| e + close.len()),
        ) {
            if end_rel <= start {
                break;
            }
            s.replace_range(start..end_rel, "");
        }
    }

    // 2. 取首行非空。
    let first_line = s
        .lines()
        .map(|l| l.trim())
        .find(|l| !l.is_empty())
        .unwrap_or("")
        .to_owned();
    let mut s = first_line;

    // 3. 剥前缀(循环剥,处理 "Title: 标题: foo" 这类双前缀)。
    let prefixes = [
        "title:", "subject:", "thread:", "标题:", "标题：", "主题:", "主题：",
    ];
    loop {
        let lower = s.to_lowercase();
        let mut stripped = false;
        for p in &prefixes {
            if lower.starts_with(p) {
                s = s[p.len()..].trim_start().to_owned();
                stripped = true;
                break;
            }
        }
        if !stripped {
            break;
        }
    }

    // 4. 剥首尾引号(中英文)。
    let quotes = ['"', '\'', '`', '“', '”', '‘', '’', '《', '》', '「', '」'];
    while let Some(c) = s.chars().next() {
        if quotes.contains(&c) {
            s.remove(0);
        } else {
            break;
        }
    }
    while let Some(c) = s.chars().last() {
        if quotes.contains(&c) {
            let new_len = s.len() - c.len_utf8();
            s.truncate(new_len);
        } else {
            break;
        }
    }

    // 5. 去尾标点。
    while let Some(c) = s.chars().last() {
        if matches!(
            c,
            '.' | '。' | '!' | '！' | '?' | '？' | ',' | '，' | ';' | '；' | ':' | ':'
        ) {
            let new_len = s.len() - c.len_utf8();
            s.truncate(new_len);
        } else {
            break;
        }
    }

    let s = s.trim().to_owned();
    if s.is_empty() {
        return None;
    }

    // 6. 50 字符截断(按 char,保护 CJK)。超长加省略号。
    const MAX_CHARS: usize = 50;
    let chars: Vec<char> = s.chars().collect();
    if chars.len() > MAX_CHARS {
        let mut truncated: String = chars.iter().take(MAX_CHARS - 1).collect();
        truncated.push('…');
        Some(truncated)
    } else {
        Some(s)
    }
}

// ---------------------------------------------------------------------------
// Event 构造辅助
// ---------------------------------------------------------------------------

enum AppendKind {
    Reasoning(String),
    Text(String),
}

fn make_add_messages_event(task_id: &str, messages: Vec<api::Message>) -> api::ResponseEvent {
    api::ResponseEvent {
        r#type: Some(api::response_event::Type::ClientActions(
            api::response_event::ClientActions {
                actions: vec![api::ClientAction {
                    action: Some(api::client_action::Action::AddMessagesToTask(
                        api::client_action::AddMessagesToTask {
                            task_id: task_id.to_owned(),
                            messages,
                        },
                    )),
                }],
            },
        )),
    }
}

fn make_append_event(task_id: &str, message_id: &str, kind: AppendKind) -> api::ResponseEvent {
    let (msg_inner, mask_path) = match kind {
        AppendKind::Reasoning(r) => (
            api::message::Message::AgentReasoning(api::message::AgentReasoning {
                reasoning: r,
                finished_duration: None,
            }),
            "agent_reasoning.reasoning",
        ),
        AppendKind::Text(t) => (
            api::message::Message::AgentOutput(api::message::AgentOutput { text: t }),
            "agent_output.text",
        ),
    };
    let message = api::Message {
        id: message_id.to_owned(),
        task_id: task_id.to_owned(),
        server_message_data: String::new(),
        citations: vec![],
        message: Some(msg_inner),
        request_id: String::new(),
        timestamp: None,
    };
    api::ResponseEvent {
        r#type: Some(api::response_event::Type::ClientActions(
            api::response_event::ClientActions {
                actions: vec![api::ClientAction {
                    action: Some(api::client_action::Action::AppendToMessageContent(
                        api::client_action::AppendToMessageContent {
                            task_id: task_id.to_owned(),
                            message: Some(message),
                            mask: Some(prost_types::FieldMask {
                                paths: vec![mask_path.to_owned()],
                            }),
                        },
                    )),
                }],
            },
        )),
    }
}

fn parse_incoming_tool_call(
    call: &ToolCall,
    mcp_ctx: Option<&crate::ai::agent::MCPContext>,
) -> anyhow::Result<api::message::tool_call::Tool> {
    // genai ToolCall.fn_arguments 是 Value;tools::* 的 from_args 期望 &str,
    // 把 Value 序列化回字符串后传入(原协议就是字符串 JSON)。
    let args_str = if call.fn_arguments.is_string() {
        call.fn_arguments.as_str().unwrap_or("").to_owned()
    } else {
        call.fn_arguments.to_string()
    };
    if tools::mcp::is_mcp_function(&call.fn_name) {
        return tools::mcp::parse_mcp_tool_call(&call.fn_name, &args_str, mcp_ctx);
    }
    let Some(tool) = tools::lookup(&call.fn_name) else {
        anyhow::bail!("unknown tool name: {}", call.fn_name);
    };
    match (tool.from_args)(&args_str) {
        Ok(t) => Ok(t),
        Err(e) => {
            // 诊断:解析失败时把 from_args 实际拿到的字符串原样打出来,
            // 配合上层 [byop] tool_call_in 的 args= 行可以判断:
            //   1. 是否模型出参类型错(bool→"true" / 数字→"1" 等)
            //   2. 是否 genai Value→string 转换中 escape 出问题
            //   3. 是否 fn_arguments 整段被字符串化(应该 object 却是 string)
            log::warn!(
                "[byop] from_args failed: tool={} err={e:#} args_str={args_str}",
                call.fn_name
            );
            Err(e)
        }
    }
}

fn make_reasoning_message(task_id: &str, request_id: &str, reasoning: String) -> api::Message {
    api::Message {
        id: Uuid::new_v4().to_string(),
        task_id: task_id.to_owned(),
        server_message_data: String::new(),
        citations: vec![],
        message: Some(api::message::Message::AgentReasoning(
            api::message::AgentReasoning {
                reasoning,
                finished_duration: None,
            },
        )),
        request_id: request_id.to_owned(),
        timestamp: None,
    }
}

fn make_agent_output_message(task_id: &str, request_id: &str, text: String) -> api::Message {
    api::Message {
        id: Uuid::new_v4().to_string(),
        task_id: task_id.to_owned(),
        server_message_data: String::new(),
        citations: vec![],
        message: Some(api::message::Message::AgentOutput(
            api::message::AgentOutput { text },
        )),
        request_id: request_id.to_owned(),
        timestamp: None,
    }
}

fn make_user_query_message(task_id: &str, request_id: &str, query: String) -> api::Message {
    api::Message {
        id: Uuid::new_v4().to_string(),
        task_id: task_id.to_owned(),
        server_message_data: String::new(),
        citations: vec![],
        message: Some(api::message::Message::UserQuery(api::message::UserQuery {
            query,
            ..Default::default()
        })),
        request_id: request_id.to_owned(),
        timestamp: None,
    }
}

fn make_tool_call_result_message(
    task_id: &str,
    request_id: &str,
    tool_call_id: String,
    content: String,
) -> api::Message {
    // ToolCallResult 持久化:warp protobuf 的 `tool_call_result.result` oneof 都是
    // 结构化 variant(RunShellCommand / Grep / ReadFiles / ...),没有通用的字符串
    // 兜底 variant。BYOP 已经在 chat_stream 自己把 result 序列化为 JSON 字符串,
    // 不再需要按 warp 协议结构化 — 直接把字符串存到 `server_message_data` 这个
    // 自由字符串字段,并把 `result` oneof 留 None。下一轮 build_chat_request 在
    // `Message::ToolCallResult` 分支需要特判:result=None 时从 server_message_data
    // 读 content(否则走 tools::serialize_result 反序列化结构化 variant)。
    api::Message {
        id: Uuid::new_v4().to_string(),
        task_id: task_id.to_owned(),
        server_message_data: content,
        citations: vec![],
        message: Some(api::message::Message::ToolCallResult(
            api::message::ToolCallResult {
                tool_call_id,
                context: None,
                result: None,
            },
        )),
        request_id: request_id.to_owned(),
        timestamp: None,
    }
}

/// BYOP `from_args` 解析失败时,emit 占位 ToolCall 作 carrier:
/// `tool` oneof 留 None(没有合适的结构化 variant),原始 fn_name + args_str 编码到
/// `server_message_data` 为 `<fn_name>\n<args_str>`。下一轮 build_chat_request →
/// `serialize_outgoing_tool_call` 的 carrier 分支据此还原,保证上游模型看到的
/// tool_use name / args 与原 call 一致(否则用 "warp_internal_empty" 占位会让模型
/// 困惑,也对不上紧随的 ToolCallResult error 上下文)。
fn make_tool_call_carrier_message(
    task_id: &str,
    request_id: &str,
    tool_call_id: &str,
    fn_name: &str,
    args_str: &str,
) -> api::Message {
    let carrier = format!("{}\n{}", fn_name, args_str);
    api::Message {
        id: Uuid::new_v4().to_string(),
        task_id: task_id.to_owned(),
        server_message_data: carrier,
        citations: vec![],
        message: Some(api::message::Message::ToolCall(api::message::ToolCall {
            tool_call_id: tool_call_id.to_owned(),
            tool: None,
        })),
        request_id: request_id.to_owned(),
        timestamp: None,
    }
}

fn make_tool_call_message(
    task_id: &str,
    request_id: &str,
    tool_call_id: &str,
    tool: api::message::tool_call::Tool,
) -> api::Message {
    api::Message {
        id: Uuid::new_v4().to_string(),
        task_id: task_id.to_owned(),
        server_message_data: String::new(),
        citations: vec![],
        message: Some(api::message::Message::ToolCall(api::message::ToolCall {
            tool_call_id: tool_call_id.to_owned(),
            tool: Some(tool),
        })),
        request_id: request_id.to_owned(),
        timestamp: None,
    }
}

fn create_task_event(task_id: &str) -> api::ResponseEvent {
    api::ResponseEvent {
        r#type: Some(api::response_event::Type::ClientActions(
            api::response_event::ClientActions {
                actions: vec![api::ClientAction {
                    action: Some(api::client_action::Action::CreateTask(
                        api::client_action::CreateTask {
                            task: Some(api::Task {
                                id: task_id.to_owned(),
                                description: String::new(),
                                dependencies: None,
                                messages: vec![],
                                summary: String::new(),
                                server_data: String::new(),
                            }),
                        },
                    )),
                }],
            },
        )),
    }
}

/// 构造一条 `Action::CreateTask` 表示新 subtask,带 `dependencies.parent_task_id`。
/// conversation 在 `apply_client_action::CreateTask` 看到 `task.parent_id()` 非空 →
/// 走 `Task::new_subtask` 路径,从 parent.messages 找匹配的 subagent tool_call、
/// 抽 `SubagentParams` 挂到 subtask、emit `BlocklistAIHistoryEvent::CreatedSubtask`。
/// LRC tag-in 浮窗 spawn 链路依赖此事件。
fn create_subtask_event(subtask_id: &str, parent_task_id: &str) -> api::ResponseEvent {
    api::ResponseEvent {
        r#type: Some(api::response_event::Type::ClientActions(
            api::response_event::ClientActions {
                actions: vec![api::ClientAction {
                    action: Some(api::client_action::Action::CreateTask(
                        api::client_action::CreateTask {
                            task: Some(api::Task {
                                id: subtask_id.to_owned(),
                                description: String::new(),
                                dependencies: Some(api::task::Dependencies {
                                    parent_task_id: parent_task_id.to_owned(),
                                }),
                                messages: vec![],
                                summary: String::new(),
                                server_data: String::new(),
                            }),
                        },
                    )),
                }],
            },
        )),
    }
}

fn make_finished_done(
    usage_metadata: Option<api::response_event::stream_finished::ConversationUsageMetadata>,
) -> api::ResponseEvent {
    api::ResponseEvent {
        r#type: Some(api::response_event::Type::Finished(
            api::response_event::StreamFinished {
                reason: Some(api::response_event::stream_finished::Reason::Done(
                    api::response_event::stream_finished::Done {},
                )),
                conversation_usage_metadata: usage_metadata,
                token_usage: vec![],
                should_refresh_model_config: false,
                request_cost: None,
            },
        )),
    }
}

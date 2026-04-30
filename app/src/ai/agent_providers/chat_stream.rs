//! BYOP 模式下的 chat completion + tool calling 适配层。
//!
//! 把 `RequestParams` 翻译为 OpenAI 兼容 chat/completions 请求,调用用户配置的
//! provider base_url,收到响应后翻译回 `warp_multi_agent_api::ResponseEvent`,
//! 让 controller 自家逻辑(权限/弹窗/执行/result 回写/触发下一轮)接管闭环。
//!
//! ## 当前能力
//!
//! - 多轮: `params.tasks` 中所有历史消息 + `params.input` 当前输入 → OpenAI messages
//! - Tool calling: 内置 tools 见 `tools::REGISTRY`(目前: `run_shell_command`, `read_files`)
//! - 模型回 `tool_calls` → 翻译为 `Message::ToolCall` → controller 按 profile
//!   权限自动 ask / auto-approve / 执行 / 写 result → 自动触发下一轮 byop
//! - Tool result 在下一轮被序列化为 `role=tool, tool_call_id=...` 的 JSON content
//!
//! ## 限制(待后续扩展)
//!
//! - 不响应 cancel
//! - 不解析 token usage
//!
//! ## 流式实现(SSE)
//!
//! `stream=true` 后用 `create_stream_byot` 拿 SSE chunk,翻译成 warp 协议:
//! - 首次出现 content/reasoning_content → emit `AddMessagesToTask` 注册 message id;
//! - 后续 chunk → emit `AppendToMessageContent` + FieldMask 增量追加,UI 打字机效果;
//! - tool_calls 按 `index` 在 buffer 中累积,流结束后一次性 emit。

use std::collections::BTreeMap;
use std::sync::Arc;

use async_openai::{config::OpenAIConfig, Client as OpenAIClient};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;
use warp_multi_agent_api as api;

use crate::ai::agent::api::{RequestParams, ResponseStream};
use crate::ai::agent::AIAgentInput;
use crate::server::server_api::AIApiError;
use ai::agent::convert::ConvertToAPITypeError;

use super::openai_compatible::{normalize_base_url, OpenAiCompatibleError};
use super::tools;

// ---------------------------------------------------------------------------
// OpenAI 协议类型(子集)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<ToolDef>,
    stream: bool,
}

#[derive(Debug, Clone, Serialize)]
struct ChatMessage {
    role: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OutToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
    /// DeepSeek reasoner / 类似 o1 thinking 模式: 上一轮的 reasoning_content 必须
    /// 在下一轮的同一 assistant message 中带回去,否则 400。
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning_content: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct OutToolCall {
    id: String,
    #[serde(rename = "type")]
    kind: &'static str, // "function"
    function: OutFunction,
}

#[derive(Debug, Clone, Serialize)]
struct OutFunction {
    name: String,
    arguments: String, // JSON 字符串
}

#[derive(Debug, Clone, Serialize)]
struct ToolDef {
    #[serde(rename = "type")]
    kind: &'static str, // "function"
    function: ToolFunction,
}

#[derive(Debug, Clone, Serialize)]
struct ToolFunction {
    name: String,
    description: String,
    parameters: Value,
}

/// 仅在流结束 buffer → parse_incoming_tool_call 时合成使用。非流式 ChatResponse
/// 已弃用,所有响应统一走 SSE。
#[derive(Debug)]
#[allow(dead_code)]
struct RespToolCall {
    id: String,
    kind: String,
    function: RespFunction,
}

#[derive(Debug)]
struct RespFunction {
    name: String,
    arguments: String,
}

// ---- streaming(SSE chunk)----

#[derive(Debug, Deserialize)]
struct RespChunk {
    #[serde(default)]
    choices: Vec<ChoiceDelta>,
}

#[derive(Debug, Deserialize)]
struct ChoiceDelta {
    #[serde(default)]
    delta: Delta,
}

#[derive(Debug, Default, Deserialize)]
struct Delta {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    reasoning_content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<RespToolCallDelta>>,
}

#[derive(Debug, Deserialize)]
struct RespToolCallDelta {
    #[serde(default)]
    index: Option<u32>,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    function: Option<RespFunctionDelta>,
}

#[derive(Debug, Default, Deserialize)]
struct RespFunctionDelta {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    arguments: Option<String>,
}

#[derive(Default)]
struct ToolCallBuf {
    id: String,
    name: String,
    args: String,
}

// ---------------------------------------------------------------------------
// System prompt
// ---------------------------------------------------------------------------

const SYSTEM_PROMPT: &str = "你是一个内嵌于 Warp 终端的编程助手,具备调用工具的能力。\n\
\n\
## 工具总览\n\
\n\
- **run_shell_command**: 跑构建/启动服务/执行脚本/查系统状态。`is_read_only=true` 时(ls/cat/git status 等)自动通过审批,否则需用户确认。\n\
- **read_files**: 读文件全部或指定行段(1-based 闭区间)。改文件前**必须先 read 看到当前内容**。\n\
- **apply_file_diffs**: 批量 edit/create/delete 文件。edit 的 search 字段必须与文件中现有内容**完全一致**(含空白和换行),否则失败。\n\
- **grep**: 在路径下逐行字面/正则搜索关键字。比 shell 调 grep/rg 更快更安全(只读、自动通过)。\n\
- **file_glob**: 用通配符(`*`/`?`/`**`/`[…]`)查找文件路径。优先于 shell 的 find/ls。\n\
- **search_codebase**: 用自然语言对代码库做**语义化**搜索(基于 embeddings + 符号索引)。仅 Local 会话支持。不知道关键字、只有概念性描述时优先用。\n\
- **read_skill**: 读取 skill markdown 模板(用户预定义的工作流提示)。\n\
- **ask_user_question**: 指令含**真实歧义**且选错代价高时反问用户(单选/多选)。**严禁**用于「是否继续」这类琐碎确认。\n\
- **MCP tools**(`mcp__server__tool` / `mcp_read_resource`): 用户配置的 MCP server 暴露的能力。按 description 调用。\n\
\n\
## 长运行命令使用范式(关键)\n\
\n\
`run_shell_command` 默认 `wait_until_complete=true`(等命令结束才返回)。\n\
**dev server / watcher / `tail -f` / 交互 REPL 等不会自然退出的进程必须传 `wait_until_complete=false`**,\n\
否则当前 turn 会卡死永远等不到结果。\n\
\n\
返回的 `LongRunningCommandSnapshot` 含 `command_id`,后续 turn 用:\n\
- `read_shell_command_output(command_id, delay_seconds=N)`: N 秒内取当前输出快照(轮询推荐用法)\n\
- `read_shell_command_output(command_id)`(不填 delay): 阻塞等命令自然退出(dev server 不会退出,慎用)\n\
- `write_to_long_running_shell_command(command_id, input, mode=line)`: 发交互输入(默认加换行)\n\
- `write_to_long_running_shell_command(command_id, input=\"\\u0003\", mode=raw)`: 发 Ctrl-C 终止进程\n\
\n\
## 通用约定\n\
\n\
- 一次回答可以并行多个 tool_calls,但**避免明显冲突的并行**(例如同时改同一个文件)。\n\
- 改文件前先 read,改完不要再 read 一次复核(diff 应用本身已校验)。\n\
- 路径优先用相对路径(相对当前工作目录),除非已知绝对路径。\n\
- 不知道工作目录 / shell 类型时,先 `run_shell_command` 跑 `pwd` (is_read_only=true) 或 `echo $SHELL` 确认。\n\
\n\
## 回答风格\n\
\n\
- 用简洁的中文。\n\
- 在执行 tool 之前,简短说明计划(1 句话即可),然后发 tool_call。\n\
- tool 返回结果后,直接基于结果继续推进,不要复述结果细节(除非用户问到)。\n\
- 不要重复用户的话,不要写多余的「正在 / 已经 / 将会」开场白。";

// ---------------------------------------------------------------------------
// Multi-turn message 转换
// ---------------------------------------------------------------------------

/// 把 RequestParams 翻译为 OpenAI messages 数组。
///
/// 顺序: `params.tasks[*].messages` 按时间戳排序(同一个 task 内部已经按发生顺序),
/// 然后 append `params.input`(当前轮新输入)。
fn build_openai_messages(params: &RequestParams) -> Vec<ChatMessage> {
    let mut messages = vec![ChatMessage {
        role: "system",
        content: Some(SYSTEM_PROMPT.to_owned()),
        tool_calls: None,
        tool_call_id: None,
        reasoning_content: None,
    }];

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

    // 把同一 request_id 的 ToolCall 聚合到同一个 assistant message 的 tool_calls 数组。
    // OpenAI 要求 assistant 的 tool_calls 紧接着对应 tool_call_id 的 tool messages。
    // DeepSeek reasoner 模式还要求 reasoning_content 在同一 assistant 块中带回去。
    let mut pending_tool_calls: Vec<OutToolCall> = Vec::new();
    let mut pending_assistant_text: Option<String> = None;
    let mut pending_reasoning: Option<String> = None;

    let flush_assistant = |msgs: &mut Vec<ChatMessage>,
                           text: &mut Option<String>,
                           tcs: &mut Vec<OutToolCall>,
                           reasoning: &mut Option<String>| {
        // OpenAI 协议要求 assistant message 必须有 content 或 tool_calls 之一。
        // 只有 reasoning_content 的"残缺"消息(上一轮流被截断 / 模型只产出思考链
        // 没产出最终回答时会出现)直接丢弃,否则下一轮回放会被服务端 400 拒绝。
        if text.is_some() || !tcs.is_empty() {
            msgs.push(ChatMessage {
                role: "assistant",
                content: text.take(),
                tool_calls: if tcs.is_empty() {
                    None
                } else {
                    Some(std::mem::take(tcs))
                },
                tool_call_id: None,
                reasoning_content: reasoning.take(),
            });
        } else {
            text.take();
            tcs.clear();
            reasoning.take();
        }
    };

    for msg in all_msgs {
        let Some(inner) = &msg.message else {
            continue;
        };
        match inner {
            api::message::Message::UserQuery(u) => {
                flush_assistant(
                    &mut messages,
                    &mut pending_assistant_text,
                    &mut pending_tool_calls,
                    &mut pending_reasoning,
                );
                messages.push(ChatMessage {
                    role: "user",
                    content: Some(u.query.clone()),
                    tool_calls: None,
                    tool_call_id: None,
                    reasoning_content: None,
                });
            }
            api::message::Message::AgentReasoning(r) => {
                // DeepSeek reasoner 的思考链。挂到下一个 assistant block 上。
                if pending_assistant_text.is_some()
                    || !pending_tool_calls.is_empty()
                    || pending_reasoning.is_some()
                {
                    flush_assistant(
                        &mut messages,
                        &mut pending_assistant_text,
                        &mut pending_tool_calls,
                        &mut pending_reasoning,
                    );
                }
                pending_reasoning = Some(r.reasoning.clone());
            }
            api::message::Message::AgentOutput(a) => {
                if pending_assistant_text.is_some() {
                    flush_assistant(
                        &mut messages,
                        &mut pending_assistant_text,
                        &mut pending_tool_calls,
                        &mut pending_reasoning,
                    );
                }
                pending_assistant_text = Some(a.text.clone());
            }
            api::message::Message::ToolCall(tc) => {
                let (name, args) =
                    serialize_outgoing_tool_call(tc, params.mcp_context.as_ref());
                pending_tool_calls.push(OutToolCall {
                    id: tc.tool_call_id.clone(),
                    kind: "function",
                    function: OutFunction {
                        name,
                        arguments: args,
                    },
                });
            }
            api::message::Message::ToolCallResult(tcr) => {
                flush_assistant(
                    &mut messages,
                    &mut pending_assistant_text,
                    &mut pending_tool_calls,
                    &mut pending_reasoning,
                );
                messages.push(ChatMessage {
                    role: "tool",
                    content: Some(tools::serialize_result(tcr)),
                    tool_calls: None,
                    tool_call_id: Some(tcr.tool_call_id.clone()),
                    reasoning_content: None,
                });
            }
            _ => {
                // 其他 message 类型(SystemQuery/UpdateTodos/...)Phase 3a 暂不送上游。
            }
        }
    }
    flush_assistant(
        &mut messages,
        &mut pending_assistant_text,
        &mut pending_tool_calls,
        &mut pending_reasoning,
    );

    // 当前轮新输入 → 追加。
    for input in &params.input {
        match input {
            AIAgentInput::UserQuery { query, .. } => {
                messages.push(ChatMessage {
                    role: "user",
                    content: Some(query.clone()),
                    tool_calls: None,
                    tool_call_id: None,
                    reasoning_content: None,
                });
            }
            AIAgentInput::ActionResult { result, .. } => {
                // 关键: 上一轮模型回了 tool_calls,client 端执行完后 result 走
                // `params.input` 而不是 `params.tasks` 的 message 历史。如果不在
                // 这里把 ActionResult 序列化为 `role=tool` message,OpenAI 协议
                // (DeepSeek/官方都严格校验)会因为缺少 tool_call_id 对应的 tool
                // message 而 400: "An assistant message with 'tool_calls' must be
                // followed by tool messages responding to each 'tool_call_id'".
                let tool_call_id = result.id.to_string();
                // 走 per-tool 结构化序列化:LongRunningCommandSnapshot 等必须保留
                // command_id / output / is_alt_screen_active 字段,否则下一轮模型
                // 拿不到 command_id 没法继续 read/write_to_long_running_*,长运行
                // 命令工具完全不可用。Display fallback 仅用于注册表里未覆盖的
                // ActionResult variant(如 SuggestPrompt/UploadArtifact 等)。
                let content = tools::serialize_action_result(result).unwrap_or_else(|| {
                    serde_json::json!({ "result": result.result.to_string() }).to_string()
                });
                messages.push(ChatMessage {
                    role: "tool",
                    content: Some(content),
                    tool_calls: None,
                    tool_call_id: Some(tool_call_id),
                    reasoning_content: None,
                });
            }
            AIAgentInput::InvokeSkill {
                skill, user_query, ..
            } => {
                // Skill 是用户预定义的 prompt 模板(markdown 文件)。把 skill 内容
                // + 可选的 user_query 拼成一条 user message 喂给上游 — 这样自定义
                // provider 路径下也能走 warp 的 skill 工作流。
                let mut composed = format!(
                    "请按下面的技能 \"{}\" 指引执行任务:\n\n{}\n\n---\n",
                    skill.name, skill.content,
                );
                if let Some(uq) = user_query {
                    composed.push_str(&format!("用户进一步指令: {}", uq.query));
                }
                messages.push(ChatMessage {
                    role: "user",
                    content: Some(composed),
                    tool_calls: None,
                    tool_call_id: None,
                    reasoning_content: None,
                });
            }
            AIAgentInput::ResumeConversation { .. }
            | AIAgentInput::AutoCodeDiffQuery { .. }
            | AIAgentInput::CreateNewProject { .. }
            | AIAgentInput::CodeReview { .. }
            | AIAgentInput::SummarizeConversation { .. } => {
                // 暂时忽略
            }
            _ => {}
        }
    }

    // 最后做一次 sanitize: 确保每个 assistant.tool_calls 后面都跟着对应每个
    // tool_call_id 的 role=tool message。
    //
    // 背景: warp 自家协议在 tool 执行后,result 不一定保留为 task.messages 中的
    // `Message::ToolCallResult` — 实测发现是被"消化"成下一条 AgentOutput
    // (模型基于 result 生成的总结)。OpenAI 协议则严格要求 tool_calls 紧跟
    // 对应 tool messages,否则 400.
    //
    // 注意必须在 params.input 处理 *之后* 跑(否则 ActionResult 转出的 tool
    // message 还没 push,sanitize 会误认为缺失,合成占位,然后 ActionResult
    // 又 push 一个真实 tool — 同一 id 出现两次,触发 "Messages with role
    // 'tool' must be a response to a preceding message with 'tool_calls'").
    sanitize_tool_call_pairs(&mut messages);

    messages
}

/// 扫描 messages,对每个 assistant.tool_calls 块,确保其后面紧跟对应每个
/// tool_call_id 的 role=tool message。缺失的合成 placeholder。
///
/// 算法:从前往后单次扫描。遇到一个含 tool_calls 的 assistant 时:
///   1. 记下它的所有 ids
///   2. 收集紧随其后的所有 role=tool messages 的 tool_call_id 集合(直到下一个
///      非 tool message 出现)
///   3. 对 ids 中没出现在收集集合的,在该 assistant 后立即插入 placeholder。
///   4. 若紧随 assistant 的下一个 message 是另一个 assistant 含 content(典型:
///      warp 把 tool 结果消化成的总结),把它的 content 取来作为 placeholder
///      content,这样上游模型至少能"看到"上一轮 tool 调用产生了什么效果。
fn sanitize_tool_call_pairs(messages: &mut Vec<ChatMessage>) {
    use std::collections::HashSet;

    let mut i = 0;
    while i < messages.len() {
        // 仅处理含 tool_calls 的 assistant
        let ids: Vec<String> = match &messages[i].tool_calls {
            Some(tcs) if messages[i].role == "assistant" && !tcs.is_empty() => {
                tcs.iter().map(|tc| tc.id.clone()).collect()
            }
            _ => {
                i += 1;
                continue;
            }
        };

        // 扫描后续紧跟的 role=tool 消息,收集已经回应的 ids;
        // 同时记录第一条非 tool 消息(若存在),用作 placeholder content 的来源。
        let mut covered: HashSet<String> = HashSet::new();
        let mut scan = i + 1;
        while scan < messages.len() && messages[scan].role == "tool" {
            if let Some(id) = &messages[scan].tool_call_id {
                covered.insert(id.clone());
            }
            scan += 1;
        }

        // 找缺失的 ids
        let missing: Vec<String> = ids.into_iter().filter(|id| !covered.contains(id)).collect();
        if missing.is_empty() {
            i = scan;
            continue;
        }

        // 占位 content 来源: 紧随的下一个 assistant message(典型场景:
        // warp 把 tool 结果消化成的下一轮 AgentOutput)的 content。
        let placeholder_content: String = messages
            .get(scan)
            .filter(|m| m.role == "assistant")
            .and_then(|m| m.content.clone())
            .map(|c| format!("(tool 执行结果已被对话消化为助手回答,内容摘要: {c})"))
            .unwrap_or_else(|| "(tool 执行结果未保留)".to_owned());

        // 在 i+1 位置插入所有缺失 id 的占位 tool message
        let inserts: Vec<ChatMessage> = missing
            .into_iter()
            .map(|id| ChatMessage {
                role: "tool",
                content: Some(placeholder_content.clone()),
                tool_calls: None,
                tool_call_id: Some(id),
                reasoning_content: None,
            })
            .collect();
        let inserted = inserts.len();
        messages.splice(i + 1..i + 1, inserts);

        // 跳过 assistant + 已 cover + 新插入的占位
        i = i + 1 + (scan - (i + 1)) + inserted;
    }
}

/// 反向: 把内部 `tool_call::Tool` variant 序列化成 OpenAI tool_calls 中的
/// `(function.name, function.arguments)` 对,用于多轮历史回放。
///
/// 这里的 (name, args_json) 必须与 `tools::REGISTRY` 中各 tool 的 `name` 与
/// `from_args` 期望的 schema 严格对齐 — 否则历史的"模型曾调用过 X"信息会
/// 退化为 unknown_tool,新一轮请求时模型上下文不完整。
fn serialize_outgoing_tool_call(
    tc: &api::message::ToolCall,
    mcp_ctx: Option<&crate::ai::agent::MCPContext>,
) -> (String, String) {
    use api::message::tool_call::Tool;
    match &tc.tool {
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
                            Some(api::ask_user_question::question::QuestionType::MultipleChoice(mc)) => (
                                mc.options.iter().map(|o| o.label.clone()).collect::<Vec<_>>(),
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
            // 注: v4a_updates 没有反向序列化(我们的 schema 也没暴露),回放时会丢
            // 失这部分信息;若用户在 server-side 已存在的 v4a 历史回到 byop 路径,
            // 模型只能看到部分 ops。Phase 4 若加 v4a 支持需同步在这里追加。
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
        _ => (
            // 真正未支持的 tool variant(SuggestPlan / UseComputer / SearchCodebase
            // 等暂未接入): 仍旧给一个占位让上游知道有调用过,但 args 是空。
            "unknown_tool".to_owned(),
            "{}".to_owned(),
        ),
    }
}

// ---------------------------------------------------------------------------
// 主流程
// ---------------------------------------------------------------------------

fn build_tools_array(params: &RequestParams) -> Vec<ToolDef> {
    let mut out: Vec<ToolDef> = tools::REGISTRY
        .iter()
        .map(|t| ToolDef {
            kind: "function",
            function: ToolFunction {
                name: t.name.to_owned(),
                description: t.description.to_owned(),
                parameters: (t.parameters)(),
            },
        })
        .collect();
    // 动态注入 MCP server 暴露的 tools
    if let Some(ctx) = params.mcp_context.as_ref() {
        for (name, description, parameters) in tools::mcp::build_mcp_tool_defs(ctx) {
            out.push(ToolDef {
                kind: "function",
                function: ToolFunction {
                    name,
                    description,
                    parameters,
                },
            });
        }
    }
    out
}

/// 构造 OpenAI 兼容 client + 准备好流式请求 body。
fn build_client_and_body(
    base_url: &str,
    api_key: &str,
    model_id: &str,
    messages: Vec<ChatMessage>,
    tools_array: Vec<ToolDef>,
) -> Result<(OpenAIClient<OpenAIConfig>, Value), OpenAiCompatibleError> {
    // base_url 规范化(剔除尾 /,确保 http/https) — async-openai 的 with_api_base
    // 期望不含 /chat/completions 后缀,我们传规范化后的 base 即可。
    let base = normalize_base_url(base_url)?;
    let mut config = OpenAIConfig::new().with_api_base(base);
    if !api_key.trim().is_empty() {
        config = config.with_api_key(api_key);
    }
    let client = OpenAIClient::with_config(config);

    // BYOT (Bring Your Own Type): 直接传/收 serde_json::Value,绕过 SDK 的强类型,
    // 这样 DeepSeek 的 `reasoning_content`、OpenRouter 的 `provider` 等非标准
    // 字段都能透传/读取。请求体仍用我们的 ChatRequest 序列化(serde::Serialize)。
    let body = ChatRequest {
        model: model_id,
        messages,
        tools: tools_array,
        stream: true,
    };
    let body_value = serde_json::to_value(&body)
        .map_err(|e| OpenAiCompatibleError::Decode(format!("serialize request: {e}")))?;

    // 调试: 打印最终发出去的 messages 序列,定位"tool_calls 后没跟 tool message"
    // 之类的协议违例。后续稳定后可以删除或降级到 trace。
    if let Some(messages) = body_value.get("messages") {
        log::warn!(
            "[byop] outgoing messages: {}",
            serde_json::to_string(messages).unwrap_or_default()
        );
    }

    Ok((client, body_value))
}

/// 把 async-openai 的 `OpenAIError` 映射回我们的 `OpenAiCompatibleError`。
/// 不同 SDK 版本的 variant 集合差异较大,这里直接用 Display 文本携带诊断信息,
/// 让 retry/UI 看到完整错误链。后续如果要按 status code 做精细分支(401/429),
/// 可以解析 Display 字符串或升级到 SDK 更稳定的 API。
fn map_openai_error(err: async_openai::error::OpenAIError) -> OpenAiCompatibleError {
    OpenAiCompatibleError::Decode(format!("{err}"))
}

/// `task_id`: conversation 的 root task id(controller 端从 history model 取)。
/// `needs_create_task`: 仅首轮(root 还是 Optimistic)需要 emit `CreateTask`。
pub async fn generate_byop_output(
    params: RequestParams,
    base_url: String,
    api_key: String,
    model_id: String,
    task_id: String,
    needs_create_task: bool,
    _cancellation_rx: futures::channel::oneshot::Receiver<()>,
) -> Result<ResponseStream, ConvertToAPITypeError> {
    let messages = build_openai_messages(&params);
    let tools_array = build_tools_array(&params);
    let conversation_id = params
        .conversation_token
        .as_ref()
        .map(|t| t.as_str().to_string())
        .unwrap_or_default();
    let request_id = Uuid::new_v4().to_string();
    let mcp_context = params.mcp_context.clone();

    // build client / body 同步阶段;失败立即作为单 Err event 发回。
    let prepared = build_client_and_body(&base_url, &api_key, &model_id, messages, tools_array);

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

        let (client, body_value) = match prepared {
            Ok(p) => p,
            Err(e) => {
                log::error!("BYOP prepare request failed: {e:#}");
                yield Err(Arc::new(AIApiError::Other(anyhow::anyhow!(
                    "BYOP prepare failed: {e}"
                ))));
                return;
            }
        };

        let mut sdk_stream = match client
            .chat()
            .create_stream_byot::<Value, Value>(body_value)
            .await
        {
            Ok(s) => s,
            Err(e) => {
                let mapped = map_openai_error(e);
                log::error!("BYOP open stream failed: {mapped:#}");
                yield Err(Arc::new(AIApiError::Other(anyhow::anyhow!(
                    "BYOP open stream failed: {mapped}"
                ))));
                return;
            }
        };

        // 2) 首轮:CreateTask 升级 Optimistic root → Server。在拿到 chunk 之前 emit,
        //    保证后续 AddMessages/Append 都落到 Server 状态的 root task。
        if needs_create_task {
            yield Ok(create_task_event(&task_id));
        }

        // 流式状态:文本 / 推理各自的 message id 在第一次 chunk 到达时生成,
        // 之后的 chunk 走 AppendToMessageContent 增量追加。
        let mut text_msg_id: Option<String> = None;
        let mut reasoning_msg_id: Option<String> = None;
        // tool_call 必须按 index 累积 — name/id 通常仅在第一个 chunk 中出现,
        // arguments 则跨多 chunk 拼接,在流结束后一次性 parse + emit。
        let mut tool_bufs: BTreeMap<u32, ToolCallBuf> = BTreeMap::new();

        while let Some(item) = sdk_stream.next().await {
            let value = match item {
                Ok(v) => v,
                Err(e) => {
                    let mapped = map_openai_error(e);
                    log::error!("BYOP stream chunk error: {mapped:#}");
                    yield Err(Arc::new(AIApiError::Other(anyhow::anyhow!(
                        "BYOP stream error: {mapped}"
                    ))));
                    return;
                }
            };
            let chunk: RespChunk = match serde_json::from_value(value) {
                Ok(c) => c,
                Err(e) => {
                    log::warn!("BYOP failed to parse chunk: {e}");
                    continue;
                }
            };
            let Some(choice) = chunk.choices.into_iter().next() else {
                continue;
            };
            let delta = choice.delta;

            // reasoning_content(DeepSeek reasoner 等)。先于文本输出,UI 显示思考块。
            if let Some(reasoning) = delta.reasoning_content.filter(|s| !s.is_empty()) {
                if let Some(id) = reasoning_msg_id.clone() {
                    yield Ok(make_append_event(&task_id, &id, AppendKind::Reasoning(reasoning)));
                } else {
                    let new_id = Uuid::new_v4().to_string();
                    let mut msg = make_reasoning_message(&task_id, &request_id, reasoning);
                    msg.id = new_id.clone();
                    reasoning_msg_id = Some(new_id);
                    yield Ok(make_add_messages_event(&task_id, vec![msg]));
                }
            }

            // 普通文本 content。
            if let Some(content) = delta.content.filter(|s| !s.is_empty()) {
                if let Some(id) = text_msg_id.clone() {
                    yield Ok(make_append_event(&task_id, &id, AppendKind::Text(content)));
                } else {
                    let new_id = Uuid::new_v4().to_string();
                    let mut msg = make_agent_output_message(&task_id, &request_id, content);
                    msg.id = new_id.clone();
                    text_msg_id = Some(new_id);
                    yield Ok(make_add_messages_event(&task_id, vec![msg]));
                }
            }

            // tool_calls deltas — 仅累积,不流式 emit(controller 期望完整 args)。
            if let Some(tcs) = delta.tool_calls {
                for tc in tcs {
                    let idx = tc.index.unwrap_or(0);
                    let buf = tool_bufs.entry(idx).or_default();
                    if let Some(id) = tc.id {
                        if !id.is_empty() {
                            buf.id = id;
                        }
                    }
                    if let Some(f) = tc.function {
                        if let Some(n) = f.name {
                            if !n.is_empty() {
                                buf.name = n;
                            }
                        }
                        if let Some(a) = f.arguments {
                            buf.args.push_str(&a);
                        }
                    }
                }
            }
        }

        // 流结束:把累积的 tool_calls 一次性发出。
        let mut final_messages: Vec<api::Message> = Vec::new();
        for (_idx, buf) in tool_bufs.into_iter() {
            if buf.name.is_empty() {
                continue;
            }
            let synth = RespToolCall {
                id: buf.id.clone(),
                kind: "function".to_owned(),
                function: RespFunction {
                    name: buf.name.clone(),
                    arguments: buf.args.clone(),
                },
            };
            match parse_incoming_tool_call(&synth, mcp_context.as_ref()) {
                Ok(warp_tool) => {
                    final_messages.push(make_tool_call_message(
                        &task_id,
                        &request_id,
                        &buf.id,
                        warp_tool,
                    ));
                }
                Err(e) => {
                    log::warn!("BYOP: failed to parse tool_call args for {}: {e:#}", buf.name);
                    final_messages.push(make_agent_output_message(
                        &task_id,
                        &request_id,
                        format!("(byop:工具 `{}` 的参数解析失败: {})", buf.name, e),
                    ));
                }
            }
        }
        if !final_messages.is_empty() {
            yield Ok(make_add_messages_event(&task_id, final_messages));
        }

        yield Ok(make_finished_done());
    };

    Ok(Box::pin(stream))
}

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
            // FieldMask 路径用 oneof variant 字段名,不要带 oneof 自身的名字 `message` —
            // prost-reflect 的 `get_field_by_name` 不把 oneof 暴露为字段,
            // `field_mask::apply_path` 在路径段找不到字段时静默 no-op
            // (crates/field_mask/src/lib.rs:103-110),导致所有 append 丢失,UI 只看到首个 chunk。
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
    tc: &RespToolCall,
    mcp_ctx: Option<&crate::ai::agent::MCPContext>,
) -> anyhow::Result<api::message::tool_call::Tool> {
    // MCP 工具走前缀路由
    if tools::mcp::is_mcp_function(&tc.function.name) {
        return tools::mcp::parse_mcp_tool_call(&tc.function.name, &tc.function.arguments, mcp_ctx);
    }
    // 静态注册的内置工具
    let Some(tool) = tools::lookup(&tc.function.name) else {
        anyhow::bail!("unknown tool name: {}", tc.function.name);
    };
    (tool.from_args)(&tc.function.arguments)
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

fn make_finished_done() -> api::ResponseEvent {
    api::ResponseEvent {
        r#type: Some(api::response_event::Type::Finished(
            api::response_event::StreamFinished {
                reason: Some(api::response_event::stream_finished::Reason::Done(
                    api::response_event::stream_finished::Done {},
                )),
                conversation_usage_metadata: None,
                token_usage: vec![],
                should_refresh_model_config: false,
                request_cost: None,
            },
        )),
    }
}

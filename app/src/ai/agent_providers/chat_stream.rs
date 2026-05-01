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
    ChatMessage, ChatOptions, ChatRequest, ChatStreamEvent, Tool as GenaiTool, ToolCall,
    ToolResponse,
};
use genai::resolver::{AuthData, Endpoint, ServiceTargetResolver};
use genai::{Client, ModelIden, ServiceTarget};

use crate::ai::agent::api::{RequestParams, ResponseStream};
use crate::ai::agent::AIAgentInput;
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

// ---------------------------------------------------------------------------
// Multi-turn message 转换
// ---------------------------------------------------------------------------

/// 累积同一 assistant turn 的 text + tool_calls + reasoning,然后 flush 成一个或两个
/// `ChatMessage`(text 一个,tool_calls 一个 — genai 把它们建模为分开的 message)。
///
/// **DeepSeek thinking-mode 关键**:assistant turn 含 tool_calls 时,
/// 必须用 `ChatMessage::with_reasoning_content(Some(reasoning))` 把上一轮的
/// reasoning_content 字段回传,否则 DeepSeek 服务端 400(genai 0.6 才有此 API,
/// 0.5.3 是 issue #138 — 见 Cargo.toml 注释)。
#[derive(Default)]
struct AssistantBuffer {
    text: Option<String>,
    tool_calls: Vec<ToolCall>,
    /// 上一轮 AgentReasoning(thinking 链)。flush 时挂到对应 assistant message
    /// 的 reasoning_content 字段(genai 内部按 adapter 序列化:DeepSeek/Kimi 走 reasoning_content,
    /// Anthropic 走 thinking blocks)。
    reasoning: Option<String>,
}

impl AssistantBuffer {
    fn flush_into(&mut self, messages: &mut Vec<ChatMessage>) {
        let reasoning = self.reasoning.take();
        let has_tool_calls = !self.tool_calls.is_empty();
        if let Some(t) = self.text.take() {
            let mut msg = ChatMessage::assistant(t);
            // 仅当本 turn 没有 tool_calls 时才把 reasoning 挂到 text message;
            // 有 tool_calls 时 reasoning 必须跟 tool_calls 在同一 message
            // (DeepSeek 服务端只对 含 tool_calls 的 assistant 强制要求 reasoning_content)。
            if !has_tool_calls {
                if let Some(r) = reasoning.as_deref().filter(|s| !s.is_empty()) {
                    msg = msg.with_reasoning_content(Some(r.to_owned()));
                }
            }
            messages.push(msg);
        }
        if has_tool_calls {
            // genai `From<Vec<ToolCall>> for ChatMessage` 自动产 assistant role +
            // MessageContent::from_tool_calls。
            let mut msg = ChatMessage::from(std::mem::take(&mut self.tool_calls));
            if let Some(r) = reasoning.filter(|s| !s.is_empty()) {
                msg = msg.with_reasoning_content(Some(r));
            }
            messages.push(msg);
        }
    }
}

/// 把 RequestParams 翻译为 genai `ChatRequest`(含 system + messages + tools)。
fn build_chat_request(params: &RequestParams) -> ChatRequest {
    let agent_ctx = latest_input_context(&params.input);
    let system_text = prompt_renderer::render_system(&params.model, agent_ctx);

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

    let mut buf = AssistantBuffer::default();

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
                let (name, args_json) =
                    serialize_outgoing_tool_call(tc, params.mcp_context.as_ref());
                buf.tool_calls.push(ToolCall {
                    call_id: tc.tool_call_id.clone(),
                    fn_name: name,
                    fn_arguments: args_json,
                    thought_signatures: None,
                });
            }
            api::message::Message::ToolCallResult(tcr) => {
                buf.flush_into(&mut messages);
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
            AIAgentInput::UserQuery { query, context, .. } => {
                // 当前轮 UserQuery 自带的附件类 context(Block / SelectedText / File / Image)
                // 严格对齐 warp 自家路径走 `api::InputContext.executed_shell_commands` 等字段
                // 上行后由后端注入 prompt 的语义。BYOP 没有后端这层,直接 prepend 到 user message。
                // 环境型 context(env / git / skills / ...)由 prompt_renderer 渲染进 system,
                // 与本路径不重叠。
                let full_text = match user_context::render_user_attachments(context) {
                    Some(prefix) => format!("{prefix}\n\n{query}"),
                    None => query.clone(),
                };
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

    let mut req = ChatRequest::from_messages(messages).with_system(system_text);
    if !tools_array.is_empty() {
        req = req.with_tools(tools_array);
    }
    req
}

/// 扫描 messages,对每个 assistant 含 tool_calls 的 message,确保其后面紧跟对应每个
/// call_id 的 tool message。缺失的合成 placeholder。
fn sanitize_tool_call_pairs(messages: &mut Vec<ChatMessage>) {
    use std::collections::HashSet;

    let mut i = 0;
    while i < messages.len() {
        // 仅处理 assistant 且 content 含 tool_calls 的 message
        let ids: Vec<String> = match (&messages[i].role, messages[i].content.tool_calls()) {
            (genai::chat::ChatRole::Assistant, calls) if !calls.is_empty() => {
                calls.iter().map(|tc| tc.call_id.clone()).collect()
            }
            _ => {
                i += 1;
                continue;
            }
        };

        // 收集后面紧跟的 Tool 消息已 cover 的 call_id
        let mut covered: HashSet<String> = HashSet::new();
        let mut scan = i + 1;
        while scan < messages.len() && messages[scan].role == genai::chat::ChatRole::Tool {
            for resp in messages[scan].content.tool_responses() {
                covered.insert(resp.call_id.clone());
            }
            scan += 1;
        }

        let missing: Vec<String> = ids.into_iter().filter(|id| !covered.contains(id)).collect();
        if missing.is_empty() {
            i = scan;
            continue;
        }

        // 占位 content 来源:紧随的下一个 assistant message 的 first_text(典型场景:
        // warp 把 tool 结果消化成的下一轮 AgentOutput)。
        let placeholder_content: String = messages
            .get(scan)
            .filter(|m| m.role == genai::chat::ChatRole::Assistant)
            .and_then(|m| m.content.first_text().map(str::to_owned))
            .map(|c| format!("(tool 执行结果已被对话消化为助手回答,内容摘要: {c})"))
            .unwrap_or_else(|| "(tool 执行结果未保留)".to_owned());

        let inserts: Vec<ChatMessage> = missing
            .into_iter()
            .map(|id| ChatMessage::from(ToolResponse::new(id, placeholder_content.clone())))
            .collect();
        let inserted = inserts.len();
        messages.splice(i + 1..i + 1, inserts);

        i = i + 1 + (scan - (i + 1)) + inserted;
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
) -> (String, Value) {
    use api::message::tool_call::Tool;
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
            (
                format!("warp_internal_{}", variant_name),
                "{}".to_owned(),
            )
        }
        None => ("warp_internal_empty".to_owned(), "{}".to_owned()),
    };
    let args_value: Value = serde_json::from_str(&args_str).unwrap_or(Value::Object(Default::default()));
    (name, args_value)
}

// ---------------------------------------------------------------------------
// Tools 数组
// ---------------------------------------------------------------------------

fn build_tools_array(params: &RequestParams) -> Vec<GenaiTool> {
    let mut out: Vec<GenaiTool> = tools::REGISTRY.iter().map(|t| t.to_genai_tool()).collect();

    if let Some(ctx) = params.mcp_context.as_ref() {
        for (name, description, parameters) in tools::mcp::build_mcp_tool_defs(ctx) {
            out.push(
                GenaiTool::new(name)
                    .with_description(description)
                    .with_schema(parameters),
            );
        }
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
    log::info!(
        "[byop] build_client: adapter={adapter_kind:?} endpoint_url={endpoint_url}"
    );
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
    OpenAiCompatibleError::Decode(format!("{err}"))
}

// ---------------------------------------------------------------------------
// 主流程
// ---------------------------------------------------------------------------

/// 标题生成所需的 BYOP 配置。可能与主请求同 provider 也可能不同(用户在 Profile
/// Editor 里独立选了 title_model)。`None` 时跳过摘要步骤。
pub struct TitleGenInput {
    pub base_url: String,
    pub api_key: String,
    pub model_id: String,
    pub api_type: AgentProviderApiType,
    pub reasoning_effort: crate::settings::ReasoningEffortSetting,
}

/// `task_id`: conversation 的 root task id(controller 端从 history model 取)。
/// `needs_create_task`: 仅首轮(root 还是 Optimistic)需要 emit `CreateTask`。
/// `title_gen`: 仅首轮且 active title_model 可解析为 BYOP 时填充;非 None 时
/// 在主流程结束后单独发一次摘要请求,把 task description(= 会话标题)写回。
pub async fn generate_byop_output(
    params: RequestParams,
    base_url: String,
    api_key: String,
    model_id: String,
    api_type: AgentProviderApiType,
    reasoning_effort: crate::settings::ReasoningEffortSetting,
    task_id: String,
    needs_create_task: bool,
    title_gen: Option<TitleGenInput>,
    _cancellation_rx: futures::channel::oneshot::Receiver<()>,
) -> Result<ResponseStream, ConvertToAPITypeError> {
    let chat_req = build_chat_request(&params);
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
        let mut persistence_messages: Vec<api::Message> = Vec::new();
        for q in &pending_user_queries {
            persistence_messages.push(make_user_query_message(&task_id, &request_id, q.clone()));
        }
        for (call_id, content) in &pending_tool_results {
            persistence_messages.push(make_tool_call_result_message(
                &task_id,
                &request_id,
                call_id.clone(),
                content.clone(),
            ));
        }
        if !persistence_messages.is_empty() {
            yield Ok(make_add_messages_event(&task_id, persistence_messages));
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

        while let Some(item) = sdk_stream.next().await {
            let event = match item {
                Ok(ev) => ev,
                Err(e) => {
                    let mapped = map_genai_error(e);
                    log::error!("[byop] stream chunk error: {mapped:#}");
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
                        yield Ok(make_append_event(&task_id, &id, AppendKind::Text(c.content)));
                    } else {
                        let new_id = Uuid::new_v4().to_string();
                        let mut msg = make_agent_output_message(&task_id, &request_id, c.content);
                        msg.id = new_id.clone();
                        text_msg_id = Some(new_id);
                        yield Ok(make_add_messages_event(&task_id, vec![msg]));
                    }
                }
                ChatStreamEvent::Chunk(_) => {}
                ChatStreamEvent::ReasoningChunk(c) if !c.content.is_empty() => {
                    reasoning_count += 1;
                    reasoning_bytes += c.content.len();
                    if let Some(id) = reasoning_msg_id.clone() {
                        yield Ok(make_append_event(&task_id, &id, AppendKind::Reasoning(c.content)));
                    } else {
                        let new_id = Uuid::new_v4().to_string();
                        let mut msg = make_reasoning_message(&task_id, &request_id, c.content);
                        msg.id = new_id.clone();
                        reasoning_msg_id = Some(new_id);
                        yield Ok(make_add_messages_event(&task_id, vec![msg]));
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
                        &task_id,
                        &request_id,
                        &call.call_id,
                        warp_tool,
                    ));
                }
                Err(e) => {
                    log::warn!("BYOP: failed to parse tool_call args for {}: {e:#}", call.fn_name);
                    final_messages.push(make_agent_output_message(
                        &task_id,
                        &request_id,
                        format!("(byop:工具 `{}` 的参数解析失败: {})", call.fn_name, e),
                    ));
                }
            }
        }
        if !final_messages.is_empty() {
            yield Ok(make_add_messages_event(&task_id, final_messages));
        }

        // 标题生成:首轮在所有主流消息发完之后,用 title_model 单独发一次短请求,
        // 把生成的简短标题作为 `Action::UpdateTaskDescription` 注入下游 conversation,
        // 这样 `task.description()` 非空,`AIConversation::title()` 优先返回它。
        if let Some(tg) = title_gen.as_ref() {
            if let Some(query) = pending_user_queries.first().cloned() {
                match generate_title_via_byop(tg, &query).await {
                    Ok(Some(title)) => {
                        log::info!("[byop] title generated: {title:?}");
                        yield Ok(make_update_task_description_event(&task_id, title));
                    }
                    Ok(None) => {
                        log::warn!("[byop] title gen returned empty content; skip");
                    }
                    Err(e) => {
                        log::warn!("[byop] title gen failed: {e:#}; skip");
                    }
                }
            }
        }

        yield Ok(make_finished_done());
    };

    Ok(Box::pin(stream))
}

/// 用独立 BYOP 配置发一个短的非工具请求,要求模型对首条 user query 输出一个
/// 5–10 词的会话标题。所有错误吞掉(返回 Err 让上游打 warn log,不影响主流程)。
///
/// 实现委托给 `oneshot::byop_oneshot_completion`,这里只负责拼 prompt 和清洗输出。
async fn generate_title_via_byop(
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
    // 中英双语都覆盖的 system,要求 plain text(无引号、无 markdown)。
    let system = "You generate concise conversation titles. \
                  Reply with ONLY a 4-8 word title (no quotes, no punctuation at the end, no markdown). \
                  Match the language of the user's message. \
                  Do not answer the question — just title it.";
    let opts = super::oneshot::OneshotOptions {
        max_chars: Some(1000),
        ..Default::default()
    };
    let raw = super::oneshot::byop_oneshot_completion(&cfg, system, user_query, &opts).await?;
    Ok(sanitize_title(&raw))
}

/// 清洗 title 文本:trim、剥引号/反引号、去尾标点、截断到 80 字符上限。
/// 空字符串 → None(让上游跳过 emit)。
fn sanitize_title(raw: &str) -> Option<String> {
    let mut s = raw.trim().to_owned();
    // 剥首尾引号(中英文)。
    let quotes = ['"', '\'', '`', '“', '”', '‘', '’', '《', '》'];
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
    // 去尾标点。
    while let Some(c) = s.chars().last() {
        if matches!(c, '.' | '。' | '!' | '!' | '?' | '?' | ',' | ',' | ';' | ';') {
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
    // 80 字符截断(按 char 而不是 byte,保护 CJK)。
    let truncated: String = s.chars().take(80).collect();
    Some(truncated)
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

fn make_update_task_description_event(task_id: &str, description: String) -> api::ResponseEvent {
    api::ResponseEvent {
        r#type: Some(api::response_event::Type::ClientActions(
            api::response_event::ClientActions {
                actions: vec![api::ClientAction {
                    action: Some(api::client_action::Action::UpdateTaskDescription(
                        api::client_action::UpdateTaskDescription {
                            task_id: task_id.to_owned(),
                            description,
                        },
                    )),
                }],
            },
        )),
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

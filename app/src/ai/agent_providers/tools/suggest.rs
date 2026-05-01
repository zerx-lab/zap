//! 提示用户类工具:`suggest_new_conversation` / `suggest_prompt`。
//!
//! 这两个 tool 都是**纯本地 channel 信号** + UI 弹窗 — 模型主动建议某个动作,
//! 用户在 UI 接受/拒绝,executor 等用户决定后回写 result。不依赖任何 server。

use anyhow::Result;
use serde::Deserialize;
use serde_json::{json, Value};
use warp_multi_agent_api as api;

use super::OpenAiTool;

// ---------------------------------------------------------------------------
// suggest_new_conversation
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct NewConvArgs {
    /// 当前 assistant message 的 id(模型若不知道可传空字符串,controller 会兜底)。
    #[serde(default)]
    message_id: String,
}

fn new_conv_parameters() -> Value {
    json!({
        "type": "object",
        "properties": {
            "message_id": {
                "type": "string",
                "description": "可选: 从哪条 assistant message 处分支新对话(留空则用当前 message)。"
            }
        },
        "additionalProperties": false
    })
}

fn new_conv_from_args(args: &str) -> Result<api::message::tool_call::Tool> {
    let parsed: NewConvArgs = if args.trim().is_empty() {
        NewConvArgs {
            message_id: String::new(),
        }
    } else {
        serde_json::from_str(args)?
    };
    Ok(api::message::tool_call::Tool::SuggestNewConversation(
        api::message::tool_call::SuggestNewConversation {
            message_id: parsed.message_id,
        },
    ))
}

fn new_conv_result_to_json(result: &api::message::tool_call_result::Result) -> Option<Value> {
    use api::message::tool_call_result::Result as R;
    use api::suggest_new_conversation_result::Result as SR;
    let r = match result {
        R::SuggestNewConversation(r) => r,
        _ => return None,
    };
    let value = match &r.result {
        Some(SR::Accepted(a)) => json!({ "status": "accepted", "message_id": a.message_id }),
        Some(SR::Rejected(_)) => json!({ "status": "rejected" }),
        None => json!({ "status": "cancelled" }),
    };
    Some(value)
}

pub static SUGGEST_NEW_CONVERSATION: OpenAiTool = OpenAiTool {
    name: "suggest_new_conversation",
    description: "建议用户从当前 message 处分支出一个新对话。\
                  适用场景:当前对话上下文已经很长且即将切换主题,或当前任务结束、\
                  下一个任务与之无关时。UI 会弹出确认框,用户接受才真正分支。\
                  **不要滥用** — 只在上下文切换收益明显时调。",
    parameters: new_conv_parameters,
    from_args: new_conv_from_args,
    result_to_json: new_conv_result_to_json,
};

// ---------------------------------------------------------------------------
// suggest_prompt
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct PromptArgs {
    /// 实际发给 agent 的 prompt 文本。
    prompt: String,
    /// 可选:UI 上展示的短标签(若 prompt 太长用作 chip 显示)。
    #[serde(default)]
    label: String,
}

fn prompt_parameters() -> Value {
    json!({
        "type": "object",
        "properties": {
            "prompt": {
                "type": "string",
                "description": "建议给用户的下一条 prompt(用户点击后实际发给 agent)。"
            },
            "label": {
                "type": "string",
                "description": "可选: chip 上显示的短标签(prompt 较长时建议提供)。"
            }
        },
        "required": ["prompt"],
        "additionalProperties": false
    })
}

fn prompt_from_args(args: &str) -> Result<api::message::tool_call::Tool> {
    use api::message::tool_call::suggest_prompt::{DisplayMode, PromptChip};
    let parsed: PromptArgs = serde_json::from_str(args)?;
    let chip = PromptChip {
        prompt: parsed.prompt,
        label: parsed.label,
    };
    Ok(api::message::tool_call::Tool::SuggestPrompt(
        api::message::tool_call::SuggestPrompt {
            display_mode: Some(DisplayMode::PromptChip(chip)),
            is_trigger_irrelevant: false,
        },
    ))
}

fn prompt_result_to_json(result: &api::message::tool_call_result::Result) -> Option<Value> {
    use api::message::tool_call_result::Result as R;
    use api::suggest_prompt_result::Result as SR;
    let r = match result {
        R::SuggestPrompt(r) => r,
        _ => return None,
    };
    let value = match &r.result {
        Some(SR::Accepted(_)) => json!({ "status": "accepted" }),
        Some(SR::Rejected(_)) => json!({ "status": "rejected" }),
        None => json!({ "status": "cancelled" }),
    };
    Some(value)
}

pub static SUGGEST_PROMPT: OpenAiTool = OpenAiTool {
    name: "suggest_prompt",
    description: "在回答末尾给用户提议下一条 prompt(以 chip 形式展示)。\
                  适用场景:任务自然延伸出明显的 follow-up(测试通过后建议跑 lint;读完代码建议补单测等)。\
                  避免给重复或显而易见的建议。",
    parameters: prompt_parameters,
    from_args: prompt_from_args,
    result_to_json: prompt_result_to_json,
};

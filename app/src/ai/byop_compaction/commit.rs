//! 把刚刚完成的 SummarizeConversation 流的产出写回 conversation.compaction_state —
//! 对齐 opencode `compaction.ts processCompaction` 末尾的状态变更 + bus.publish(Compacted)。
//!
//! 本模块独立于 controller,作为可单元测试的 helper(虽然真实调用站点在 controller.rs)。

use warp_multi_agent_api as api;

use crate::ai::agent::conversation::AIConversation;

use super::algorithm::{prune_decisions, MessageRef};
use super::config::CompactionConfig;
use super::message_view::{build_tool_name_lookup, project};
use super::state::CompletedCompaction;

/// 从 conversation 的 root task 倒序找最后一条 `Message::AgentOutput` —
/// 它就是模型刚 emit 的摘要文本。
///
/// `user_msg_id` 选最后一条 AgentOutput 之前最近一条真实 UserQuery 的 id;
/// 没有时合成一个独立 uuid(只用作 marker key,build_chat_request 的 hidden
/// 投影不会命中真实 message)。
pub fn commit_summarization(conversation: &mut AIConversation, overflow: bool) -> bool {
    // 用 conversation 已有的 linearized messages accessor — 跨所有 task 已按时间序合并
    let mut all_msgs: Vec<&api::Message> = conversation.all_linearized_messages();
    all_msgs.sort_by_key(|m| {
        m.timestamp
            .as_ref()
            .map(|ts| (ts.seconds, ts.nanos))
            .unwrap_or((0, 0))
    });

    let last_agent_output: Option<(String, String)> = all_msgs.iter().rev().find_map(|m| {
        let inner = m.message.as_ref()?;
        match inner {
            api::message::Message::AgentOutput(a) => Some((m.id.clone(), a.text.clone())),
            _ => None,
        }
    });

    let Some((assistant_id, summary_text)) = last_agent_output else {
        log::warn!("[byop-compaction] commit: no AgentOutput found — nothing to commit");
        return false;
    };

    let assistant_id_str: &str = &assistant_id;
    let assistant_pos = all_msgs
        .iter()
        .position(|m| m.id.as_str() == assistant_id_str);
    let user_msg_id: String = assistant_pos
        .and_then(|pos| {
            all_msgs[..pos]
                .iter()
                .rev()
                .find_map(|m| match m.message.as_ref() {
                    Some(api::message::Message::UserQuery(_)) => Some(m.id.clone()),
                    _ => None,
                })
        })
        .unwrap_or_else(|| format!("compaction-trigger-{}", uuid::Uuid::new_v4()));

    let auto = overflow;
    let summary_len = summary_text.len();
    let completed = CompletedCompaction {
        user_msg_id: user_msg_id.clone(),
        assistant_msg_id: assistant_id.clone(),
        tail_start_id: None,
        summary_text: Some(summary_text),
        auto,
        overflow,
    };
    log::info!(
        "[byop-compaction] commit: assistant_msg={} user_msg={} summary_len={} auto={} overflow={}",
        assistant_id,
        user_msg_id,
        summary_len,
        auto,
        overflow,
    );
    conversation.compaction_state.push_completed(completed);
    true
}

/// 在每次 LLM 请求前自动跑 prune — 1:1 对齐 opencode `compaction.ts:297-341`。
///
/// 计算决策(哪些 ToolCallResult 的 output 应被替换为占位)然后写入
/// `conversation.compaction_state.markers.tool_output_compacted_at`。
/// 实际替换发生在 `chat_stream::build_chat_request` 投影时(读 marker)。
///
/// `cfg.prune == false` 时 no-op。
pub fn prune_now(conversation: &mut AIConversation, cfg: &CompactionConfig) -> usize {
    if !cfg.prune {
        return 0;
    }
    let all_msgs: Vec<&api::Message> = conversation.all_linearized_messages();
    if all_msgs.is_empty() {
        return 0;
    }
    let tool_names = build_tool_name_lookup(all_msgs.iter().copied());
    let state_snapshot = conversation.compaction_state.clone();
    let views = project(&all_msgs, &state_snapshot, &tool_names);
    // 用 trait 引用避免泛型推导歧义
    let views_ref: &[_] = &views;
    let decisions = prune_decisions::<super::message_view::WarpMessageView<'_>>(views_ref);
    if decisions.is_empty() {
        return 0;
    }
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let count = decisions.len();
    for (msg_id, _call_id) in decisions {
        // msg_id 是 ToolCallResult 的 message id;mark_tool_compacted 会在 marker 上写时间戳
        conversation
            .compaction_state
            .mark_tool_compacted(msg_id, now_ms);
    }
    log::info!("[byop-compaction] pruned {count} tool output(s)");
    count
}

// Reference traits for type inference
#[allow(unused_imports)]
use super::algorithm::Role as _Role;
#[allow(unused_imports)]
use super::algorithm::ToolOutputRef as _ToolOutputRef;
// Mention MessageRef so that the import isn't dropped
#[allow(dead_code)]
fn _ensure_message_ref_imported<M: MessageRef>(_m: &M) {}

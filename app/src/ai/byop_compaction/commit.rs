//! 把刚刚完成的 SummarizeConversation 流的产出写回 conversation.compaction_state —
//! 对齐 opencode `compaction.ts processCompaction` 末尾的状态变更 + bus.publish(Compacted)。
//!
//! 本模块独立于 controller,作为可单元测试的 helper(虽然真实调用站点在 controller.rs)。

use warp_multi_agent_api as api;

use crate::ai::agent::conversation::AIConversation;

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
    let assistant_pos = all_msgs.iter().position(|m| m.id.as_str() == assistant_id_str);
    let user_msg_id: String = assistant_pos
        .and_then(|pos| {
            all_msgs[..pos].iter().rev().find_map(|m| match m.message.as_ref() {
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

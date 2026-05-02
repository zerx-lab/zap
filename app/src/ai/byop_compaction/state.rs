//! 压缩 sidecar 状态 — 挂在 `AIConversation` 上,与 warp `api::Message` 协议解耦。
//!
//! 因为 warp 的 `api::Message` 来自外部 protobuf 依赖 (`warp_multi_agent_api`),
//! 无法新增字段标记 `is_summary` / `compacted` 等;本 sidecar 用 message_id 索引
//! 把这些"压缩元数据"挂在 conversation 这一侧。
//!
//! 序列化版本号 [`CompactionState::VERSION`] 在 schema 演进时手动 bump,
//! 反序列化失败的旧 conversation 退化为 `Default`(等价"从未被压缩")。

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

/// 触发压缩的来源。`Auto` 仅由 token-overflow 自动触发,`Manual` 是 /compact /compact-and。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CompactionTrigger {
    Manual,
    Auto,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MessageMarker {
    /// 这条 assistant message 是一份摘要,内容用于在请求拼装时替换前面的历史。
    #[serde(default)]
    pub is_summary: bool,
    /// 这条 user message 是一次 compaction 触发占位(opencode `parts.some(p => p.type === "compaction")`)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compaction_trigger: Option<CompactionTrigger>,
    /// 这条 ToolCallResult 的 output 已被 prune,投影时替换为占位符。Unix epoch ms。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_output_compacted_at: Option<u64>,
    /// 自动续跑时合成的 user "Continue..." synthetic message 标记
    /// (对齐 opencode `metadata.compaction_continue`)。
    #[serde(default)]
    pub synthetic_continue: bool,
}

/// 一个已完成的压缩区间(对齐 opencode `completedCompactions()` 返回项)。
///
/// `user_msg_id` 是触发摘要的 user message(带 compaction_trigger marker),
/// `assistant_msg_id` 是合成的摘要 AgentOutput message。两者在 [`CompactionState::hidden_message_ids`]
/// 中视为已被覆盖,投影时跳过 — 但摘要文本本身会被取出代填到 head 区。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletedCompaction {
    pub user_msg_id: String,
    pub assistant_msg_id: String,
    /// tail 起点 message id,用于 split 验证 / debug。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tail_start_id: Option<String>,
    /// 摘要内容(从 assistant message 直接取也可,但缓存到 state 方便 build_prompt 拿 previous_summary)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary_text: Option<String>,
    pub auto: bool,
    pub overflow: bool,
}

/// 与 `AIConversation` 一同持久化的 sidecar 表。
///
/// 默认值 = 空表 = 未压缩状态,完全无侵入。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionState {
    /// schema 版本,演进时 bump。
    #[serde(default = "CompactionState::current_version")]
    pub version: u32,
    #[serde(default)]
    markers: HashMap<String, MessageMarker>,
    #[serde(default)]
    completed: Vec<CompletedCompaction>,
}

impl Default for CompactionState {
    fn default() -> Self {
        Self {
            version: Self::VERSION,
            markers: HashMap::new(),
            completed: Vec::new(),
        }
    }
}

impl CompactionState {
    pub const VERSION: u32 = 1;
    fn current_version() -> u32 { Self::VERSION }

    pub fn marker(&self, msg_id: &str) -> Option<&MessageMarker> {
        self.markers.get(msg_id)
    }

    /// 写一个 marker(merge 到已有 marker 上,而不是覆盖整个 marker)。
    pub fn upsert_marker(&mut self, msg_id: impl Into<String>, f: impl FnOnce(&mut MessageMarker)) {
        let entry = self.markers.entry(msg_id.into()).or_default();
        f(entry);
    }

    /// 标记一条 ToolCallResult 的 output 已 prune。
    pub fn mark_tool_compacted(&mut self, msg_id: impl Into<String>, now_ms: u64) {
        self.upsert_marker(msg_id, |m| m.tool_output_compacted_at = Some(now_ms));
    }

    /// 推一次完成的压缩。
    pub fn push_completed(&mut self, c: CompletedCompaction) {
        // 同步把 user 与 assistant 各自打上 marker(便于投影时单独识别)。
        self.upsert_marker(c.user_msg_id.clone(), |m| {
            m.compaction_trigger = Some(if c.auto { CompactionTrigger::Auto } else { CompactionTrigger::Manual });
        });
        self.upsert_marker(c.assistant_msg_id.clone(), |m| m.is_summary = true);
        self.completed.push(c);
    }

    /// 标记一条 synthetic "Continue..." user message(auto+overflow 路径合成)。
    pub fn mark_synthetic_continue(&mut self, msg_id: impl Into<String>) {
        self.upsert_marker(msg_id, |m| m.synthetic_continue = true);
    }

    /// 取最后一次完成的压缩(用于 [`super::prompt::build_prompt`] 的增量摘要锚点)。
    pub fn previous_summary(&self) -> Option<&str> {
        self.completed.last().and_then(|c| c.summary_text.as_deref())
    }

    pub fn completed(&self) -> &[CompletedCompaction] {
        &self.completed
    }

    /// 所有应在拼请求时跳过的 message id(对齐 opencode `hidden`):
    /// 已完成压缩的每个区间的 user_msg_id + assistant_msg_id。
    ///
    /// 注:这只是"原本要从历史里隐去的 message id 集",**不**包含摘要本身 —
    /// 摘要文本由 `project_for_request` 在 head 第一个 hidden 位置插入合成消息覆盖。
    pub fn hidden_message_ids(&self) -> HashSet<String> {
        self.completed
            .iter()
            .flat_map(|c| [c.user_msg_id.clone(), c.assistant_msg_id.clone()])
            .collect()
    }

    /// 调试 / 测试入口:看一条 marker 是否存在。
    #[cfg(test)]
    pub(crate) fn marker_count(&self) -> usize {
        self.markers.len()
    }
}

#[cfg(test)]
mod state_tests {
    use super::*;

    fn cc(uid: &str, aid: &str, auto: bool) -> CompletedCompaction {
        CompletedCompaction {
            user_msg_id: uid.to_string(),
            assistant_msg_id: aid.to_string(),
            tail_start_id: None,
            summary_text: Some(format!("summary-{aid}")),
            auto,
            overflow: false,
        }
    }

    #[test]
    fn push_completed_marks_both_messages() {
        let mut s = CompactionState::default();
        s.push_completed(cc("u1", "a1", true));
        assert!(s.marker("u1").unwrap().compaction_trigger == Some(CompactionTrigger::Auto));
        assert!(s.marker("a1").unwrap().is_summary);
    }

    #[test]
    fn previous_summary_returns_last() {
        let mut s = CompactionState::default();
        s.push_completed(cc("u1", "a1", false));
        s.push_completed(cc("u2", "a2", false));
        assert_eq!(s.previous_summary(), Some("summary-a2"));
    }

    #[test]
    fn hidden_message_ids_covers_all_completed() {
        let mut s = CompactionState::default();
        s.push_completed(cc("u1", "a1", false));
        s.push_completed(cc("u2", "a2", false));
        let h = s.hidden_message_ids();
        assert!(h.contains("u1"));
        assert!(h.contains("a1"));
        assert!(h.contains("u2"));
        assert!(h.contains("a2"));
        assert_eq!(h.len(), 4);
    }

    #[test]
    fn upsert_marker_merges() {
        let mut s = CompactionState::default();
        s.upsert_marker("m1", |m| m.is_summary = true);
        s.upsert_marker("m1", |m| m.synthetic_continue = true);
        let m = s.marker("m1").unwrap();
        assert!(m.is_summary);
        assert!(m.synthetic_continue);
        assert_eq!(s.marker_count(), 1);
    }

    #[test]
    fn default_serializable_roundtrip() {
        let s = CompactionState::default();
        let j = serde_json::to_string(&s).unwrap();
        let back: CompactionState = serde_json::from_str(&j).unwrap();
        assert_eq!(back.version, CompactionState::VERSION);
        assert!(back.completed.is_empty());
    }
}

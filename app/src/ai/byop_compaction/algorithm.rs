//! 压缩核心算法 — 1:1 移植 opencode `compaction.ts:141-341`(turns / select / splitTurn / prune)。
//!
//! 与 warp 的具体消息类型解耦:对外通过 [`MessageRef`] trait 抽象,
//! 真实实现见 `super::message_view`。
use std::hash::Hash;

use super::CompactionConfig;
use super::consts::{PRUNE_MINIMUM, PRUNE_PROTECT, PRUNE_PROTECTED_TOOLS};
use super::overflow::{ModelLimit, usable};

/// 消息的角色 — 用于 turn 检测与 select。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    User,
    Assistant,
    Tool,
}

/// 单条 tool 输出的元信息(prune 决策需要)。
#[derive(Debug, Clone)]
pub struct ToolOutputRef<CallId> {
    pub call_id: CallId,
    pub tool_name: String,
    /// 估算 token 数(对齐 opencode `Token.estimate(part.state.output)`)。
    pub output_size: usize,
    pub completed: bool,
    /// 已被 prune 标记 `compacted`,继续遍历时遇到要 break。
    pub already_compacted: bool,
}

/// 抽象的消息引用 — algorithm 只与本 trait 交互,与 warp 类型解耦。
pub trait MessageRef {
    type Id: Clone + Eq + Hash;
    type CallId: Clone + Eq + Hash;

    fn id(&self) -> Self::Id;
    fn role(&self) -> Role;

    /// user message 是否承载了一次 compaction 触发标记(opencode `parts.some(p => p.type === "compaction")`)。
    fn is_compaction_marker(&self) -> bool;

    /// assistant message 是否是摘要本身(opencode `info.summary === true`)。
    fn is_summary(&self) -> bool;

    /// 单条消息的 token 估算 — 实现可用 `serde_json` + `super::token::estimate`。
    fn estimate_size(&self) -> usize;

    /// 这条消息内的所有 tool outputs(prune 用)。assistant message 才会有。
    fn tool_outputs(&self) -> Vec<ToolOutputRef<Self::CallId>>;
}

/// `compaction.ts:76-80` 类型对应。
#[derive(Debug, Clone)]
pub struct Turn<Id> {
    pub start: usize,
    pub end: usize,
    pub id: Id,
}

/// `compaction.ts:82-85`。
#[derive(Debug, Clone)]
pub struct Tail<Id> {
    pub start: usize,
    pub id: Id,
}

/// `select` 返回值:`head` 是要送给摘要 LLM 的范围,`tail_start_id` 是保留段起点。
#[derive(Debug, Clone)]
pub struct SelectResult<Id> {
    pub head_end: usize,
    pub tail_start_id: Option<Id>,
}

/// `compaction.ts:141-157`。
pub fn turns<M: MessageRef>(messages: &[M]) -> Vec<Turn<M::Id>> {
    let mut result: Vec<Turn<M::Id>> = Vec::new();
    let n = messages.len();
    for (i, msg) in messages.iter().enumerate() {
        if msg.role() != Role::User {
            continue;
        }
        if msg.is_compaction_marker() {
            continue;
        }
        result.push(Turn { start: i, end: n, id: msg.id() });
    }
    let len = result.len();
    if len > 1 {
        for i in 0..len - 1 {
            result[i].end = result[i + 1].start;
        }
    }
    result
}

/// `compaction.ts:159-182` splitTurn — 在 turn 内部找第一个能塞进 budget 的切点。
fn split_turn<M, EstFn>(
    messages: &[M],
    turn: &Turn<M::Id>,
    budget: usize,
    estimate: &EstFn,
) -> Option<Tail<M::Id>>
where
    M: MessageRef,
    EstFn: Fn(&[M]) -> usize,
{
    if budget == 0 {
        return None;
    }
    if turn.end.saturating_sub(turn.start) <= 1 {
        return None;
    }
    let mut start = turn.start + 1;
    while start < turn.end {
        let size = estimate(&messages[start..turn.end]);
        if size > budget {
            start += 1;
            continue;
        }
        return Some(Tail { start, id: messages[start].id() });
    }
    None
}

/// `compaction.ts:244-293` select — 切出 head/tail。
///
/// `estimate_slice` 对应 opencode `estimate({ messages: slice, model })`。
/// 调用方传入因为它要决定如何把 message 列表序列化(JSON)再用 `Token.estimate`。
pub fn select<M, EstFn>(
    messages: &[M],
    cfg: &CompactionConfig,
    model: ModelLimit,
    estimate_slice: EstFn,
) -> SelectResult<M::Id>
where
    M: MessageRef,
    EstFn: Fn(&[M]) -> usize,
{
    let limit = cfg.tail_turns;
    if limit == 0 {
        return SelectResult { head_end: messages.len(), tail_start_id: None };
    }
    let usable_tokens = usable(cfg, model);
    let budget = cfg.preserve_recent_budget(usable_tokens);
    let all = turns(messages);
    if all.is_empty() {
        return SelectResult { head_end: messages.len(), tail_start_id: None };
    }
    let recent_start = all.len().saturating_sub(limit);
    let recent: Vec<&Turn<M::Id>> = all[recent_start..].iter().collect();
    let sizes: Vec<usize> = recent
        .iter()
        .map(|t| estimate_slice(&messages[t.start..t.end]))
        .collect();

    let mut total: usize = 0;
    let mut keep: Option<Tail<M::Id>> = None;
    for i in (0..recent.len()).rev() {
        let turn = recent[i];
        let size = sizes[i];
        if total + size <= budget {
            total += size;
            keep = Some(Tail { start: turn.start, id: turn.id.clone() });
            continue;
        }
        let remaining = budget.saturating_sub(total);
        let split = split_turn(messages, turn, remaining, &estimate_slice);
        if split.is_some() {
            keep = split;
        }
        // 注意 opencode 的实现:首次 size 超 budget 就 break,无论 splitTurn 是否找到都不再尝试更早 turn。
        break;
    }

    match keep {
        None => SelectResult { head_end: messages.len(), tail_start_id: None },
        Some(t) if t.start == 0 => SelectResult { head_end: messages.len(), tail_start_id: None },
        Some(t) => SelectResult { head_end: t.start, tail_start_id: Some(t.id) },
    }
}

/// `compaction.ts:297-341` prune 决策 — 返回应被标记 `compacted` 的 (message_id, tool_call_id) 对。
///
/// 调用方据此写入 `CompactionState.markers`(实际 protobuf message 不动)。
pub fn prune_decisions<M: MessageRef>(messages: &[M]) -> Vec<(M::Id, M::CallId)> {
    let mut total: usize = 0;
    let mut pruned: usize = 0;
    let mut to_prune: Vec<(M::Id, M::CallId)> = Vec::new();
    let mut user_turns_seen: usize = 0;

    'outer: for msg in messages.iter().rev() {
        if msg.role() == Role::User {
            user_turns_seen += 1;
        }
        // 至少保留最近 2 个 user turn 不动(opencode `if (turns < 2) continue`)。
        if user_turns_seen < 2 {
            continue;
        }
        // 已是摘要边界 — 不再往前看。
        if msg.role() == Role::Assistant && msg.is_summary() {
            break 'outer;
        }
        let outputs = msg.tool_outputs();
        for tp in outputs.into_iter().rev() {
            if !tp.completed {
                continue;
            }
            if PRUNE_PROTECTED_TOOLS.contains(&tp.tool_name.as_str()) {
                continue;
            }
            if tp.already_compacted {
                break 'outer;
            }
            let estimate = tp.output_size;
            total += estimate;
            if total <= PRUNE_PROTECT {
                continue;
            }
            pruned += estimate;
            to_prune.push((msg.id(), tp.call_id));
        }
    }

    if pruned > PRUNE_MINIMUM {
        to_prune
    } else {
        Vec::new()
    }
}

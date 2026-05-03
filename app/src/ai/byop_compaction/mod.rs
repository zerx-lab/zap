//! BYOP 本地会话压缩 — 1:1 复刻 opencode `packages/opencode/src/session/{compaction,overflow,summary}.ts`。
//!
//! 入口 API:
//! - [`overflow::is_overflow`] — 自动触发判断(基于 LLM response usage)
//! - [`algorithm::select`] — 切分 head(送摘要 LLM) + tail(原样保留)
//! - [`algorithm::prune`] — 只清旧 tool output(不删消息)
//! - [`prompt::build_prompt`] — 拼摘要请求文本
//!
//! 与 warp 服务端 protobuf `SummarizeConversation` 解耦,只在 BYOP 路径生效。
pub mod algorithm;
pub mod commit;
pub mod config;
pub mod message_view;
pub mod overflow;
pub mod prompt;
pub mod state;
pub mod token;

pub use config::CompactionConfig;
pub use overflow::{is_overflow, usable};

/// 字节级对齐 opencode `compaction.ts` 顶部常数(行 33-39, overflow.ts:6, util/token.ts:1)。
pub mod consts {
    pub const PRUNE_MINIMUM: usize = 20_000;
    pub const PRUNE_PROTECT: usize = 40_000;
    pub const TOOL_OUTPUT_MAX_CHARS: usize = 2_000;
    pub const DEFAULT_TAIL_TURNS: usize = 2;
    pub const MIN_PRESERVE_RECENT_TOKENS: usize = 2_000;
    pub const MAX_PRESERVE_RECENT_TOKENS: usize = 8_000;
    pub const COMPACTION_BUFFER: usize = 20_000;
    pub const CHARS_PER_TOKEN: usize = 4;
    pub const PRUNE_PROTECTED_TOOLS: &[&str] = &["skill"];
}

#[cfg(test)]
mod tests;

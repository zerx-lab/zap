//! 压缩配置 — 对齐 opencode `Config.compaction`:
//!
//! ```ts
//! compaction: {
//!   auto?: boolean,                  // default: true
//!   prune?: boolean,                 // default: true
//!   tail_turns?: NonNegativeInt,     // default: 2
//!   preserve_recent_tokens?: NonNegativeInt,
//!   reserved?: NonNegativeInt,
//! }
//! ```
//!
//! warp 这边把它放在 settings/ai.rs 的 BYOPCompactionSettings,反序列化后转成本结构。
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionConfig {
    /// 自动 overflow 触发开关。默认 true。
    pub auto: bool,
    /// tool output prune 开关。默认 true。
    pub prune: bool,
    /// 保留最近几个 user turn 作 tail。默认 2。
    pub tail_turns: usize,
    /// 强制覆盖 `preserve_recent_budget`(token)。None 则按 opencode 公式算。
    pub preserve_recent_tokens: Option<usize>,
    /// 强制覆盖 `usable()` 中的 reserved buffer(token)。None 则取 min(20_000, max_output)。
    pub reserved: Option<usize>,
    /// 摘要专用 model 引用(可选)。设了用它,没设回退到 conversation 当前 model。
    pub compaction_model: Option<CompactionModelRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompactionModelRef {
    pub provider_id: String,
    pub model_id: String,
}

impl Default for CompactionConfig {
    fn default() -> Self {
        Self {
            auto: true,
            prune: true,
            tail_turns: super::consts::DEFAULT_TAIL_TURNS,
            preserve_recent_tokens: None,
            reserved: None,
            compaction_model: None,
        }
    }
}

impl CompactionConfig {
    /// 计算实际的 preserve_recent_budget — 对齐 opencode `compaction.ts:134-139`:
    /// `cfg.preserve_recent_tokens ?? min(MAX, max(MIN, floor(usable * 0.25)))`
    pub fn preserve_recent_budget(&self, usable_tokens: usize) -> usize {
        use super::consts::{MAX_PRESERVE_RECENT_TOKENS, MIN_PRESERVE_RECENT_TOKENS};
        self.preserve_recent_tokens.unwrap_or_else(|| {
            MAX_PRESERVE_RECENT_TOKENS.min(MIN_PRESERVE_RECENT_TOKENS.max(usable_tokens / 4))
        })
    }
}

//! Overflow 判断 — 1:1 移植 opencode `packages/opencode/src/session/overflow.ts`。
//!
//! ```ts
//! const COMPACTION_BUFFER = 20_000
//!
//! export function usable(input: { cfg, model }) {
//!   const context = input.model.limit.context
//!   if (context === 0) return 0
//!   const reserved = input.cfg.compaction?.reserved
//!     ?? Math.min(COMPACTION_BUFFER, ProviderTransform.maxOutputTokens(input.model))
//!   return input.model.limit.input
//!     ? Math.max(0, input.model.limit.input - reserved)
//!     : Math.max(0, context - ProviderTransform.maxOutputTokens(input.model))
//! }
//!
//! export function isOverflow(input: { cfg, tokens, model }) {
//!   if (input.cfg.compaction?.auto === false) return false
//!   if (input.model.limit.context === 0) return false
//!   const count = input.tokens.total
//!     || input.tokens.input + input.tokens.output + input.tokens.cache.read + input.tokens.cache.write
//!   return count >= usable(input)
//! }
//! ```
use super::consts::COMPACTION_BUFFER;
use super::CompactionConfig;

/// 模型 token 限制 — 来源:models.dev metadata 或 BYOP provider 配置。
#[derive(Debug, Clone, Copy)]
pub struct ModelLimit {
    /// 整体 context window
    pub context: usize,
    /// 单独的 input 上限(不少 provider 区分 input/output)。0 表示未知,回退到 context - max_output。
    pub input: usize,
    /// 单次 response 最大 output token
    pub max_output: usize,
}

impl ModelLimit {
    /// 拿不到 metadata 时的保守回退(对齐当下 mainstream Anthropic/OpenAI 主力模型)。
    pub const FALLBACK: ModelLimit = ModelLimit {
        context: 200_000,
        input: 180_000,
        max_output: 8_000,
    };
}

/// 当前对话累计 token 用量 — 字段对齐 opencode `MessageV2.Assistant.tokens`。
#[derive(Debug, Clone, Copy, Default)]
pub struct TokenCounts {
    /// LLM 直接给的总数(优先用)
    pub total: usize,
    pub input: usize,
    pub output: usize,
    pub cache_read: usize,
    pub cache_write: usize,
}

impl TokenCounts {
    /// 对齐 opencode:`tokens.total || input+output+cache.read+cache.write`
    pub fn count(&self) -> usize {
        if self.total > 0 {
            self.total
        } else {
            self.input + self.output + self.cache_read + self.cache_write
        }
    }
}

/// 可用 token 数 — `cfg.reserved ?? min(COMPACTION_BUFFER, max_output)` 作为缓冲。
pub fn usable(cfg: &CompactionConfig, model: ModelLimit) -> usize {
    if model.context == 0 {
        return 0;
    }
    let reserved = cfg
        .reserved
        .unwrap_or_else(|| COMPACTION_BUFFER.min(model.max_output));
    if model.input > 0 {
        model.input.saturating_sub(reserved)
    } else {
        model.context.saturating_sub(model.max_output)
    }
}

/// `count >= usable(...)` 即 overflow。`cfg.auto == false` 时永远 false。
pub fn is_overflow(cfg: &CompactionConfig, tokens: TokenCounts, model: ModelLimit) -> bool {
    if !cfg.auto {
        return false;
    }
    if model.context == 0 {
        return false;
    }
    tokens.count() >= usable(cfg, model)
}

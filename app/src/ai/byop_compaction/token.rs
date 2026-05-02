//! Token 估算 — 对齐 opencode `packages/opencode/src/util/token.ts`。
//!
//! ```ts
//! const CHARS_PER_TOKEN = 4
//! export function estimate(input: string) {
//!   return Math.max(0, Math.round((input || "").length / CHARS_PER_TOKEN))
//! }
//! ```
//!
//! 用 `chars().count()` 而不是 `len()`,避免 UTF-8 多字节字符把估算扭曲到天上。
//! opencode 在 JS 里 `.length` 对 BMP 内字符是 1,与 chars().count() 在多数情况下一致;
//! 对超出 BMP 的 emoji,JS 是 2 (UTF-16 surrogate pair),Rust chars().count() 是 1 —
//! 这点小偏差对 head/tail 切分不构成实际影响。
use super::consts::CHARS_PER_TOKEN;

/// `Math.round(len / 4)` 等价。空串返回 0。
pub fn estimate(input: &str) -> usize {
    let n = input.chars().count();
    // Math.round 是 banker's rounding 之前的"四舍五入到偶数"在 JS 里表现为标准四舍五入,
    // 这里用 (n + 2) / 4 等价于 round(n / 4) 对正整数。
    (n + CHARS_PER_TOKEN / 2) / CHARS_PER_TOKEN
}

/// JSON 序列化后估算 — 对齐 opencode `compaction.ts:241`:
/// `Token.estimate(JSON.stringify(msgs))`
pub fn estimate_json<T: serde::Serialize>(value: &T) -> usize {
    serde_json::to_string(value).map(|s| estimate(&s)).unwrap_or(0)
}

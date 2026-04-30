//! BYOP (Bring Your Own Provider) 的 `LLMId` 前缀编解码。
//!
//! 自定义 Agent 提供商的模型在 `LLMId` 字符串里用前缀 `byop:` 区分,
//! 以便 controller 在请求出口判断该走 warp 后端还是用户自己的 OpenAI 兼容端点。
//!
//! 编码格式: `byop:<provider_id>:<model_id>`
//! - `provider_id` 是 `AgentProvider.id`(UUID)
//! - `model_id` 是 `AgentProviderModel.id`(发给上游 API 的 `model` 字段值)
//!
//! 示例: `byop:6f3b...:deepseek-chat`
//!
//! `provider_id` 是 UUID 不含冒号,`model_id` 可能含冒号(部分上游存在 `vendor:model` 风格的命名),
//! 因此 split 时只在第一个冒号处拆。

use ai::LLMId;

pub const BYOP_PREFIX: &str = "byop:";

/// 把 `(provider_id, model_id)` 编码成单一 `LLMId`。
pub fn encode(provider_id: &str, model_id: &str) -> LLMId {
    LLMId::from(format!("{BYOP_PREFIX}{provider_id}:{model_id}"))
}

/// 若 `LLMId` 是 BYOP 编码,返回 `(provider_id, model_id)`,否则返回 `None`。
pub fn decode(id: &LLMId) -> Option<(String, String)> {
    let s = id.as_str().strip_prefix(BYOP_PREFIX)?;
    let (pid, mid) = s.split_once(':')?;
    if pid.is_empty() || mid.is_empty() {
        return None;
    }
    Some((pid.to_owned(), mid.to_owned()))
}

/// 这个 `LLMId` 是不是 BYOP 编码(供调用方在不需要拆字段时快速判断)。
pub fn is_byop(id: &LLMId) -> bool {
    id.as_str().starts_with(BYOP_PREFIX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let id = encode("uuid-123", "deepseek-chat");
        assert_eq!(id.as_str(), "byop:uuid-123:deepseek-chat");
        assert_eq!(
            decode(&id),
            Some(("uuid-123".to_owned(), "deepseek-chat".to_owned()))
        );
    }

    #[test]
    fn model_id_with_colon_is_preserved() {
        // 例如 OpenRouter 的 "anthropic/claude-3-haiku" 不含冒号,
        // 但部分网关可能用 "vendor:model:variant"。我们只在第一个冒号处 split,
        // 余下部分整体作为 model_id。
        let id = encode("uuid-1", "vendor:model:v2");
        assert_eq!(
            decode(&id),
            Some(("uuid-1".to_owned(), "vendor:model:v2".to_owned()))
        );
    }

    #[test]
    fn non_byop_returns_none() {
        let id = LLMId::from("gpt-5.2");
        assert_eq!(decode(&id), None);
        assert!(!is_byop(&id));
    }

    #[test]
    fn missing_parts_returns_none() {
        assert_eq!(decode(&LLMId::from("byop:")), None);
        assert_eq!(decode(&LLMId::from("byop:uuid")), None); // 没冒号
        assert_eq!(decode(&LLMId::from("byop::model")), None); // 空 provider_id
        assert_eq!(decode(&LLMId::from("byop:uuid:")), None); // 空 model_id
    }
}

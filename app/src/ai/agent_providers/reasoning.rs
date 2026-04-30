//! 模型 reasoning(思考链)能力的启发式判定。
//!
//! 背景:genai 0.6 各 adapter 内部**不**对模型做 capability gate ——
//! 只要 `ChatOptions::reasoning_effort` 非空就照样注入 thinking 参数。
//! 这对**不支持 reasoning 的模型**(claude-3-5-haiku / gpt-4o / gemini-1.5-pro)
//! 会让上游 API 直接 400,所以 client 端必须自己判定。
//!
//! 判定策略沿用 opencode `provider/transform.ts::variants()` 的"硬编码 + 子串匹配":
//! BYOP 用户填的 model id 是任意字符串,无法靠 registry 元数据,只能匹配命名约定。
//!
//! 参考:
//! - genai 0.6 anthropic adapter 的 SUPPORT_EFFORT_MODELS / SUPPORT_ADAPTTIVE_THINK_MODELS
//! - opencode v5 的 anthropicAdaptiveEfforts / OPENAI_EFFORTS 名单
//! - 各 provider 官方文档的 thinking-mode model 列表

use crate::settings::AgentProviderApiType;

/// 判定指定 (api_type, model_name) 组合是否支持 reasoning(思考链)。
///
/// 仅当返回 `true` 时才向 genai 注入 `reasoning_effort`,否则按原样发送
/// 普通 chat 请求,避免向旧模型(如 claude-3-5-haiku / gpt-4o)注入 thinking
/// 参数被上游拒绝。
///
/// 命名约定按各家 model id 风格(全转 lowercase 后子串匹配):
/// - **Anthropic**:`claude-opus-4` / `claude-sonnet-4` / `claude-haiku-4` /
///   `claude-3-7-sonnet`(extended thinking 起点)及更新版本
/// - **OpenAI / OpenAIResp**:`o1` / `o3` / `o4` 系列、`gpt-5`、`codex`
/// - **Gemini**:`gemini-2.5*` / `gemini-3*`(2.5 起 thinking,3.x 全系)
/// - **DeepSeek**:`deepseek-reasoner` / `deepseek-r1` / `deepseek-v4-flash` /
///   `deepseek-thinking`(genai DeepSeek adapter **未接 reasoning_effort 字段**,
///   client 把 reasoning_effort 注入也会被丢弃 — 此处保守返回 `false`)
/// - **Ollama**:走 OpenAI 兼容路径,后端模型 id 不可控,**保守返回 `false`**
///   (用户若确实在跑 thinking 模型,可在 Settings 显式调档,后续再放宽)
pub fn model_supports_reasoning(api_type: AgentProviderApiType, model_id: &str) -> bool {
    let id = model_id.to_ascii_lowercase();

    // genai 后缀推断会自动 strip,但 client 自己判定时也要忽略尾巴(否则
    // `claude-sonnet-4-5-low` 命中不了)。匹配前再剥一层。
    let id = strip_effort_suffix(&id);

    match api_type {
        AgentProviderApiType::Anthropic => is_anthropic_reasoning_model(id),
        AgentProviderApiType::OpenAi | AgentProviderApiType::OpenAiResp => {
            is_openai_reasoning_model(id)
        }
        AgentProviderApiType::Gemini => is_gemini_reasoning_model(id),
        // DeepSeek adapter 不消费 ChatOptions.reasoning_effort(走 reasoning_content
        // 字段读响应,不主动写请求);Ollama 后端模型 id 任意,无法静态判定。
        AgentProviderApiType::DeepSeek | AgentProviderApiType::Ollama => false,
    }
}

fn strip_effort_suffix(id: &str) -> &str {
    if let Some((prefix, last)) = id.rsplit_once('-') {
        if matches!(
            last,
            "none" | "minimal" | "low" | "medium" | "high" | "xhigh" | "max" | "zero"
        ) {
            return prefix;
        }
    }
    id
}

fn is_anthropic_reasoning_model(id: &str) -> bool {
    // claude-3-7-sonnet 是 extended thinking 的起点(2025-02 发布)。
    if id.contains("claude-3-7-sonnet") {
        return true;
    }
    // claude-opus-4* / claude-sonnet-4* / claude-haiku-4* 全系支持。
    // 同时兼容 `4.5` / `4-5` / `4_5` 三种点号风格。
    let four_series = [
        "claude-opus-4",
        "claude-sonnet-4",
        "claude-haiku-4",
    ];
    if four_series.iter().any(|prefix| id.contains(prefix)) {
        return true;
    }
    false
}

fn is_openai_reasoning_model(id: &str) -> bool {
    // o-series reasoning 模型(o1 / o1-mini / o1-pro / o3 / o3-mini / o4 / o4-mini)。
    // 注意 `o1-mini` 在 opencode azure case 被排除,但 OpenAI 官方接受 reasoning_effort,
    // 这里按上游 OpenAI 行为保留。
    let o_series_prefixes = ["o1", "o3", "o4"];
    for prefix in o_series_prefixes {
        if id == prefix
            || id.starts_with(&format!("{prefix}-"))
            || id.starts_with(&format!("{prefix}_"))
        {
            return true;
        }
    }
    // GPT-5 系列(全系 reasoning)+ codex 变体(gpt-5-codex / codex-* / o*-codex 等)。
    if id.contains("gpt-5") || id.contains("codex") {
        return true;
    }
    false
}

fn is_gemini_reasoning_model(id: &str) -> bool {
    // gemini-2.5-* 起 thinking 模式(flash-thinking-exp / pro / pro-thinking)。
    // gemini-3.* 全系(opencode 在 levels 上区分 3 / 3.1)。
    if id.contains("gemini-2.5") || id.contains("gemini-3") {
        return true;
    }
    // 历史 thinking exp 通道(2.0 flash-thinking-exp 也算)。
    if id.contains("thinking") {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anthropic_supported() {
        let t = AgentProviderApiType::Anthropic;
        assert!(model_supports_reasoning(t, "claude-opus-4-5"));
        assert!(model_supports_reasoning(t, "claude-sonnet-4-6"));
        assert!(model_supports_reasoning(t, "claude-opus-4-7"));
        assert!(model_supports_reasoning(t, "claude-3-7-sonnet-20250219"));
        // 后缀不影响判定
        assert!(model_supports_reasoning(t, "claude-sonnet-4-5-high"));
        assert!(model_supports_reasoning(t, "claude-opus-4-7-max"));
    }

    #[test]
    fn anthropic_unsupported() {
        let t = AgentProviderApiType::Anthropic;
        assert!(!model_supports_reasoning(t, "claude-3-5-haiku-20241022"));
        assert!(!model_supports_reasoning(t, "claude-3-5-sonnet-20241022"));
        assert!(!model_supports_reasoning(t, "claude-3-opus-20240229"));
        assert!(!model_supports_reasoning(t, "claude-2.1"));
    }

    #[test]
    fn openai_supported() {
        let t = AgentProviderApiType::OpenAi;
        assert!(model_supports_reasoning(t, "o1"));
        assert!(model_supports_reasoning(t, "o1-mini"));
        assert!(model_supports_reasoning(t, "o3-mini"));
        assert!(model_supports_reasoning(t, "o4-mini"));
        assert!(model_supports_reasoning(t, "gpt-5"));
        assert!(model_supports_reasoning(t, "gpt-5-codex"));
        assert!(model_supports_reasoning(t, "gpt-5-codex-high"));
    }

    #[test]
    fn openai_unsupported() {
        let t = AgentProviderApiType::OpenAi;
        assert!(!model_supports_reasoning(t, "gpt-4o"));
        assert!(!model_supports_reasoning(t, "gpt-4-turbo"));
        assert!(!model_supports_reasoning(t, "gpt-3.5-turbo"));
    }

    #[test]
    fn gemini_supported() {
        let t = AgentProviderApiType::Gemini;
        assert!(model_supports_reasoning(t, "gemini-2.5-pro"));
        assert!(model_supports_reasoning(t, "gemini-2.5-flash"));
        assert!(model_supports_reasoning(t, "gemini-3-pro"));
        assert!(model_supports_reasoning(t, "gemini-2.0-flash-thinking-exp"));
    }

    #[test]
    fn gemini_unsupported() {
        let t = AgentProviderApiType::Gemini;
        assert!(!model_supports_reasoning(t, "gemini-1.5-pro"));
        assert!(!model_supports_reasoning(t, "gemini-1.5-flash"));
        assert!(!model_supports_reasoning(t, "gemini-2.0-flash"));
    }

    #[test]
    fn deepseek_and_ollama_always_false() {
        assert!(!model_supports_reasoning(
            AgentProviderApiType::DeepSeek,
            "deepseek-reasoner"
        ));
        assert!(!model_supports_reasoning(
            AgentProviderApiType::Ollama,
            "qwq-32b"
        ));
    }
}

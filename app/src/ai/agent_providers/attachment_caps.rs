//! BYOP 模式下按 `api_type` × `model_id` 推断该模型支持哪些 multimodal 附件类型。
//!
//! genai 0.6 的 `ContentPart::Binary` 在线协议层全自动适配(参见
//! `chat_stream.rs` 注释表格):
//! - OpenAI: image→`image_url{data:URL}`,pdf/file→`type:"file" file_data:data:URL`,audio→`input_audio`
//! - Anthropic: image→`image base64`,其它→`document base64`(实际仅 PDF 有效)
//! - Gemini: 全部走 `inline_data`
//!
//! 但**线协议支持** ≠ **模型支持**。这里只放"模型实际能消费"的判断,避免给 GPT-3.5
//! 或 Claude Sonnet 1.0 这种纯文本模型送图片导致上游报错。
//!
//! 判定走 model_id 子串匹配,与 `prompt_renderer::resolve_template` 风格对齐。
//! 子串规则故意宽松(包含子串就算命中),目标是"覆盖未来同家族 minor 升级"
//! 而不是"精确版本枚举",失误概率与维护成本权衡向后者倾斜。

use super::models_dev;
use crate::settings::{AgentProviderApiType, AgentProviderModel};

/// 一个模型对附件类型的支持能力表。
#[derive(Debug, Clone, Copy, Default)]
pub struct AttachmentCaps {
    /// 是否支持图片(image/* MIME)。
    pub images: bool,
    /// 是否支持 PDF(application/pdf MIME)。
    pub pdf: bool,
    /// 是否支持音频(audio/* MIME)。
    pub audio: bool,
}

impl AttachmentCaps {
    /// 任何 multimodal 能力都没有 → 上游必须降级到纯文本路径。
    pub fn is_text_only(&self) -> bool {
        !self.images && !self.pdf && !self.audio
    }

    /// 给定 mime,问该模型能否吃下这条 binary 附件。
    pub fn supports_mime(&self, mime: &str) -> bool {
        let lower = mime.trim().to_ascii_lowercase();
        if lower.starts_with("image/") {
            return self.images;
        }
        if lower == "application/pdf" {
            return self.pdf;
        }
        if lower.starts_with("audio/") {
            return self.audio;
        }
        false
    }
}

/// 优先查 models.dev catalog,catalog miss 时按 (api_type, model_id 子串) 兜底。
///
/// catalog 是真实模型能力的权威来源(用户在 settings 里点了 "Sync from models.dev"
/// 或 24h 自动刷新会拉到);兜底规则保证离线 / 还没拉到时主流模型也能用。
pub fn caps_for(api_type: AgentProviderApiType, model_id: &str) -> AttachmentCaps {
    if let Some(c) = models_dev::lookup_caps("", model_id) {
        return AttachmentCaps {
            images: c.vision,
            pdf: c.pdf,
            audio: c.audio,
        };
    }
    caps_for_by_substring(api_type, model_id)
}

/// 解析单个模型的最终 capability,**带用户三态覆盖**。三层优先级:
/// 1. 用户在 settings 显式 `Some(_)` → 直接用,绕过推断
/// 2. `None` → models.dev catalog 推断
/// 3. catalog miss → substring fallback
///
/// `provider_id` 用于 catalog 的 provider 精确匹配(应对 OpenRouter 这种聚合
/// provider 的特殊路径);catalog miss 时降级走 fallback 不需要 provider_id。
pub fn resolve_for_model(
    provider_id: &str,
    api_type: AgentProviderApiType,
    model: &AgentProviderModel,
) -> AttachmentCaps {
    let inferred = if let Some(c) = models_dev::lookup_caps(provider_id, &model.id) {
        AttachmentCaps {
            images: c.vision,
            pdf: c.pdf,
            audio: c.audio,
        }
    } else {
        caps_for_by_substring(api_type, &model.id)
    };
    AttachmentCaps {
        images: model.image.unwrap_or(inferred.images),
        pdf: model.pdf.unwrap_or(inferred.pdf),
        audio: model.audio.unwrap_or(inferred.audio),
    }
}

/// 给 UI 用的"推断结果"快照(忽略用户覆盖,只看 catalog/fallback)。
/// 用来在 chip tooltip 里展示"Auto: catalog says supported"语义。
pub fn inferred_for_model(
    provider_id: &str,
    api_type: AgentProviderApiType,
    model_id: &str,
) -> AttachmentCaps {
    if let Some(c) = models_dev::lookup_caps(provider_id, model_id) {
        AttachmentCaps {
            images: c.vision,
            pdf: c.pdf,
            audio: c.audio,
        }
    } else {
        caps_for_by_substring(api_type, model_id)
    }
}

/// 按 (api_type, model_id 子串) 兜底查表。
///
/// 默认对所有非已知模型保守返回"全 false",好处是不会错误地往不支持的模型
/// 塞 binary 导致 400;代价是新模型上线后需要手动加入(可接受,反正每个新模型
/// 还有 reasoning_effort / context_window 等其它配置要更新)。
fn caps_for_by_substring(api_type: AgentProviderApiType, model_id: &str) -> AttachmentCaps {
    let lower = model_id.to_ascii_lowercase();
    match api_type {
        AgentProviderApiType::OpenAi | AgentProviderApiType::OpenAiResp => {
            // GPT-4o / 4.1 / 5 系列:image + pdf。3.5 系列纯文本。
            if lower.contains("gpt-4o")
                || lower.contains("gpt-4.1")
                || lower.contains("gpt-5")
                || lower.contains("o1")
                || lower.contains("o3")
                || lower.contains("o4")
            {
                AttachmentCaps {
                    images: true,
                    pdf: true,
                    audio: false,
                }
            } else if lower.contains("gpt-4o-audio") || lower.contains("gpt-realtime") {
                AttachmentCaps {
                    images: true,
                    pdf: true,
                    audio: true,
                }
            } else {
                AttachmentCaps::default()
            }
        }
        AgentProviderApiType::Anthropic => {
            // Claude 3 / 3.5 / 4 / 4.5 / 4.7 全系都支持 vision + document(PDF)。
            if lower.contains("claude-3")
                || lower.contains("claude-4")
                || lower.contains("claude-opus")
                || lower.contains("claude-sonnet")
                || lower.contains("claude-haiku")
            {
                AttachmentCaps {
                    images: true,
                    pdf: true,
                    audio: false,
                }
            } else {
                AttachmentCaps::default()
            }
        }
        AgentProviderApiType::Gemini => {
            // Gemini 1.5+ / 2 / 2.5 全系 multimodal,inline_data 支持 image/pdf/audio/video。
            if lower.contains("gemini-1.5")
                || lower.contains("gemini-2")
                || lower.contains("gemini-pro-vision")
            {
                AttachmentCaps {
                    images: true,
                    pdf: true,
                    audio: true,
                }
            } else {
                AttachmentCaps::default()
            }
        }
        AgentProviderApiType::Ollama => {
            // Ollama 多数模型纯文本。Vision 模型(LLaVA / bakllava / llama3.2-vision /
            // qwen2-vl / minicpm-v / moondream)按 model_id 子串匹配开 image 能力。
            // PDF/audio 在 Ollama 协议下基本无解,保守返 false。
            let vision = lower.contains("llava")
                || lower.contains("bakllava")
                || lower.contains("vision")
                || lower.contains("-vl")
                || lower.contains("minicpm-v")
                || lower.contains("moondream");
            AttachmentCaps {
                images: vision,
                pdf: false,
                audio: false,
            }
        }
        AgentProviderApiType::DeepSeek => {
            // DeepSeek 现有公开模型(v3/r1/coder/chat)目前都是纯文本。
            // 未来 deepseek-vl 系列上线时再开。
            if lower.contains("vl") {
                AttachmentCaps {
                    images: true,
                    pdf: false,
                    audio: false,
                }
            } else {
                AttachmentCaps::default()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openai_4o_supports_image_and_pdf() {
        // 走 fallback 规则:测试环境 models.dev catalog 未加载,lookup_caps 返回 None。
        let caps = caps_for_by_substring(AgentProviderApiType::OpenAi, "gpt-4o-2024-08-06");
        assert!(caps.images);
        assert!(caps.pdf);
        assert!(!caps.audio);
    }

    #[test]
    fn openai_3_5_text_only() {
        let caps = caps_for_by_substring(AgentProviderApiType::OpenAi, "gpt-3.5-turbo");
        assert!(caps.is_text_only());
    }

    #[test]
    fn claude_sonnet_supports_image_and_pdf() {
        let caps = caps_for_by_substring(AgentProviderApiType::Anthropic, "claude-sonnet-4-5");
        assert!(caps.images);
        assert!(caps.pdf);
    }

    #[test]
    fn gemini_2_5_full_multimodal() {
        let caps = caps_for_by_substring(AgentProviderApiType::Gemini, "gemini-2.5-pro");
        assert!(caps.images);
        assert!(caps.pdf);
        assert!(caps.audio);
    }

    #[test]
    fn ollama_default_text_only() {
        let caps = caps_for_by_substring(AgentProviderApiType::Ollama, "qwen2.5:7b");
        assert!(caps.is_text_only());
    }

    #[test]
    fn ollama_vision_models_get_images() {
        let caps = caps_for_by_substring(AgentProviderApiType::Ollama, "llava:13b");
        assert!(caps.images);
        assert!(!caps.pdf);
    }

    #[test]
    fn deepseek_chat_text_only() {
        let caps = caps_for_by_substring(AgentProviderApiType::DeepSeek, "deepseek-chat");
        assert!(caps.is_text_only());
    }

    #[test]
    fn supports_mime_routing() {
        let full = AttachmentCaps {
            images: true,
            pdf: true,
            audio: true,
        };
        assert!(full.supports_mime("image/png"));
        assert!(full.supports_mime("application/pdf"));
        assert!(full.supports_mime("audio/mp3"));
        assert!(!full.supports_mime("application/zip"));
        assert!(!full.supports_mime("text/plain"));
    }
}

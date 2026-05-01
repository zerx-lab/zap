//! 自定义 Agent 提供商支持。
//!
//! 这个模块负责:
//! - 把每个 Provider 的 `api_key` 安全地存到 OS keychain (secure_storage),
//!   而 Provider 元数据(name/base_url/model 列表) 走普通 settings.toml。
//! - 通过 `OpenAiCompatibleClient` 调用 `${base_url}/models`
//!   抓取上游可用模型列表(供 UI "Fetch models" 按钮使用)。
//!
//! 第二阶段会基于这套配置实现 `AiProvider` trait,
//! 把 Agent 的 multi-agent 调用分流到本地 Provider。

pub mod active_ai;
pub mod chat_stream;
pub mod llm_id;
pub mod models_dev;
pub mod oneshot;
pub mod openai_compatible;
pub mod prompt_renderer;
pub mod reasoning;
pub mod secrets;
pub mod tools;
pub mod user_context;

// 当前外部使用点:
// - `fetch_openai_compatible_models`: ai_page.rs 中的 FetchAgentProviderModels handler
// - `AgentProviderSecrets`: ai_page.rs 中的多个 handler 与 lib.rs 注册点
// 其余符号(`OpenAiCompatibleError`/`OpenAiCompatibleModel`/`AgentProviderSecretsEvent`)
// 仍可通过 `crate::ai::agent_providers::openai_compatible::*` 等完整路径访问,
// 这里不再 re-export 以避免 `unused_imports` 告警。
pub use openai_compatible::fetch_openai_compatible_models;
pub use secrets::AgentProviderSecrets;

// ---------------------------------------------------------------------------
// LLMInfo 合成:把 settings 中配置的 agent_providers 转成 picker 可用的形态
// ---------------------------------------------------------------------------

use std::collections::HashMap;

use settings::Setting;
use warpui::{AppContext, SingletonEntity};

use crate::ai::llms::{
    AvailableLLMs, DisableReason, LLMInfo, LLMProvider, LLMUsageMetadata, ModelsByFeature,
};
use crate::settings::{AISettings, AgentProvider};

/// 合成给定 provider 的所有合法 (provider, model) 对的 LLMInfo 列表。
///
/// "合法"=  provider 有非空 base_url + 至少 1 个 model + 在 secrets 中能查到 api_key。
/// 不合法的 provider 会整体被忽略(picker 中干脆不展示其下的模型),
/// 这样用户能直观地看到"哪些 provider 没填全 → 没出现"。
fn build_byop_llm_infos(app: &AppContext) -> Vec<LLMInfo> {
    let providers = AISettings::as_ref(app).agent_providers.value().clone();
    let secrets = AgentProviderSecrets::as_ref(app);
    let mut out = Vec::new();

    for provider in providers {
        if provider.base_url.trim().is_empty() {
            continue;
        }
        if provider.models.is_empty() {
            continue;
        }
        let has_key = secrets
            .get(&provider.id)
            .map(|k| !k.is_empty())
            .unwrap_or(false);
        if !has_key {
            continue;
        }

        let provider_label = if provider.name.trim().is_empty() {
            provider.id.clone()
        } else {
            provider.name.clone()
        };

        for model in &provider.models {
            if model.id.trim().is_empty() {
                continue;
            }
            let display_name = if model.name.trim().is_empty() {
                model.id.clone()
            } else {
                model.name.clone()
            };
            out.push(LLMInfo {
                display_name: format!("{provider_label} / {display_name}"),
                base_model_name: format!("{provider_label} / {display_name}"),
                id: llm_id::encode(&provider.id, &model.id),
                reasoning_level: None,
                usage_metadata: LLMUsageMetadata {
                    request_multiplier: 1,
                    credit_multiplier: None,
                },
                description: None,
                disable_reason: None,
                vision_supported: false,
                spec: None,
                provider: LLMProvider::Unknown,
                host_configs: HashMap::new(),
                discount_percentage: None,
            });
        }
    }

    out
}

/// 占位条目:当用户没配任何合法 provider 时,picker 至少要有 1 个条目
/// (`AvailableLLMs::new` 拒绝空列表)。该条目用 `DisableReason::Unavailable` 灰显,
/// 选不动,提示用户去设置中配。
fn placeholder_llm_info() -> LLMInfo {
    LLMInfo {
        display_name: "未配置自定义提供商 — 请到 设置 → AI 添加".to_owned(),
        base_model_name: "未配置".to_owned(),
        id: ai::LLMId::from("byop-placeholder"),
        reasoning_level: None,
        usage_metadata: LLMUsageMetadata {
            request_multiplier: 1,
            credit_multiplier: None,
        },
        description: None,
        disable_reason: Some(DisableReason::Unavailable),
        vision_supported: false,
        spec: None,
        provider: LLMProvider::Unknown,
        host_configs: HashMap::new(),
        discount_percentage: None,
    }
}

/// 构造一个完全由 BYOP 模型填充的 `ModelsByFeature`。
/// 4 个 feature(agent_mode / coding / cli_agent / computer_use)使用同一份模型集合 —
/// 自定义 provider 不区分 capability,所有模型都能用作任意 feature。
pub fn build_byop_models_by_feature(app: &AppContext) -> ModelsByFeature {
    let mut choices = build_byop_llm_infos(app);
    if choices.is_empty() {
        choices.push(placeholder_llm_info());
    }

    let default_id = choices[0].id.clone();
    let make = || {
        AvailableLLMs::new(default_id.clone(), choices.clone(), None)
            .expect("choices is non-empty by construction")
    };

    ModelsByFeature {
        agent_mode: make(),
        coding: make(),
        cli_agent: Some(make()),
        computer_use: Some(make()),
    }
}

/// 给定一个 BYOP `LLMId`,从 `AISettings` 与 secrets 里查出 `(provider, api_key, model_id)`。
/// 任一信息缺失返回 `None`(controller 调用方应映射为 `InvalidApiKey` 错误)。
pub fn lookup_byop(
    app: &AppContext,
    id: &ai::LLMId,
) -> Option<(AgentProvider, String, String)> {
    let (provider_id, model_id) = llm_id::decode(id)?;
    let providers = AISettings::as_ref(app).agent_providers.value().clone();
    let provider = providers.into_iter().find(|p| p.id == provider_id)?;
    let api_key = AgentProviderSecrets::as_ref(app)
        .get(&provider_id)
        .map(str::to_owned)?;
    Some((provider, api_key, model_id))
}

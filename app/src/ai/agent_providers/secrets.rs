//! `AgentProviderSecrets`:把每个自定义 Provider 的 API key 保存到 OS 密钥库。
//!
//! 数据形态: `HashMap<provider_id, api_key>`,通过 `serde_json` 序列化后写入
//! `secure_storage` 的 `AgentProviderSecrets` 键。
//!
//! 设计参考 `crates/ai/src/api_keys.rs::ApiKeyManager`。

use std::collections::HashMap;

use warpui::{Entity, ModelContext, SingletonEntity};
use warpui_extras::secure_storage::{self, AppContextExt};

const SECURE_STORAGE_KEY: &str = "AgentProviderSecrets";

/// 当任意 Provider 的 API key 发生变化时发出。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentProviderSecretsEvent {
    KeysUpdated,
}

/// 单例:管理用户自定义 Provider 的 API key。
pub struct AgentProviderSecrets {
    keys: HashMap<String, String>,
}

impl AgentProviderSecrets {
    /// 启动时从 secure storage 读取所有 key。
    pub fn new(ctx: &mut ModelContext<Self>) -> Self {
        Self {
            keys: Self::load_from_storage(ctx),
        }
    }

    /// 读取指定 Provider 的 API key,若未配置则返回 `None`。
    pub fn get(&self, provider_id: &str) -> Option<&str> {
        self.keys.get(provider_id).map(String::as_str)
    }

    /// 设置/更新某个 Provider 的 API key。
    /// 传入空字符串等价于删除。
    pub fn set(&mut self, provider_id: &str, api_key: String, ctx: &mut ModelContext<Self>) {
        if api_key.is_empty() {
            self.keys.remove(provider_id);
        } else {
            self.keys.insert(provider_id.to_owned(), api_key);
        }
        ctx.emit(AgentProviderSecretsEvent::KeysUpdated);
        self.persist(ctx);
    }

    /// 删除某个 Provider(连带其 secret)。
    pub fn remove(&mut self, provider_id: &str, ctx: &mut ModelContext<Self>) {
        if self.keys.remove(provider_id).is_some() {
            ctx.emit(AgentProviderSecretsEvent::KeysUpdated);
            self.persist(ctx);
        }
    }

    fn load_from_storage(ctx: &mut ModelContext<Self>) -> HashMap<String, String> {
        let raw = match ctx.secure_storage().read_value(SECURE_STORAGE_KEY) {
            Ok(json) => json,
            Err(secure_storage::Error::NotFound) => return HashMap::new(),
            Err(e) => {
                log::error!("Failed to read agent provider secrets: {e:#}");
                return HashMap::new();
            }
        };
        serde_json::from_str(&raw).unwrap_or_else(|e| {
            log::error!("Failed to deserialize agent provider secrets: {e:#}");
            HashMap::new()
        })
    }

    fn persist(&self, ctx: &mut ModelContext<Self>) {
        let json = match serde_json::to_string(&self.keys) {
            Ok(json) => json,
            Err(e) => {
                log::error!("Failed to serialize agent provider secrets: {e:#}");
                return;
            }
        };
        if let Err(e) = ctx.secure_storage().write_value(SECURE_STORAGE_KEY, &json) {
            log::error!("Failed to write agent provider secrets: {e:#}");
        }
    }
}

impl Entity for AgentProviderSecrets {
    type Event = AgentProviderSecretsEvent;
}

impl SingletonEntity for AgentProviderSecrets {}

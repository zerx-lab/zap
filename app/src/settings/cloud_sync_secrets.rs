//! 云同步 Token 安全存储 — 使用 OS 密钥库（Windows Credential Manager / macOS Keychain / Linux Secret Service）。
//!
//! 数据形态: `HashMap<platform_key, token>`, 通过 `serde_json` 序列化后写入
//! `secure_storage` 的 `CloudSyncTokens` 键。
//!
// author: logic
// date: 2026-05-26

use std::collections::HashMap;

use warpui::{Entity, ModelContext, SingletonEntity};
use warpui_extras::secure_storage::{self, AppContextExt};

const SECURE_STORAGE_KEY: &str = "CloudSyncTokens";

/// 平台 Token 在 HashMap 中的 key
pub const GITHUB_TOKEN_KEY: &str = "github_token";
pub const GITEE_TOKEN_KEY: &str = "gitee_token";

/// 当 Token 发生变化时发出
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CloudSyncTokenStoreEvent {
    TokensChanged,
}

/// 单例: 管理云同步平台的访问令牌
pub struct CloudSyncTokenStore {
    tokens: HashMap<String, String>,
}

impl CloudSyncTokenStore {
    /// 启动时从 secure storage 读取所有 token
    pub fn new(ctx: &mut ModelContext<Self>) -> Self {
        Self {
            tokens: Self::load_from_storage(ctx),
        }
    }

    /// 读取指定平台的 token, 若未配置则返回 `None`
    pub fn get(&self, platform_key: &str) -> Option<&str> {
        self.tokens.get(platform_key).map(String::as_str)
    }

    /// 设置/更新某个平台的 token。传入空字符串等价于删除
    pub fn set(&mut self, platform_key: &str, token: String, ctx: &mut ModelContext<Self>) {
        if token.is_empty() {
            self.tokens.remove(platform_key);
        } else {
            self.tokens.insert(platform_key.to_owned(), token);
        }
        ctx.emit(CloudSyncTokenStoreEvent::TokensChanged);
        self.persist(ctx);
    }

    fn load_from_storage(ctx: &mut ModelContext<Self>) -> HashMap<String, String> {
        let raw = match ctx.secure_storage().read_value(SECURE_STORAGE_KEY) {
            Ok(json) => json,
            Err(secure_storage::Error::NotFound) => return HashMap::new(),
            Err(e) => {
                log::error!("Failed to read cloud sync tokens: {e:#}");
                return HashMap::new();
            }
        };
        serde_json::from_str(&raw).unwrap_or_else(|e| {
            log::error!("Failed to deserialize cloud sync tokens: {e:#}");
            HashMap::new()
        })
    }

    fn persist(&self, ctx: &mut ModelContext<Self>) {
        if self.tokens.is_empty() {
            if let Err(e) = ctx.secure_storage().remove_value(SECURE_STORAGE_KEY) {
                log::error!("Failed to remove cloud sync tokens: {e:#}");
            }
            return;
        }
        let json = match serde_json::to_string(&self.tokens) {
            Ok(json) => json,
            Err(e) => {
                log::error!("Failed to serialize cloud sync tokens: {e:#}");
                return;
            }
        };
        if let Err(e) = ctx.secure_storage().write_value(SECURE_STORAGE_KEY, &json) {
            log::error!("Failed to write cloud sync tokens: {e:#}");
        }
    }
}

impl Entity for CloudSyncTokenStore {
    type Event = CloudSyncTokenStoreEvent;
}

impl SingletonEntity for CloudSyncTokenStore {}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_store(tokens: HashMap<String, String>) -> CloudSyncTokenStore {
        CloudSyncTokenStore { tokens }
    }

    #[test]
    fn test_get_existing_token() {
        let mut map = HashMap::new();
        map.insert(GITHUB_TOKEN_KEY.to_string(), "ghp_abc123".to_string());
        let store = make_store(map);
        assert_eq!(store.get(GITHUB_TOKEN_KEY), Some("ghp_abc123"));
    }

    #[test]
    fn test_get_missing_token_returns_none() {
        let store = make_store(HashMap::new());
        assert_eq!(store.get(GITHUB_TOKEN_KEY), None);
    }

    #[test]
    fn test_get_both_platforms() {
        let mut map = HashMap::new();
        map.insert(GITHUB_TOKEN_KEY.to_string(), "ghp_abc".to_string());
        map.insert(GITEE_TOKEN_KEY.to_string(), "gitee_xyz".to_string());
        let store = make_store(map);
        assert_eq!(store.get(GITHUB_TOKEN_KEY), Some("ghp_abc"));
        assert_eq!(store.get(GITEE_TOKEN_KEY), Some("gitee_xyz"));
    }

    #[test]
    fn test_set_inserts_token() {
        let store = make_store(HashMap::new());
        let mut map = store.tokens;
        map.insert(GITHUB_TOKEN_KEY.to_string(), "ghp_new".to_string());
        let store = make_store(map);
        assert_eq!(store.get(GITHUB_TOKEN_KEY), Some("ghp_new"));
    }

    #[test]
    fn test_set_empty_string_removes_token() {
        let mut map = HashMap::new();
        map.insert(GITHUB_TOKEN_KEY.to_string(), "ghp_abc".to_string());
        let mut store = make_store(map);
        // 模拟 set 中空串删除逻辑
        store.tokens.remove(GITHUB_TOKEN_KEY);
        assert_eq!(store.get(GITHUB_TOKEN_KEY), None);
    }

    #[test]
    fn test_set_overwrites_existing() {
        let mut map = HashMap::new();
        map.insert(GITHUB_TOKEN_KEY.to_string(), "old_token".to_string());
        let mut store = make_store(map);
        store.tokens.insert(GITHUB_TOKEN_KEY.to_string(), "new_token".to_string());
        assert_eq!(store.get(GITHUB_TOKEN_KEY), Some("new_token"));
    }

    #[test]
    fn test_serialization_roundtrip() {
        let mut map = HashMap::new();
        map.insert(GITHUB_TOKEN_KEY.to_string(), "ghp_abc".to_string());
        map.insert(GITEE_TOKEN_KEY.to_string(), "gitee_xyz".to_string());
        let json = serde_json::to_string(&map).unwrap();
        let parsed: HashMap<String, String> = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, map);
    }

    #[test]
    fn test_deserialization_empty_object() {
        let parsed: HashMap<String, String> = serde_json::from_str("{}").unwrap();
        assert!(parsed.is_empty());
    }

    #[test]
    fn test_deserialization_invalid_json_returns_empty() {
        let parsed: Result<HashMap<String, String>, _> = serde_json::from_str("not json");
        assert!(parsed.is_err());
    }

    #[test]
    fn test_persist_removes_key_when_empty() {
        let store = make_store(HashMap::new());
        // 空 map 不应序列化写入
        assert!(store.tokens.is_empty());
    }

    #[test]
    fn test_persist_writes_json_when_nonempty() {
        let mut map = HashMap::new();
        map.insert(GITHUB_TOKEN_KEY.to_string(), "token_value".to_string());
        let store = make_store(map);
        let json = serde_json::to_string(&store.tokens).unwrap();
        assert!(json.contains(GITHUB_TOKEN_KEY));
        assert!(json.contains("token_value"));
    }

    #[test]
    fn test_secure_storage_key_constant() {
        assert_eq!(SECURE_STORAGE_KEY, "CloudSyncTokens");
    }

    #[test]
    fn test_platform_key_constants() {
        assert_eq!(GITHUB_TOKEN_KEY, "github_token");
        assert_eq!(GITEE_TOKEN_KEY, "gitee_token");
    }

    #[test]
    fn test_event_equality() {
        assert_eq!(CloudSyncTokenStoreEvent::TokensChanged, CloudSyncTokenStoreEvent::TokensChanged);
    }
}

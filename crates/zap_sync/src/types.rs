//! 云同步通用类型定义
//!
// author: logic
// date: 2026-05-24

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 同步平台
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncPlatform {
    GitHub,
    Gitee,
}

impl SyncPlatform {
    /// 获取平台 API 基础 URL
    pub fn base_url(&self) -> &str {
        match self {
            Self::GitHub => "https://api.github.com",
            Self::Gitee => "https://gitee.com/api/v5",
        }
    }

    /// 获取平台显示名称
    pub fn label(&self) -> &str {
        match self {
            Self::GitHub => "GitHub",
            Self::Gitee => "Gitee",
        }
    }

    /// 获取平台持久化标识
    pub fn to_db_str(&self) -> &str {
        match self {
            Self::GitHub => "github",
            Self::Gitee => "gitee",
        }
    }
}

/// 同步结果
#[derive(Debug, Clone)]
pub enum SyncResult {
    Success { version: i64, platform: SyncPlatform },
    Conflict { local_version: i64, remote_version: i64 },
    AlreadyUpToDate { version: i64 },
}

/// Gist 列表条目（API 返回）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GistEntry {
    pub id: String,
    pub description: Option<String>,
}

/// Gist 详情（API 返回）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GistDetail {
    pub id: String,
    pub files: serde_json::Map<String, serde_json::Value>,
}

/// Gist 中的完整同步数据（顶层结构）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncData {
    pub version: i64,
    pub synced_at: String,
    /// 各 section 的数据，key 为 section 名（如 "ssh"），value 为该 section 的 JSON
    #[serde(flatten)]
    pub sections: serde_json::Map<String, serde_json::Value>,
}

/// 同步引擎错误
#[derive(Debug, Error)]
pub enum SyncEngineError {
    #[error("加密错误: {0}")]
    Crypto(String),
    #[error("Gist 错误: {0}")]
    Gist(String),
    #[error("数据提供者错误: {0}")]
    Provider(String),
    #[error("序列化错误: {0}")]
    Serialization(String),
    #[error("版本存储错误: {0}")]
    VersionStore(String),
}

/// 同步元数据（版本号管理 trait，由调用方实现）
pub trait SyncVersionStore: Send + Sync {
    /// 获取当前同步版本号
    fn get_sync_version(&self) -> Result<i64, SyncEngineError>;
    /// 设置同步版本号
    fn set_sync_version(&self, version: i64) -> Result<(), SyncEngineError>;
    /// 更新同步元数据（时间、平台）
    fn update_sync_meta(&self, time: &str, platform: &str) -> Result<(), SyncEngineError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_platform_base_url() {
        assert_eq!(SyncPlatform::GitHub.base_url(), "https://api.github.com");
        assert_eq!(SyncPlatform::Gitee.base_url(), "https://gitee.com/api/v5");
    }

    #[test]
    fn test_sync_platform_label() {
        assert_eq!(SyncPlatform::GitHub.label(), "GitHub");
        assert_eq!(SyncPlatform::Gitee.label(), "Gitee");
    }

    #[test]
    fn test_sync_platform_to_db_str() {
        assert_eq!(SyncPlatform::GitHub.to_db_str(), "github");
        assert_eq!(SyncPlatform::Gitee.to_db_str(), "gitee");
    }

    #[test]
    fn test_sync_platform_equality() {
        assert_eq!(SyncPlatform::GitHub, SyncPlatform::GitHub);
        assert_ne!(SyncPlatform::GitHub, SyncPlatform::Gitee);
    }

    #[test]
    fn test_sync_data_serialization() {
        let mut sections = serde_json::Map::new();
        sections.insert("ssh".to_string(), serde_json::json!({"nodes": []}));
        let data = SyncData {
            version: 42,
            synced_at: "2026-01-01T00:00:00Z".to_string(),
            sections,
        };
        let json = serde_json::to_string(&data).unwrap();
        let parsed: SyncData = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.version, 42);
        assert_eq!(parsed.synced_at, "2026-01-01T00:00:00Z");
        assert!(parsed.sections.contains_key("ssh"));
    }

    #[test]
    fn test_sync_data_empty_sections() {
        let data = SyncData {
            version: 0,
            synced_at: String::new(),
            sections: serde_json::Map::new(),
        };
        let json = serde_json::to_string(&data).unwrap();
        let parsed: SyncData = serde_json::from_str(&json).unwrap();
        assert!(parsed.sections.is_empty());
    }

    #[test]
    fn test_gist_entry_deserialization() {
        let json = r#"{"id":"abc123","description":"ZAP_CONFIG"}"#;
        let entry: GistEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.id, "abc123");
        assert_eq!(entry.description, Some("ZAP_CONFIG".to_string()));
    }

    #[test]
    fn test_gist_entry_null_description() {
        let json = r#"{"id":"abc123","description":null}"#;
        let entry: GistEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.id, "abc123");
        assert_eq!(entry.description, None);
    }

    #[test]
    fn test_gist_detail_deserialization() {
        let json = r#"{"id":"gist1","files":{}}"#;
        let detail: GistDetail = serde_json::from_str(json).unwrap();
        assert_eq!(detail.id, "gist1");
        assert!(detail.files.is_empty());
    }

    #[test]
    fn test_sync_engine_error_display() {
        let err = SyncEngineError::Crypto("bad key".to_string());
        assert_eq!(format!("{err}"), "加密错误: bad key");

        let err = SyncEngineError::Gist("not found".to_string());
        assert_eq!(format!("{err}"), "Gist 错误: not found");

        let err = SyncEngineError::Provider("db fail".to_string());
        assert_eq!(format!("{err}"), "数据提供者错误: db fail");

        let err = SyncEngineError::Serialization("parse err".to_string());
        assert_eq!(format!("{err}"), "序列化错误: parse err");

        let err = SyncEngineError::VersionStore("io err".to_string());
        assert_eq!(format!("{err}"), "版本存储错误: io err");
    }
}

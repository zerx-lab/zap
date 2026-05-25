//! 云同步通用类型定义
//!
// author: logic
// date: 2026-05-24

use serde::{Deserialize, Serialize};

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
#[derive(Debug)]
pub enum SyncEngineError {
    Crypto(String),
    Gist(String),
    Provider(String),
    Serialization(String),
    VersionStore(String),
}

impl std::fmt::Display for SyncEngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Crypto(e) => write!(f, "加密错误: {e}"),
            Self::Gist(e) => write!(f, "Gist 错误: {e}"),
            Self::Provider(e) => write!(f, "数据提供者错误: {e}"),
            Self::Serialization(e) => write!(f, "序列化错误: {e}"),
            Self::VersionStore(e) => write!(f, "版本存储错误: {e}"),
        }
    }
}

impl std::error::Error for SyncEngineError {}

/// 同步元数据（版本号管理 trait，由调用方实现）
pub trait SyncVersionStore: Send + Sync {
    /// 获取当前同步版本号
    fn get_sync_version(&self) -> Result<i64, SyncEngineError>;
    /// 设置同步版本号
    fn set_sync_version(&self, version: i64) -> Result<(), SyncEngineError>;
    /// 更新同步元数据（时间、平台）
    fn update_sync_meta(&self, time: &str, platform: &str) -> Result<(), SyncEngineError>;
}

//! 同步引擎
//!
// author: logic
// date: 2026-05-24

use crate::gist_client::GistClient;
use crate::types::*;
use chrono::Utc;

/// 数据提供者 trait，各业务模块实现此 trait 接入同步
pub trait SyncDataProvider: Send + Sync {
    /// 数据所属的 section key（如 "ssh"）
    fn section_key(&self) -> &str;

    /// 收集本地数据，返回该 section 的 JSON Value
    fn collect_data(&self, token: &str) -> Result<serde_json::Value, SyncEngineError>;

    /// 将云端数据应用到本地
    fn apply_data(&self, token: &str, data: &serde_json::Value) -> Result<(), SyncEngineError>;
}

/// 同步引擎，负责上传/下载同步数据到 Gist
pub struct SyncEngine {
    client: GistClient,
}

impl SyncEngine {
    /// 创建新的 SyncEngine 实例
    pub fn new() -> Self {
        Self {
            client: GistClient::new(),
        }
    }

    /// 上传数据到指定平台
    pub async fn upload(
        &self,
        platform: SyncPlatform,
        token: &str,
        providers: &[&dyn SyncDataProvider],
        version_store: &dyn SyncVersionStore,
    ) -> Result<SyncResult, SyncEngineError> {
        let local_version = version_store.get_sync_version()?;

        let mut sections = serde_json::Map::new();
        for provider in providers {
            let data = provider.collect_data(token)?;
            sections.insert(provider.section_key().to_string(), data);
        }

        let sync_data = SyncData {
            version: local_version,
            synced_at: Utc::now().to_rfc3339(),
            sections,
        };

        let content = serde_json::to_string_pretty(&sync_data)
            .map_err(|e| SyncEngineError::Serialization(e.to_string()))?;

        if let Some(gist_id) = self
            .client
            .find_gist(platform, token)
            .await
            .map_err(|e| SyncEngineError::Gist(e.to_string()))?
        {
            let remote_content = self
                .client
                .get_gist_content(platform, token, &gist_id)
                .await
                .map_err(|e| SyncEngineError::Gist(e.to_string()))?;
            let remote_data: SyncData = serde_json::from_str(&remote_content)
                .map_err(|e| SyncEngineError::Serialization(e.to_string()))?;

            if remote_data.version > local_version {
                return Ok(SyncResult::Conflict {
                    local_version,
                    remote_version: remote_data.version,
                });
            }

            self.client
                .update_gist(platform, token, &gist_id, &content)
                .await
                .map_err(|e| SyncEngineError::Gist(e.to_string()))?;
        } else {
            self.client
                .create_gist(platform, token, &content)
                .await
                .map_err(|e| SyncEngineError::Gist(e.to_string()))?;
        }

        version_store.update_sync_meta(&Utc::now().to_rfc3339(), platform.label())?;

        Ok(SyncResult::Success {
            version: local_version,
            platform,
        })
    }

    /// 从指定平台下载数据
    pub async fn download(
        &self,
        platform: SyncPlatform,
        token: &str,
        providers: &[&dyn SyncDataProvider],
        version_store: &dyn SyncVersionStore,
    ) -> Result<SyncResult, SyncEngineError> {
        let gist_id = self
            .client
            .find_gist(platform, token)
            .await
            .map_err(|e| SyncEngineError::Gist(e.to_string()))?
            .ok_or_else(|| SyncEngineError::Gist("Gist 未找到".to_string()))?;

        let remote_content = self
            .client
            .get_gist_content(platform, token, &gist_id)
            .await
            .map_err(|e| SyncEngineError::Gist(e.to_string()))?;
        let remote_data: SyncData = serde_json::from_str(&remote_content)
            .map_err(|e| SyncEngineError::Serialization(e.to_string()))?;

        let local_version = version_store.get_sync_version()?;

        if remote_data.version <= local_version {
            return Ok(SyncResult::AlreadyUpToDate {
                version: remote_data.version,
            });
        }

        for provider in providers {
            let key = provider.section_key();
            if let Some(section_data) = remote_data.sections.get(key) {
                provider.apply_data(token, section_data)?;
            }
        }

        version_store.set_sync_version(remote_data.version)?;
        version_store.update_sync_meta(&Utc::now().to_rfc3339(), platform.label())?;

        Ok(SyncResult::Success {
            version: remote_data.version,
            platform,
        })
    }

    /// 获取本地版本号
    pub fn get_local_version(version_store: &dyn SyncVersionStore) -> Result<i64, SyncEngineError> {
        version_store.get_sync_version()
    }
}

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

    /// 强制上传，忽略远程版本冲突。版本号由引擎内部管理，失败时回滚
    pub async fn force_upload(
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

        // 查找已有 Gist
        let gist_id = self
            .client
            .find_gist(platform, token)
            .await
            .map_err(|e| SyncEngineError::Gist(e.to_string()))?;

        // 确定远程版本号
        let remote_version = if let Some(ref gid) = gist_id {
            let remote_content = self
                .client
                .get_gist_content(platform, token, gid)
                .await
                .map_err(|e| SyncEngineError::Gist(e.to_string()))?;
            let remote_data: SyncData = serde_json::from_str(&remote_content)
                .map_err(|e| SyncEngineError::Serialization(e.to_string()))?;
            Some(remote_data.version)
        } else {
            None
        };

        let new_version = std::cmp::max(local_version, remote_version.unwrap_or(0)) + 1;

        let sync_data = SyncData {
            version: new_version,
            synced_at: Utc::now().to_rfc3339(),
            sections,
        };
        let content = serde_json::to_string_pretty(&sync_data)
            .map_err(|e| SyncEngineError::Serialization(e.to_string()))?;

        // 先递增版本号
        version_store.set_sync_version(new_version)?;

        // 上传，失败时回滚版本号
        let upload_result = if let Some(gid) = gist_id {
            self.client
                .update_gist(platform, token, &gid, &content)
                .await
        } else {
            self.client
                .create_gist(platform, token, &content)
                .await
                .map(|_| ())
        };

        if let Err(e) = upload_result {
            if let Err(rollback_err) = version_store.set_sync_version(local_version) {
                log::error!("强制上传失败后回滚版本号失败: {rollback_err}");
            }
            return Err(SyncEngineError::Gist(e.to_string()));
        }

        version_store.update_sync_meta(&Utc::now().to_rfc3339(), platform.label())?;

        Ok(SyncResult::Success {
            version: new_version,
            platform,
        })
    }

    /// 获取本地版本号
    pub fn get_local_version(version_store: &dyn SyncVersionStore) -> Result<i64, SyncEngineError> {
        version_store.get_sync_version()
    }
}

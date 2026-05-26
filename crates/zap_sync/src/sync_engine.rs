//! 同步引擎
//!
// author: logic
// date: 2026-05-24

use crate::gist_client::{GistClient, GistOps};
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
pub struct SyncEngine<C: GistOps> {
    client: C,
}

impl SyncEngine<GistClient> {
    /// 创建新的 SyncEngine 实例（使用真实 GistClient）
    pub fn new() -> Self {
        Self {
            client: GistClient::new(),
        }
    }
}

impl<C: GistOps> SyncEngine<C> {
    /// 使用自定义 GistOps 实现创建引擎
    pub fn with_client(client: C) -> Self {
        Self { client }
    }

    /// 上传数据到指定平台
    pub async fn upload(
        &self,
        platform: SyncPlatform,
        token: &str,
        providers: &[&dyn SyncDataProvider],
        version_store: &dyn SyncVersionStore,
    ) -> Result<SyncResult, SyncEngineError> {
        let local_version = tokio::task::block_in_place(|| version_store.get_sync_version())?;

        let mut sections = serde_json::Map::new();
        for provider in providers {
            let data = tokio::task::block_in_place(|| provider.collect_data(token))?;
            sections.insert(provider.section_key().to_string(), data);
        }

        let sync_data = SyncData {
            version: local_version,
            synced_at: Utc::now().to_rfc3339(),
            sections,
        };

        let content = serde_json::to_string_pretty(&sync_data)
            .map_err(|e| SyncEngineError::Serialization(e.to_string()))?;

        let token_owned = token.to_string();

        if let Some(gist_id) = self
            .client
            .find_gist(platform, token_owned.clone())
            .await
            .map_err(|e| SyncEngineError::Gist(e.to_string()))?
        {
            let remote_content = self
                .client
                .get_gist_content(platform, token_owned.clone(), gist_id.clone())
                .await
                .map_err(|e| SyncEngineError::Gist(e.to_string()))?;
            let remote_data: SyncData = serde_json::from_str(&remote_content)
                .map_err(|e| SyncEngineError::Serialization(e.to_string()))?;

            if remote_data.version >= local_version {
                return Ok(SyncResult::Conflict {
                    local_version,
                    remote_version: remote_data.version,
                });
            }

            self.client
                .update_gist(platform, token_owned, gist_id, content)
                .await
                .map_err(|e| SyncEngineError::Gist(e.to_string()))?;
        } else {
            self.client
                .create_gist(platform, token_owned, content)
                .await
                .map_err(|e| SyncEngineError::Gist(e.to_string()))?;
        }

        tokio::task::block_in_place(|| version_store.update_sync_meta(&Utc::now().to_rfc3339(), platform.to_db_str()))?;
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
        let token_owned = token.to_string();

        let gist_id = self
            .client
            .find_gist(platform, token_owned.clone())
            .await
            .map_err(|e| SyncEngineError::Gist(e.to_string()))?
            .ok_or_else(|| SyncEngineError::Gist("Gist 未找到".to_string()))?;

        let remote_content = self
            .client
            .get_gist_content(platform, token_owned, gist_id)
            .await
            .map_err(|e| SyncEngineError::Gist(e.to_string()))?;
        let remote_data: SyncData = serde_json::from_str(&remote_content)
            .map_err(|e| SyncEngineError::Serialization(e.to_string()))?;

        let local_version = tokio::task::block_in_place(|| version_store.get_sync_version())?;

        if remote_data.version <= local_version {
            return Ok(SyncResult::AlreadyUpToDate {
                version: remote_data.version,
            });
        }

        for provider in providers {
            let key = provider.section_key();
            if let Some(section_data) = remote_data.sections.get(key) {
                tokio::task::block_in_place(|| provider.apply_data(token, section_data))?;
            }
        }

        tokio::task::block_in_place(|| version_store.set_sync_version(remote_data.version))?;
        tokio::task::block_in_place(|| version_store.update_sync_meta(&Utc::now().to_rfc3339(), platform.to_db_str()))?;

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
        let local_version = tokio::task::block_in_place(|| version_store.get_sync_version())?;

        let mut sections = serde_json::Map::new();
        for provider in providers {
            let data = tokio::task::block_in_place(|| provider.collect_data(token))?;
            sections.insert(provider.section_key().to_string(), data);
        }

        let token_owned = token.to_string();

        // 查找已有 Gist
        let gist_id = self
            .client
            .find_gist(platform, token_owned.clone())
            .await
            .map_err(|e| SyncEngineError::Gist(e.to_string()))?;

        // 确定远程版本号
        let remote_version = if let Some(ref gid) = gist_id {
            let remote_content = self
                .client
                .get_gist_content(platform, token_owned.clone(), gid.clone())
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
        tokio::task::block_in_place(|| version_store.set_sync_version(new_version))?;

        // 上传，失败时回滚版本号
        let upload_result = if let Some(gid) = gist_id {
            self.client
                .update_gist(platform, token_owned, gid, content)
                .await
        } else {
            self.client
                .create_gist(platform, token_owned, content)
                .await
                .map(|_| ())
        };

        if let Err(e) = upload_result {
            if let Err(rollback_err) = tokio::task::block_in_place(|| version_store.set_sync_version(local_version)) {
                log::error!("强制上传失败后回滚版本号失败: {rollback_err}");
            }
            return Err(SyncEngineError::Gist(e.to_string()));
        }

        tokio::task::block_in_place(|| version_store.update_sync_meta(&Utc::now().to_rfc3339(), platform.to_db_str()))?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GistClientError;
    use std::sync::Mutex;

    struct MockVersionStore {
        version: Mutex<i64>,
    }

    impl MockVersionStore {
        fn new(v: i64) -> Self {
            Self {
                version: Mutex::new(v),
            }
        }
    }

    impl SyncVersionStore for MockVersionStore {
        fn get_sync_version(&self) -> Result<i64, SyncEngineError> {
            Ok(*self.version.lock().unwrap())
        }
        fn set_sync_version(&self, version: i64) -> Result<(), SyncEngineError> {
            *self.version.lock().unwrap() = version;
            Ok(())
        }
        fn update_sync_meta(&self, _time: &str, _platform: &str) -> Result<(), SyncEngineError> {
            Ok(())
        }
    }

    #[test]
    fn test_get_local_version() {
        let store = MockVersionStore::new(42);
        let result = SyncEngine::<MockGistOps>::get_local_version(&store).unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn test_get_local_version_default() {
        let store = MockVersionStore::new(0);
        let result = SyncEngine::<MockGistOps>::get_local_version(&store).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_mock_version_store_set() {
        let store = MockVersionStore::new(1);
        store.set_sync_version(99).unwrap();
        assert_eq!(store.get_sync_version().unwrap(), 99);
    }

    struct MockGistOps {
        find_result: Mutex<Option<String>>,
        content: String,
        create_called: Mutex<bool>,
        update_called: Mutex<bool>,
    }

    impl MockGistOps {
        fn new(find_result: Option<String>, content: &str) -> Self {
            Self {
                find_result: Mutex::new(find_result),
                content: content.to_string(),
                create_called: Mutex::new(false),
                update_called: Mutex::new(false),
            }
        }
    }

    impl GistOps for MockGistOps {
        async fn validate_token(&self, _platform: SyncPlatform, _token: String) -> Result<String, GistClientError> {
            Ok("testuser".to_string())
        }
        async fn find_gist(&self, _platform: SyncPlatform, _token: String) -> Result<Option<String>, GistClientError> {
            Ok(self.find_result.lock().unwrap().clone())
        }
        async fn create_gist(&self, _platform: SyncPlatform, _token: String, _content: String) -> Result<String, GistClientError> {
            *self.create_called.lock().unwrap() = true;
            Ok("new_gist_id".to_string())
        }
        async fn update_gist(&self, _platform: SyncPlatform, _token: String, _gist_id: String, _content: String) -> Result<(), GistClientError> {
            *self.update_called.lock().unwrap() = true;
            Ok(())
        }
        async fn get_gist_content(&self, _platform: SyncPlatform, _token: String, _gist_id: String) -> Result<String, GistClientError> {
            Ok(self.content.clone())
        }
    }

    struct MockProvider;

    impl SyncDataProvider for MockProvider {
        fn section_key(&self) -> &str { "ssh" }
        fn collect_data(&self, _token: &str) -> Result<serde_json::Value, SyncEngineError> {
            Ok(serde_json::json!({"nodes": []}))
        }
        fn apply_data(&self, _token: &str, _data: &serde_json::Value) -> Result<(), SyncEngineError> {
            Ok(())
        }
    }

    fn make_sync_data_json(version: i64) -> String {
        let data = SyncData {
            version,
            synced_at: "2026-01-01T00:00:00Z".to_string(),
            sections: {
                let mut m = serde_json::Map::new();
                m.insert("ssh".to_string(), serde_json::json!({"nodes": []}));
                m
            },
        };
        serde_json::to_string_pretty(&data).unwrap()
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_upload_creates_new_gist() {
        do_test_upload_creates_new_gist(SyncPlatform::GitHub).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_upload_creates_new_gist_gitee() {
        do_test_upload_creates_new_gist(SyncPlatform::Gitee).await;
    }

    async fn do_test_upload_creates_new_gist(platform: SyncPlatform) {
        let mock = MockGistOps::new(None, "");
        let engine = SyncEngine::with_client(mock);
        let provider = MockProvider;
        let store = MockVersionStore::new(1);
        let result = engine.upload(platform, "token", &[&provider], &store).await.unwrap();
        assert!(matches!(result, SyncResult::Success { version: 1, .. }));
        assert!(*engine.client.create_called.lock().unwrap());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_upload_updates_existing_gist() {
        do_test_upload_updates_existing_gist(SyncPlatform::GitHub).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_upload_updates_existing_gist_gitee() {
        do_test_upload_updates_existing_gist(SyncPlatform::Gitee).await;
    }

    async fn do_test_upload_updates_existing_gist(platform: SyncPlatform) {
        let mock = MockGistOps::new(Some("gist123".to_string()), &make_sync_data_json(0));
        let engine = SyncEngine::with_client(mock);
        let provider = MockProvider;
        let store = MockVersionStore::new(1);
        let result = engine.upload(platform, "token", &[&provider], &store).await.unwrap();
        assert!(matches!(result, SyncResult::Success { version: 1, .. }));
        assert!(*engine.client.update_called.lock().unwrap());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_upload_detects_conflict() {
        do_test_upload_detects_conflict(SyncPlatform::GitHub).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_upload_detects_conflict_gitee() {
        do_test_upload_detects_conflict(SyncPlatform::Gitee).await;
    }

    async fn do_test_upload_detects_conflict(platform: SyncPlatform) {
        let mock = MockGistOps::new(Some("gist123".to_string()), &make_sync_data_json(5));
        let engine = SyncEngine::with_client(mock);
        let provider = MockProvider;
        let store = MockVersionStore::new(1);
        let result = engine.upload(platform, "token", &[&provider], &store).await.unwrap();
        assert!(matches!(result, SyncResult::Conflict { local_version: 1, remote_version: 5 }));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_download_already_up_to_date() {
        do_test_download_already_up_to_date(SyncPlatform::GitHub).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_download_already_up_to_date_gitee() {
        do_test_download_already_up_to_date(SyncPlatform::Gitee).await;
    }

    async fn do_test_download_already_up_to_date(platform: SyncPlatform) {
        let mock = MockGistOps::new(Some("gist123".to_string()), &make_sync_data_json(1));
        let engine = SyncEngine::with_client(mock);
        let provider = MockProvider;
        let store = MockVersionStore::new(5);
        let result = engine.download(platform, "token", &[&provider], &store).await.unwrap();
        assert!(matches!(result, SyncResult::AlreadyUpToDate { version: 1 }));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_download_succeeds() {
        do_test_download_succeeds(SyncPlatform::GitHub).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_download_succeeds_gitee() {
        do_test_download_succeeds(SyncPlatform::Gitee).await;
    }

    async fn do_test_download_succeeds(platform: SyncPlatform) {
        let mock = MockGistOps::new(Some("gist123".to_string()), &make_sync_data_json(10));
        let engine = SyncEngine::with_client(mock);
        let provider = MockProvider;
        let store = MockVersionStore::new(1);
        let result = engine.download(platform, "token", &[&provider], &store).await.unwrap();
        assert!(matches!(result, SyncResult::Success { version: 10, .. }));
        assert_eq!(store.get_sync_version().unwrap(), 10);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_download_gist_not_found() {
        do_test_download_gist_not_found(SyncPlatform::GitHub).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_download_gist_not_found_gitee() {
        do_test_download_gist_not_found(SyncPlatform::Gitee).await;
    }

    async fn do_test_download_gist_not_found(platform: SyncPlatform) {
        let mock = MockGistOps::new(None, "");
        let engine = SyncEngine::with_client(mock);
        let provider = MockProvider;
        let store = MockVersionStore::new(1);
        let result = engine.download(platform, "token", &[&provider], &store).await;
        assert!(result.is_err());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_upload_conflict_on_equal_version() {
        let mock = MockGistOps::new(Some("gist123".to_string()), &make_sync_data_json(3));
        let engine = SyncEngine::with_client(mock);
        let provider = MockProvider;
        let store = MockVersionStore::new(3);
        let result = engine.upload(SyncPlatform::GitHub, "token", &[&provider], &store).await.unwrap();
        assert!(matches!(result, SyncResult::Conflict { local_version: 3, remote_version: 3 }));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_force_upload_new_gist() {
        let mock = MockGistOps::new(None, "");
        let engine = SyncEngine::with_client(mock);
        let provider = MockProvider;
        let store = MockVersionStore::new(1);
        let result = engine.force_upload(SyncPlatform::GitHub, "token", &[&provider], &store).await.unwrap();
        assert!(matches!(result, SyncResult::Success { version: 2, .. }));
        assert_eq!(store.get_sync_version().unwrap(), 2);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_force_upload_existing_gist_max_version() {
        let mock = MockGistOps::new(Some("gist123".to_string()), &make_sync_data_json(5));
        let engine = SyncEngine::with_client(mock);
        let provider = MockProvider;
        let store = MockVersionStore::new(3);
        let result = engine.force_upload(SyncPlatform::GitHub, "token", &[&provider], &store).await.unwrap();
        assert!(matches!(result, SyncResult::Success { version: 6, .. }));
        assert_eq!(store.get_sync_version().unwrap(), 6);
    }
}

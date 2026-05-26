//! Gist API 客户端
//!
// author: logic
// date: 2026-05-24

use crate::types::{GistDetail, GistEntry, SyncPlatform};
use reqwest::Client;
use serde_json::json;
use thiserror::Error;

const GIST_DESCRIPTION: &str = "ZAP_CONFIG";
const GIST_FILENAME: &str = "zap_config.json";

/// Gist API 客户端错误
#[derive(Debug, Error)]
pub enum GistClientError {
    #[error("网络请求失败: {0}")]
    Request(#[from] reqwest::Error),
    #[error("Gist 未找到")]
    NotFound,
    #[error("Token 未配置")]
    NoToken,
    #[error("API 错误: {status} {body}")]
    Api { status: u16, body: String },
}

/// Gist 操作 trait，支持真实客户端和测试 mock
pub trait GistOps: Send + Sync {
    /// 验证 Token 是否有效，返回用户名
    fn validate_token(&self, platform: SyncPlatform, token: String) -> impl std::future::Future<Output = Result<String, GistClientError>> + Send;

    /// 查找 description 为 ZAP_CONFIG 的 Gist
    fn find_gist(&self, platform: SyncPlatform, token: String) -> impl std::future::Future<Output = Result<Option<String>, GistClientError>> + Send;

    /// 创建新 Gist
    fn create_gist(&self, platform: SyncPlatform, token: String, content: String) -> impl std::future::Future<Output = Result<String, GistClientError>> + Send;

    /// 更新已有 Gist
    fn update_gist(&self, platform: SyncPlatform, token: String, gist_id: String, content: String) -> impl std::future::Future<Output = Result<(), GistClientError>> + Send;

    /// 获取 Gist 文件内容
    fn get_gist_content(&self, platform: SyncPlatform, token: String, gist_id: String) -> impl std::future::Future<Output = Result<String, GistClientError>> + Send;
}

/// Gist API 客户端，支持 GitHub 和 Gitee
pub struct GistClient {
    client: Client,
}

impl GistClient {
    /// 创建新的 GistClient 实例
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .user_agent("Zap-Terminal")
                .build()
                .unwrap_or_default(),
        }
    }

    /// 构建认证头，GitHub 用 Bearer，Gitee 用 token 前缀
    fn auth_header(platform: SyncPlatform, token: &str) -> String {
        match platform {
            SyncPlatform::GitHub => format!("Bearer {token}"),
            SyncPlatform::Gitee => format!("token {token}"),
        }
    }

    /// 验证 Token 是否有效，返回用户名
    pub async fn validate_token(
        &self,
        platform: SyncPlatform,
        token: &str,
    ) -> Result<String, GistClientError> {
        if token.is_empty() {
            return Err(GistClientError::NoToken);
        }
        let url = format!("{}/user", platform.base_url());
        let resp = self
            .client
            .get(&url)
            .header("Authorization", Self::auth_header(platform, token))
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(GistClientError::Api {
                status: resp.status().as_u16(),
                body: resp.text().await.unwrap_or_default(),
            });
        }

        let user: serde_json::Value = resp.json().await?;
        let login = user["login"].as_str().unwrap_or("unknown");
        Ok(login.to_string())
    }

    /// 查找 description 为 ZAP_CONFIG 的 Gist，返回其 ID
    pub async fn find_gist(
        &self,
        platform: SyncPlatform,
        token: &str,
    ) -> Result<Option<String>, GistClientError> {
        if token.is_empty() {
            return Err(GistClientError::NoToken);
        }
        let base_url = platform.base_url();
        let mut page = 1;

        loop {
            let url = format!("{base_url}/gists?page={page}&per_page=100");
            let resp = self
                .client
                .get(&url)
                .header("Authorization", Self::auth_header(platform, token))
                .send()
                .await?;

            if !resp.status().is_success() {
                return Err(GistClientError::Api {
                    status: resp.status().as_u16(),
                    body: resp.text().await.unwrap_or_default(),
                });
            }

            let gists: Vec<GistEntry> = resp.json().await?;

            if gists.is_empty() {
                return Ok(None);
            }

            if let Some(found) = gists
                .iter()
                .find(|g| g.description.as_deref() == Some(GIST_DESCRIPTION))
            {
                return Ok(Some(found.id.clone()));
            }

            page += 1;
        }
    }

    /// 创建新 Gist
    pub async fn create_gist(
        &self,
        platform: SyncPlatform,
        token: &str,
        content: &str,
    ) -> Result<String, GistClientError> {
        if token.is_empty() {
            return Err(GistClientError::NoToken);
        }
        let url = format!("{}/gists", platform.base_url());
        let body = json!({
            "description": GIST_DESCRIPTION,
            "public": false,
            "files": {
                GIST_FILENAME: {
                    "content": content
                }
            }
        });
        let resp = self
            .client
            .post(&url)
            .header("Authorization", Self::auth_header(platform, token))
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(GistClientError::Api {
                status: resp.status().as_u16(),
                body: resp.text().await.unwrap_or_default(),
            });
        }

        let detail: GistDetail = resp.json().await?;
        Ok(detail.id)
    }

    /// 更新已有 Gist
    pub async fn update_gist(
        &self,
        platform: SyncPlatform,
        token: &str,
        gist_id: &str,
        content: &str,
    ) -> Result<(), GistClientError> {
        if token.is_empty() {
            return Err(GistClientError::NoToken);
        }
        let url = format!("{}/gists/{gist_id}", platform.base_url());
        let body = json!({
            "description": GIST_DESCRIPTION,
            "files": {
                GIST_FILENAME: {
                    "content": content
                }
            }
        });
        let resp = self
            .client
            .patch(&url)
            .header("Authorization", Self::auth_header(platform, token))
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(GistClientError::Api {
                status: resp.status().as_u16(),
                body: resp.text().await.unwrap_or_default(),
            });
        }

        Ok(())
    }

    /// 获取 Gist 文件内容，自动处理截断
    pub async fn get_gist_content(
        &self,
        platform: SyncPlatform,
        token: &str,
        gist_id: &str,
    ) -> Result<String, GistClientError> {
        if token.is_empty() {
            return Err(GistClientError::NoToken);
        }
        let url = format!("{}/gists/{gist_id}", platform.base_url());
        let resp = self
            .client
            .get(&url)
            .header("Authorization", Self::auth_header(platform, token))
            .send()
            .await?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(GistClientError::NotFound);
        }
        if !resp.status().is_success() {
            return Err(GistClientError::Api {
                status: resp.status().as_u16(),
                body: resp.text().await.unwrap_or_default(),
            });
        }

        let detail: serde_json::Value = resp.json().await?;
        let file_obj = &detail["files"][GIST_FILENAME];

        if file_obj["truncated"].as_bool() == Some(true) {
            let raw_url = file_obj["raw_url"]
                .as_str()
                .ok_or(GistClientError::NotFound)?;
            let raw_resp = self
                .client
                .get(raw_url)
                .header("Authorization", Self::auth_header(platform, token))
                .send()
                .await?;
            if !raw_resp.status().is_success() {
                return Err(GistClientError::Api {
                    status: raw_resp.status().as_u16(),
                    body: raw_resp.text().await.unwrap_or_default(),
                });
            }
            Ok(raw_resp.text().await?)
        } else {
            let content = file_obj["content"]
                .as_str()
                .ok_or(GistClientError::NotFound)?;
            Ok(content.to_string())
        }
    }
}

impl GistOps for GistClient {
    async fn validate_token(&self, platform: SyncPlatform, token: String) -> Result<String, GistClientError> {
        self.validate_token(platform, &token).await
    }

    async fn find_gist(&self, platform: SyncPlatform, token: String) -> Result<Option<String>, GistClientError> {
        self.find_gist(platform, &token).await
    }

    async fn create_gist(&self, platform: SyncPlatform, token: String, content: String) -> Result<String, GistClientError> {
        self.create_gist(platform, &token, &content).await
    }

    async fn update_gist(&self, platform: SyncPlatform, token: String, gist_id: String, content: String) -> Result<(), GistClientError> {
        self.update_gist(platform, &token, &gist_id, &content).await
    }

    async fn get_gist_content(&self, platform: SyncPlatform, token: String, gist_id: String) -> Result<String, GistClientError> {
        self.get_gist_content(platform, &token, &gist_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_header_github() {
        let header = GistClient::auth_header(SyncPlatform::GitHub, "mytoken");
        assert_eq!(header, "Bearer mytoken");
    }

    #[test]
    fn test_auth_header_gitee() {
        let header = GistClient::auth_header(SyncPlatform::Gitee, "mytoken");
        assert_eq!(header, "token mytoken");
    }

    #[test]
    fn test_empty_token_returns_early() {
        assert!(GistClient::auth_header(SyncPlatform::GitHub, "").ends_with(""));
        assert!(GistClient::auth_header(SyncPlatform::Gitee, "").ends_with(""));
    }
}

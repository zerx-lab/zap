//! OpenAI 兼容客户端的最小子集:目前只用来抓 `/models` 列表。
//!
//! 等第二阶段做 multi-agent 调用时,这里会扩展为完整的
//! Chat Completions + 工具调用 stream。

use serde::Deserialize;

use http_client::Client;

/// `/models` 端点返回的单个模型条目。
///
/// 我们只关心 `id`(给 Agent 用作 model 名)。其他字段(`object`/`created`/`owned_by`)
/// 不同提供商差异较大,这里全部忽略。
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct OpenAiCompatibleModel {
    pub id: String,
    /// 由 `owned_by` 推断的拥有者,主要用作 UI 展示,可能为空。
    #[serde(default)]
    pub owned_by: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ModelsResponse {
    data: Vec<OpenAiCompatibleModel>,
}

/// fetch 期间可能出现的错误。
#[derive(Debug, thiserror::Error)]
pub enum OpenAiCompatibleError {
    #[error("base URL 无效: {0}")]
    InvalidBaseUrl(String),

    #[error("HTTP 错误: {0}")]
    Http(#[from] reqwest::Error),

    #[error("HTTP 状态码 {status}: {body}")]
    Status { status: u16, body: String },

    #[error("响应解析失败: {0}")]
    Decode(String),
}

/// 把用户输入的 base_url 规范化成绝对 URL 形式,
/// 容忍尾部 `/`、缺失的 `/v1`、`/openai/v1` 等情况。
pub(crate) fn normalize_base_url(input: &str) -> Result<String, OpenAiCompatibleError> {
    let trimmed = input.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err(OpenAiCompatibleError::InvalidBaseUrl(
            "base URL 不能为空".to_string(),
        ));
    }
    if !(trimmed.starts_with("http://") || trimmed.starts_with("https://")) {
        return Err(OpenAiCompatibleError::InvalidBaseUrl(format!(
            "base URL 必须以 http:// 或 https:// 开头: {trimmed}"
        )));
    }
    Ok(trimmed.to_string())
}

/// 调用 `${base_url}/models`,返回模型 ID 列表(已去重 + 按字母序排序)。
///
/// 鉴权:若 `api_key` 非空则以 `Authorization: Bearer ...` 形式带上。
/// 部分本地服务(如 Ollama)允许不带鉴权,因此 key 为空时不发送 header。
pub async fn fetch_openai_compatible_models(
    client: Client,
    base_url: &str,
    api_key: Option<&str>,
) -> Result<Vec<OpenAiCompatibleModel>, OpenAiCompatibleError> {
    let base = normalize_base_url(base_url)?;
    let url = format!("{base}/models");

    let mut req = client.get(&url);
    if let Some(key) = api_key.filter(|k| !k.trim().is_empty()) {
        req = req.bearer_auth(key);
    }

    let response = req.send().await?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(OpenAiCompatibleError::Status {
            status: status.as_u16(),
            body,
        });
    }

    let parsed: ModelsResponse = response
        .json()
        .await
        .map_err(|e| OpenAiCompatibleError::Decode(e.to_string()))?;

    let mut models = parsed.data;
    models.sort_by(|a, b| a.id.cmp(&b.id));
    models.dedup_by(|a, b| a.id == b.id);
    Ok(models)
}

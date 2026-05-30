//! GitHub Copilot / GitHub Models OAuth 登录支持。

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context as _};
use serde::Deserialize;
use serde_json::Value;

use super::{AgentProviderOAuthCredentials, AgentProviderOAuthKind};

pub const COPILOT_PROVIDER_NAME: &str = "Copilot Auth";
pub const COPILOT_BASE_URL: &str = "https://api.githubcopilot.com/";

// 说明:
// 这里使用公开可观察到的 GitHub Copilot CLI OAuth app client id 来走 device flow。
// 我们没有在仓库里找到 OpenCode 私有实现细节,因此这里按 GitHub 官方 device flow
// 协议接入,client id 视作与 Copilot CLI 生态对齐的实现细节。
const GITHUB_COPILOT_OAUTH_CLIENT_ID: &str = "01ab8ac9400c4e429b23";
const GITHUB_DEVICE_CODE_URL: &str = "https://github.com/login/device/code";
const GITHUB_ACCESS_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
const GITHUB_COPILOT_TOKEN_EXCHANGE_URL: &str = "https://api.github.com/copilot_internal/v2/token";
const GITHUB_USER_URL: &str = "https://api.github.com/user";
const GITHUB_COPILOT_MODELS_URL: &str = "https://api.githubcopilot.com/models";
const USER_AGENT: &str = "Zap/1.0";
const EDITOR_VERSION: &str = "Zap/1.0";
const EDITOR_PLUGIN_VERSION: &str = "Zap/1.0";
const COPILOT_INTEGRATION_ID: &str = "zap";
const OAUTH_SCOPE: &str = "read:user";

#[derive(Debug, Clone)]
pub struct LoginFlow {
    pub auth_url: String,
    pub user_code: String,
    device_code: String,
    interval_secs: u64,
    expires_at_ms: u64,
    cancel_flag: Arc<AtomicBool>,
}

impl LoginFlow {
    pub fn cancel_handle(&self) -> Arc<AtomicBool> {
        self.cancel_flag.clone()
    }
}

#[derive(Debug, Deserialize)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: u64,
}

#[derive(Debug, Deserialize)]
struct AccessTokenResponse {
    #[serde(default)]
    access_token: Option<String>,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    error_description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GitHubUserResponse {
    login: String,
}

#[derive(Debug, Deserialize)]
struct CopilotTokenExchangeResponse {
    token: String,
    expires_at: u64,
}

#[cfg(not(target_family = "wasm"))]
fn build_blocking_reqwest_client() -> anyhow::Result<reqwest::blocking::Client> {
    use http_client::ProxyMode;

    let cfg = http_client::current_proxy_config();
    let builder = match cfg.mode {
        ProxyMode::System => reqwest::blocking::Client::builder(),
        ProxyMode::Off => reqwest::blocking::Client::builder().no_proxy(),
        ProxyMode::Custom => {
            let trimmed = cfg.url.trim();
            if trimmed.is_empty() {
                reqwest::blocking::Client::builder()
            } else {
                let mut proxy = reqwest::Proxy::all(trimmed)
                    .with_context(|| format!("无效的 HTTP 代理 URL: {trimmed}"))?;
                if !cfg.username.is_empty() || !cfg.password.is_empty() {
                    proxy = proxy.basic_auth(&cfg.username, &cfg.password);
                }
                if !cfg.no_proxy.trim().is_empty() {
                    if let Some(no_proxy) = reqwest::NoProxy::from_string(cfg.no_proxy.trim()) {
                        proxy = proxy.no_proxy(Some(no_proxy));
                    }
                }
                reqwest::blocking::Client::builder().proxy(proxy)
            }
        }
    };
    builder
        .build()
        .context("构造 Copilot blocking HTTP 客户端失败")
}

#[cfg(target_family = "wasm")]
fn build_blocking_reqwest_client() -> anyhow::Result<reqwest::blocking::Client> {
    reqwest::blocking::Client::builder()
        .build()
        .context("构造 Copilot blocking HTTP 客户端失败")
}

pub fn begin_login() -> anyhow::Result<LoginFlow> {
    let client = build_blocking_reqwest_client()?;
    let response = client
        .post(GITHUB_DEVICE_CODE_URL)
        .header("Accept", "application/json")
        .header("User-Agent", USER_AGENT)
        .form(&[
            ("client_id", GITHUB_COPILOT_OAUTH_CLIENT_ID),
            ("scope", OAUTH_SCOPE),
        ])
        .send()
        .context("请求 GitHub device code 失败")?
        .error_for_status()
        .context("GitHub device code 返回错误状态")?
        .json::<DeviceCodeResponse>()
        .context("解析 GitHub device code 响应失败")?;

    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or_default();

    Ok(LoginFlow {
        auth_url: response.verification_uri,
        user_code: response.user_code,
        device_code: response.device_code,
        interval_secs: response.interval.max(1),
        expires_at_ms: now_ms + response.expires_in.saturating_mul(1000),
        cancel_flag: Arc::new(AtomicBool::new(false)),
    })
}

pub fn request_headers() -> Vec<(String, String)> {
    vec![
        ("Editor-Version".to_string(), EDITOR_VERSION.to_string()),
        (
            "Editor-Plugin-Version".to_string(),
            EDITOR_PLUGIN_VERSION.to_string(),
        ),
        (
            "Copilot-Integration-Id".to_string(),
            COPILOT_INTEGRATION_ID.to_string(),
        ),
    ]
}

pub async fn wait_for_login(flow: LoginFlow) -> anyhow::Result<AgentProviderOAuthCredentials> {
    let client = http_client::Client::new();
    let mut interval_secs = flow.interval_secs;

    loop {
        if flow.cancel_flag.load(Ordering::Relaxed) {
            return Err(anyhow!("Copilot 登录已取消"));
        }

        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or_default();
        if now_ms >= flow.expires_at_ms {
            return Err(anyhow!("Copilot 登录已过期，请重新点击登录"));
        }

        tokio::time::sleep(Duration::from_secs(interval_secs)).await;

        let payload = client
            .post(GITHUB_ACCESS_TOKEN_URL)
            .header("Accept", "application/json")
            .header("User-Agent", USER_AGENT)
            .form(&[
                ("client_id", GITHUB_COPILOT_OAUTH_CLIENT_ID),
                ("device_code", flow.device_code.as_str()),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ])
            .send()
            .await
            .context("轮询 GitHub access token 失败")?
            .error_for_status()
            .map_err(|e| anyhow::Error::new(e).context("GitHub access token 返回错误状态"))?
            .json::<AccessTokenResponse>()
            .await
            .context("解析 GitHub access token 响应失败")?;

        if let Some(access_token) = payload.access_token {
            let copilot = exchange_github_token(&client, &access_token).await?;
            let account_id = fetch_github_login(&access_token).await?;
            return Ok(AgentProviderOAuthCredentials {
                kind: AgentProviderOAuthKind::Copilot,
                access_token: copilot.token,
                refresh_token: String::new(),
                expires_at_ms: copilot.expires_at.saturating_mul(1000),
                account_id,
            });
        }

        match payload.error.as_deref() {
            Some("authorization_pending") => {}
            Some("slow_down") => {
                interval_secs = interval_secs.saturating_add(5);
            }
            Some("access_denied") => return Err(anyhow!("Copilot 登录已取消")),
            Some("expired_token") => return Err(anyhow!("Copilot 登录已过期，请重新点击登录")),
            Some(other) => {
                let extra = payload
                    .error_description
                    .as_deref()
                    .map(|msg| format!(" ({msg})"))
                    .unwrap_or_default();
                return Err(anyhow!("Copilot OAuth 失败: {other}{extra}"));
            }
            None => {
                return Err(anyhow!("Copilot OAuth 响应缺少 access_token"));
            }
        }
    }
}

async fn exchange_github_token(
    client: &http_client::Client,
    github_token: &str,
) -> anyhow::Result<CopilotTokenExchangeResponse> {
    client
        .get(GITHUB_COPILOT_TOKEN_EXCHANGE_URL)
        .header("Authorization", format!("Token {github_token}"))
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .context("请求 GitHub Copilot token exchange 失败")?
        .error_for_status()
        .map_err(|e| anyhow::Error::new(e).context("GitHub Copilot token exchange 返回错误状态"))?
        .json::<CopilotTokenExchangeResponse>()
        .await
        .context("解析 GitHub Copilot token exchange 响应失败")
}

pub fn cancel_login(flow: &LoginFlow) {
    flow.cancel_flag.store(true, Ordering::Relaxed);
}

async fn fetch_github_login(token: &str) -> anyhow::Result<String> {
    let client = http_client::Client::new();
    let response: GitHubUserResponse = client
        .get(GITHUB_USER_URL)
        .header("Accept", "application/vnd.github+json")
        .header("Authorization", format!("Bearer {token}"))
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .context("请求 GitHub user 失败")?
        .error_for_status()
        .map_err(|e| anyhow::Error::new(e).context("GitHub user 返回错误状态"))?
        .json()
        .await
        .context("解析 GitHub user 响应失败")?;
    Ok(response.login)
}

pub async fn fetch_copilot_oauth_models(
    token: &str,
) -> anyhow::Result<Vec<crate::settings::AgentProviderModel>> {
    let client = http_client::Client::new();
    let headers = request_headers();
    let payload = client
        .get(GITHUB_COPILOT_MODELS_URL)
        .header("Accept", "application/json")
        .header("Authorization", format!("Bearer {token}"))
        .header("User-Agent", USER_AGENT)
        .header(&headers[0].0, &headers[0].1)
        .header(&headers[1].0, &headers[1].1)
        .header(&headers[2].0, &headers[2].1)
        .send()
        .await
        .context("请求 GitHub Copilot models 失败")?
        .error_for_status()
        .map_err(|e| anyhow::Error::new(e).context("GitHub Copilot models 返回错误状态"))?
        .json::<Value>()
        .await
        .context("解析 GitHub Copilot models 失败")?;

    let entries = match &payload {
        Value::Array(items) => items,
        Value::Object(map) => map
            .get("data")
            .and_then(Value::as_array)
            .ok_or_else(|| anyhow!("GitHub Copilot models 响应缺少 data 数组"))?,
        _ => {
            return Err(anyhow!(
                "GitHub Copilot models 响应格式不支持: 期望数组或 {{ data: [...] }}"
            ));
        }
    };

    let mut models: Vec<_> = entries
        .iter()
        .filter_map(parse_copilot_model)
        .collect();

    models.sort_by(|a, b| a.id.cmp(&b.id));
    models.dedup_by(|a, b| a.id == b.id);
    if models.is_empty() {
        return Err(anyhow!("GitHub Copilot models 返回了空模型列表"));
    }
    Ok(models)
}

fn parse_copilot_model(entry: &Value) -> Option<crate::settings::AgentProviderModel> {
    let id = entry.get("id")?.as_str()?.trim().to_owned();
    if id.is_empty() {
        return None;
    }

    let name = entry
        .get("name")
        .and_then(Value::as_str)
        .or_else(|| entry.get("display_name").and_then(Value::as_str))
        .or_else(|| entry.get("model_picker_name").and_then(Value::as_str))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(id.as_str())
        .to_owned();

    Some(crate::settings::AgentProviderModel {
        name,
        id,
        context_window: 0,
        max_output_tokens: 0,
        reasoning: false,
        tool_call: true,
        image: None,
        pdf: None,
        audio: None,
    })
}

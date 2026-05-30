//! Codex / ChatGPT OAuth login support for Agent Providers.

use std::collections::HashMap;
use std::io::{Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context as _};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rand::{distributions::Alphanumeric, Rng as _};
use serde::Deserialize;
use sha2::{Digest as _, Sha256};
use url::Url;

use super::{AgentProviderOAuthCredentials, AgentProviderOAuthKind};

pub const CODEX_PROVIDER_NAME: &str = "Codex Auth";
pub const CODEX_BASE_URL: &str = "https://chatgpt.com/backend-api/codex/";

const CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
const AUTHORIZE_URL: &str = "https://auth.openai.com/oauth/authorize";
const TOKEN_URL: &str = "https://auth.openai.com/oauth/token";
const REDIRECT_URI: &str = "http://localhost:1455/auth/callback";
const CALLBACK_ADDR: &str = "127.0.0.1:1455";
const SCOPE: &str = "openid profile email offline_access";

#[derive(Debug)]
pub struct LoginFlow {
    pub auth_url: String,
    pub code_verifier: String,
    pub callback_rx: Receiver<anyhow::Result<String>>,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: u64,
}

pub fn request_headers(account_id: &str) -> Vec<(String, String)> {
    vec![
        ("chatgpt-account-id".to_string(), account_id.to_string()),
        (
            "OpenAI-Beta".to_string(),
            "responses=experimental".to_string(),
        ),
        ("originator".to_string(), "codex_cli_rs".to_string()),
        ("accept".to_string(), "text/event-stream".to_string()),
    ]
}

pub fn begin_login() -> anyhow::Result<LoginFlow> {
    let listener = TcpListener::bind(CALLBACK_ADDR)
        .with_context(|| format!("无法监听 Codex OAuth 回调端口 {CALLBACK_ADDR}"))?;
    let state = random_string(32);
    let code_verifier = random_string(96);
    let code_challenge = pkce_challenge(&code_verifier);
    let (tx, rx) = mpsc::channel();
    let expected_state = state.clone();

    thread::spawn(move || {
        let result = receive_callback(listener, &expected_state);
        let _ = tx.send(result);
    });

    let mut auth_url = Url::parse(AUTHORIZE_URL)?;
    auth_url
        .query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair("client_id", CLIENT_ID)
        .append_pair("redirect_uri", REDIRECT_URI)
        .append_pair("scope", SCOPE)
        .append_pair("state", &state)
        .append_pair("code_challenge", &code_challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("id_token_add_organizations", "true")
        .append_pair("codex_cli_simplified_flow", "true")
        .append_pair("originator", "codex_cli_rs");

    Ok(LoginFlow {
        auth_url: auth_url.to_string(),
        code_verifier,
        callback_rx: rx,
    })
}

pub async fn wait_for_login(flow: LoginFlow) -> anyhow::Result<AgentProviderOAuthCredentials> {
    let code = tokio::task::spawn_blocking(move || {
        flow.callback_rx
            .recv_timeout(Duration::from_secs(300))
            .map_err(|_| anyhow!("Codex OAuth 登录超时"))?
            .map(|code| (code, flow.code_verifier))
    })
    .await
    .context("等待 Codex OAuth 回调失败")??;

    exchange_code(&code.0, &code.1).await
}

pub async fn refresh_credentials(
    credentials: &AgentProviderOAuthCredentials,
) -> anyhow::Result<AgentProviderOAuthCredentials> {
    let client = reqwest::Client::new();
    let token: TokenResponse = client
        .post(TOKEN_URL)
        .form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", credentials.refresh_token.as_str()),
            ("client_id", CLIENT_ID),
        ])
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    credentials_from_token_response(token, Some(credentials.refresh_token.clone()))
}

async fn exchange_code(
    code: &str,
    code_verifier: &str,
) -> anyhow::Result<AgentProviderOAuthCredentials> {
    let client = reqwest::Client::new();
    let token: TokenResponse = client
        .post(TOKEN_URL)
        .form(&[
            ("grant_type", "authorization_code"),
            ("client_id", CLIENT_ID),
            ("code", code),
            ("code_verifier", code_verifier),
            ("redirect_uri", REDIRECT_URI),
        ])
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    credentials_from_token_response(token, None)
}

fn credentials_from_token_response(
    token: TokenResponse,
    fallback_refresh_token: Option<String>,
) -> anyhow::Result<AgentProviderOAuthCredentials> {
    let refresh_token = token
        .refresh_token
        .or(fallback_refresh_token)
        .ok_or_else(|| anyhow!("Codex OAuth 响应缺少 refresh_token"))?;
    let account_id = chatgpt_account_id_from_jwt(&token.access_token)
        .ok_or_else(|| anyhow!("Codex OAuth access token 缺少 ChatGPT account id"))?;
    let now_ms = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64;
    Ok(AgentProviderOAuthCredentials {
        kind: AgentProviderOAuthKind::Codex,
        access_token: token.access_token,
        refresh_token,
        expires_at_ms: now_ms + token.expires_in.saturating_mul(1000),
        account_id,
    })
}

fn receive_callback(listener: TcpListener, expected_state: &str) -> anyhow::Result<String> {
    let (mut stream, _) = listener.accept()?;
    let request = read_http_request(&mut stream)?;
    let path = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .ok_or_else(|| anyhow!("Codex OAuth 回调请求格式无效"))?;
    let url = Url::parse(&format!("http://localhost{path}"))?;
    if url.path() != "/auth/callback" {
        write_callback_response(&mut stream, false)?;
        return Err(anyhow!("Codex OAuth 回调路径无效"));
    }
    let params: HashMap<String, String> = url.query_pairs().into_owned().collect();
    if params.get("state").map(String::as_str) != Some(expected_state) {
        write_callback_response(&mut stream, false)?;
        return Err(anyhow!("Codex OAuth state 校验失败"));
    }
    let code = params
        .get("code")
        .cloned()
        .ok_or_else(|| anyhow!("Codex OAuth 回调缺少 code"))?;
    write_callback_response(&mut stream, true)?;
    Ok(code)
}

fn read_http_request(stream: &mut TcpStream) -> anyhow::Result<String> {
    stream.set_read_timeout(Some(Duration::from_secs(30)))?;
    let mut buf = [0_u8; 4096];
    let n = stream.read(&mut buf)?;
    Ok(String::from_utf8_lossy(&buf[..n]).into_owned())
}

fn write_callback_response(stream: &mut TcpStream, ok: bool) -> anyhow::Result<()> {
    let body = if ok {
        "Codex login succeeded. You can close this tab and return to Zap."
    } else {
        "Codex login failed. Please return to Zap and try again."
    };
    let status = if ok { "200 OK" } else { "400 Bad Request" };
    write!(
        stream,
        "HTTP/1.1 {status}\r\ncontent-type: text/plain; charset=utf-8\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    )?;
    Ok(())
}

fn random_string(len: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}

fn pkce_challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

fn chatgpt_account_id_from_jwt(token: &str) -> Option<String> {
    let payload = token.split('.').nth(1)?;
    let bytes = URL_SAFE_NO_PAD.decode(payload).ok()?;
    let json: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    json.get("https://api.openai.com/auth")?
        .get("chatgpt_account_id")?
        .as_str()
        .map(str::to_owned)
}

pub fn codex_oauth_models() -> Vec<crate::settings::AgentProviderModel> {
    [
        ("GPT 5.5", "gpt-5.5"),
        ("GPT 5.4", "gpt-5.4"),
        ("GPT 5.4 Mini", "gpt-5.4-mini"),
        ("GPT 5.3 Codex", "gpt-5.3-codex"),
    ]
    .into_iter()
    .map(|(name, id)| crate::settings::AgentProviderModel {
        name: name.to_string(),
        id: id.to_string(),
        context_window: 272_000,
        max_output_tokens: 128_000,
        reasoning: true,
        tool_call: true,
        image: Some(true),
        pdf: Some(false),
        audio: Some(false),
    })
    .collect()
}

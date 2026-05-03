// openWarp(Channel::Oss)autoupdate 走 GitHub Releases API,而非 Warp 官方
// channel_versions / GCS。本模块只负责"取最新 release 元数据" + "按文件名挑资产";
// 实际的下载落盘 + 打开目录由 windows.rs / mac.rs 完成。

use std::sync::Mutex;
use std::time::Duration;

use anyhow::{Context as _, Result};
use lazy_static::lazy_static;
use serde::Deserialize;

const REPO_OWNER: &str = "zerx-lab";
const REPO_NAME: &str = "warp";

// GitHub 强制要求 User-Agent;同时显式声明 API 版本避免未来 default 漂移。
const USER_AGENT: &str = "OpenWarp-Autoupdate";
const ACCEPT: &str = "application/vnd.github+json";
const API_VERSION: &str = "2022-11-28";

const FETCH_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Deserialize)]
pub struct GithubRelease {
    pub tag_name: String,
    pub html_url: String,
    pub assets: Vec<GithubAsset>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GithubAsset {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
}

impl GithubRelease {
    pub fn version(&self) -> &str {
        self.tag_name.trim_start_matches('v')
    }

    pub fn find_asset(&self, expected_name: &str) -> Option<&GithubAsset> {
        self.assets.iter().find(|a| a.name == expected_name)
    }
}

lazy_static! {
    /// 最近一次 fetch 到的 release。fetch_version 写入,download_update 读取。
    /// 这样 download 阶段不必再次请求 GitHub API,也避免 race(两次请求间 release 翻新)。
    static ref LATEST_RELEASE: Mutex<Option<GithubRelease>> = Mutex::new(None);
}

pub fn cached_release() -> Option<GithubRelease> {
    LATEST_RELEASE.lock().ok().and_then(|g| g.clone())
}

fn store_cached(release: GithubRelease) {
    if let Ok(mut guard) = LATEST_RELEASE.lock() {
        *guard = Some(release);
    }
}

pub async fn fetch_latest_release(client: &http_client::Client) -> Result<GithubRelease> {
    let url = format!("https://api.github.com/repos/{REPO_OWNER}/{REPO_NAME}/releases/latest");
    log::info!("Fetching latest release from {url}");
    let release: GithubRelease = client
        .get(&url)
        .header("User-Agent", USER_AGENT)
        .header("Accept", ACCEPT)
        .header("X-GitHub-Api-Version", API_VERSION)
        .timeout(FETCH_TIMEOUT)
        .send()
        .await
        .context("调用 GitHub Releases API 失败")?
        .error_for_status()
        .context("GitHub Releases API 返回非 2xx 状态码")?
        .json()
        .await
        .context("解析 GitHub Releases JSON 失败")?;
    log::info!(
        "GitHub latest release: tag={} assets={}",
        release.tag_name,
        release.assets.len()
    );
    store_cached(release.clone());
    Ok(release)
}

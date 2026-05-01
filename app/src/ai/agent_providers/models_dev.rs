//! models.dev 数据源接入。
//!
//! 在用户打开 Providers 设置页时,后台异步拉取 `https://models.dev/api.json`,
//! 缓存到 `${cache_dir}/models-dev.json`。下一次启动直接读缓存,
//! 缓存命中且未过 TTL(默认 24h) 不再发请求;过期/缺失时再去拉。
//!
//! 数据结构对齐 opencode 的 `provider/models.ts`:顶层是
//! `{ <provider_id>: Provider }`,Provider 含 `models: { <model_id>: Model }`。
//! 我们只关心 UI "快速选择" 需要的几个字段:
//! - provider: id / name / api / env(暗示需要哪个 env var)
//! - model:    id / name / limit.context / limit.output / reasoning / tool_call
//!
//! 没列出的字段一律走 `serde(default)` + `#[allow(dead_code)]` 容忍。
//!
//! 设计取舍:**同步缓存读、异步网络拉**。读侧给 UI 用,要快;
//! 拉侧后台 spawn,失败不弹错只 log,缓存读不到就给空数据,UI 展示
//! "暂未拉取到 models.dev,请检查网络"。

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::sync::RwLock;
use std::time::{Duration, SystemTime};

use http_client::Client;
use serde::{Deserialize, Serialize};

const MODELS_DEV_URL: &str = "https://models.dev/api.json";
const CACHE_FILENAME: &str = "models-dev.json";
const CACHE_TTL: Duration = Duration::from_secs(24 * 60 * 60);
const FETCH_TIMEOUT: Duration = Duration::from_secs(15);

/// `models.dev` 顶层数据 — provider_id → Provider。
pub type Catalog = BTreeMap<String, Provider>;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Provider {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    /// 上游 API base URL,例如 `https://api.deepseek.com/v1`。
    #[serde(default)]
    pub api: Option<String>,
    /// 该 provider 通常需要的环境变量名,例如 `["DEEPSEEK_API_KEY"]`。
    #[serde(default)]
    pub env: Vec<String>,
    /// 可用模型,key 为模型 id。
    #[serde(default)]
    pub models: BTreeMap<String, Model>,
    /// 文档 URL(部分 provider 有)。
    #[serde(default)]
    pub doc: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Model {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub family: Option<String>,
    #[serde(default)]
    pub release_date: Option<String>,
    #[serde(default)]
    pub reasoning: bool,
    #[serde(default = "default_true")]
    pub tool_call: bool,
    /// 上下文窗口上限。
    #[serde(default)]
    pub limit: ModelLimit,
    /// "alpha" / "beta" / "deprecated" 标签。
    #[serde(default)]
    pub status: Option<String>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelLimit {
    #[serde(default)]
    pub context: u32,
    #[serde(default)]
    pub output: u32,
}

// ── 进程内单例缓存 ──────────────────────────────────────────────────────────

#[derive(Debug, Default)]
struct State {
    /// 已加载的 catalog。`None` 表示从未加载成功。
    catalog: Option<Catalog>,
    /// 缓存最后修改时间(用于判断是否过期)。
    loaded_at: Option<SystemTime>,
}

fn state() -> &'static RwLock<State> {
    static S: OnceLock<RwLock<State>> = OnceLock::new();
    S.get_or_init(|| RwLock::new(State::default()))
}

fn cache_path() -> PathBuf {
    let mut p = warp_core::paths::cache_dir();
    p.push(CACHE_FILENAME);
    p
}

/// 读已加载的 catalog 副本(无锁等待 — 直接克隆)。
/// 没数据返回 `None`,UI 应展示 "正在拉取" / 重试按钮。
pub fn cached() -> Option<Catalog> {
    state().read().ok().and_then(|s| s.catalog.clone())
}

/// 把磁盘缓存读进内存(同步,非阻塞;只在 process 启动或 UI 第一次需要时调用)。
/// 如果磁盘缓存不存在或解析失败,返回 false,调用方应触发一次网络拉取。
pub fn load_from_disk() -> bool {
    let path = cache_path();
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(_) => return false,
    };
    let mtime = std::fs::metadata(&path)
        .ok()
        .and_then(|m| m.modified().ok());
    match serde_json::from_slice::<Catalog>(&bytes) {
        Ok(catalog) => {
            if let Ok(mut s) = state().write() {
                s.catalog = Some(catalog);
                s.loaded_at = mtime;
            }
            true
        }
        Err(e) => {
            log::warn!("[models.dev] 解析磁盘缓存失败 ({path:?}): {e}");
            false
        }
    }
}

/// 缓存是否过期 — 不存在或超过 TTL。
pub fn is_stale() -> bool {
    let s = match state().read() {
        Ok(s) => s,
        Err(_) => return true,
    };
    match s.loaded_at {
        Some(t) => SystemTime::now()
            .duration_since(t)
            .map(|d| d > CACHE_TTL)
            .unwrap_or(true),
        None => true,
    }
}

/// 异步拉取 models.dev 并写入磁盘缓存与内存缓存。
/// 失败仅 log,不向上 propagate(UI 调用方按 `cached()` 是否为 `Some` 决定显示)。
pub async fn fetch_and_cache(client: Client) -> Result<(), String> {
    let resp = client
        .get(MODELS_DEV_URL)
        .timeout(FETCH_TIMEOUT)
        .send()
        .await
        .map_err(|e| format!("HTTP 请求失败: {e}"))?;

    let status = resp.status();
    if !status.is_success() {
        return Err(format!("HTTP {status}"));
    }
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("读响应体失败: {e}"))?;

    let catalog: Catalog =
        serde_json::from_slice(&bytes).map_err(|e| format!("JSON 解析失败: {e}"))?;

    // 写盘 — 失败不算致命,只 log。
    let path = cache_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Err(e) = std::fs::write(&path, &bytes) {
        log::warn!("[models.dev] 写磁盘缓存失败 ({path:?}): {e}");
    }

    if let Ok(mut s) = state().write() {
        s.catalog = Some(catalog);
        s.loaded_at = Some(SystemTime::now());
    }
    Ok(())
}

// ── chip 行折叠/展开状态(进程级,避免 widget rebuild 丢) ─────────────────

static CHIPS_EXPANDED: AtomicBool = AtomicBool::new(false);

pub fn chips_expanded() -> bool {
    CHIPS_EXPANDED.load(Ordering::Relaxed)
}

pub fn toggle_chips_expanded() {
    CHIPS_EXPANDED.fetch_xor(true, Ordering::Relaxed);
}

// ── 快速添加 chip 行的搜索过滤 ──────────────────────────────────────────────

fn search_state() -> &'static RwLock<String> {
    static S: OnceLock<RwLock<String>> = OnceLock::new();
    S.get_or_init(|| RwLock::new(String::new()))
}

pub fn search_query() -> String {
    search_state()
        .read()
        .ok()
        .map(|s| s.clone())
        .unwrap_or_default()
}

pub fn set_search_query(q: String) {
    if let Ok(mut s) = search_state().write() {
        *s = q;
    }
}

/// 按当前搜索 query 过滤 catalog,大小写不敏感子串匹配 provider.name 与 provider.id。
/// 空 query 返回全部条目顺序。返回拥有所有权的 Vec 以便 UI 端 take/iter。
pub fn filter_catalog(catalog: &Catalog, query: &str) -> Vec<(String, Provider)> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return catalog
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
    }
    catalog
        .iter()
        .filter(|(id, p)| id.to_lowercase().contains(&q) || p.name.to_lowercase().contains(&q))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

/// 把 models.dev 的 Model 转换成本地 settings 用的 AgentProviderModel。
pub fn into_agent_provider_model(model: &Model) -> crate::settings::AgentProviderModel {
    crate::settings::AgentProviderModel {
        name: if model.name.is_empty() {
            model.id.clone()
        } else {
            model.name.clone()
        },
        id: model.id.clone(),
        context_window: model.limit.context,
        max_output_tokens: model.limit.output,
        reasoning: model.reasoning,
        tool_call: model.tool_call,
    }
}

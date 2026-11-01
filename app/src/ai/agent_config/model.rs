//! Agent 配置条目的统一模型。
//!
//! 把 Claude / Codex / Grok（以及只读的 Gemini / Cursor）的
//! Rules · Hooks · Skills · Commands · Settings · MCP 抽象成同一套条目，
//! 供发现层、Settings 三栏页和命令面板共用。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// 配置所属的 CLI agent。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentKind {
    Claude,
    Codex,
    Grok,
    /// 共享的 `~/.agents` 命名空间（skills 扇出源）。
    Agents,
    /// 只读展示。
    Gemini,
    /// 只读展示。
    Cursor,
}

impl AgentKind {
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Claude => "Claude Code",
            Self::Codex => "Codex",
            Self::Grok => "Grok",
            Self::Agents => ".agents",
            Self::Gemini => "Gemini",
            Self::Cursor => "Cursor",
        }
    }

    /// 可编辑的 agent（Gemini / Cursor 只读）。
    pub fn is_editable(self) -> bool {
        matches!(self, Self::Claude | Self::Codex | Self::Grok | Self::Agents)
    }

    pub fn all() -> &'static [AgentKind] {
        &[
            Self::Claude,
            Self::Codex,
            Self::Grok,
            Self::Agents,
            Self::Gemini,
            Self::Cursor,
        ]
    }
}

/// 配置类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConfigKind {
    Rules,
    Hooks,
    Skills,
    Commands,
    Settings,
    Mcp,
}

impl ConfigKind {
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Rules => "Rules",
            Self::Hooks => "Hooks",
            Self::Skills => "Skills",
            Self::Commands => "Commands",
            Self::Settings => "Settings",
            Self::Mcp => "MCP",
        }
    }

    pub fn all() -> &'static [ConfigKind] {
        &[
            Self::Rules,
            Self::Hooks,
            Self::Skills,
            Self::Commands,
            Self::Settings,
            Self::Mcp,
        ]
    }
}

/// 作用域：越靠前优先级越高（展示用）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum ConfigScope {
    /// 企业 / 托管（Claude managed-settings）。
    Managed,
    /// 项目本地（settings.local.json 等，通常 gitignored）。
    Local,
    /// 仓库项目级（会进 git）。
    Project,
    /// 用户全局（~/.claude 等）。
    User,
}

impl ConfigScope {
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Managed => "托管",
            Self::Local => "本地",
            Self::Project => "项目",
            Self::User => "全局",
        }
    }
}

/// 文档格式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DocumentFormat {
    Json,
    Toml,
    Markdown,
    /// 技能目录等（只列元信息，不整文件编辑）。
    Directory,
}

/// 谁在托管这条配置（例如 zap-cli-agent-bridge）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ManagedBy {
    None,
    Bridge,
}

/// 统一配置条目。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentConfigItem {
    pub id: String,
    pub agent: AgentKind,
    pub kind: ConfigKind,
    pub scope: ConfigScope,
    pub title: String,
    pub source_path: PathBuf,
    /// JSON Pointer / TOML dotted path；整文件时为空。
    pub pointer: String,
    pub format: DocumentFormat,
    pub executable: bool,
    pub managed_by: ManagedBy,
    /// 项目级写入会进 git 时为 true。
    pub project_tracked: bool,
    /// Codex hooks 改后需重新信任。
    pub needs_codex_retrust: bool,
    pub summary: String,
    pub read_only: bool,
}

impl AgentConfigItem {
    pub fn new_id(agent: AgentKind, kind: ConfigKind, scope: ConfigScope, title: &str) -> String {
        format!(
            "{}:{}:{}:{}",
            agent.display_name(),
            kind.display_name(),
            scope.display_name(),
            title
        )
    }
}

/// 生效链上一环。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveLayer {
    pub scope: ConfigScope,
    pub path: PathBuf,
    pub exists: bool,
    pub wins: bool,
}

/// 扫描上下文。
#[derive(Debug, Clone, Default)]
pub struct ScanContext {
    pub home: Option<PathBuf>,
    pub repo_root: Option<PathBuf>,
}

impl ScanContext {
    pub fn from_env(repo_root: Option<PathBuf>) -> Self {
        Self {
            home: dirs::home_dir(),
            repo_root,
        }
    }

    pub fn home_path(&self, rel: impl AsRef<Path>) -> Option<PathBuf> {
        self.home.as_ref().map(|h| h.join(rel))
    }

    pub fn repo_path(&self, rel: impl AsRef<Path>) -> Option<PathBuf> {
        self.repo_root.as_ref().map(|r| r.join(rel))
    }
}

/// 体检结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthFinding {
    pub severity: HealthSeverity,
    pub agent: AgentKind,
    pub title: String,
    pub detail: String,
    pub fixable_by_bridge: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthSeverity {
    Info,
    Warn,
    Error,
}

//! "Candidates" 区域的视图模型 —— 把 `warp_ssh_manager::load_candidates()`
//! 的结果(及已导入别名集合、折叠状态)摊平成 UI 友好的 [`CandidateRow`]
//! 列表。
//!
//! 设计要点(对应 `specs/gh-110-ssh-config-import/{PRODUCT,TECH}.md`):
//!
//! - `rows()` 是**纯函数**:只依赖 view-model 的当前字段,不碰 IO / runtime,
//!   单元测试可以直接构造一个 `CandidatesViewModel` 并断言输出。这正是 TDD
//!   讨论里要求的点 —— PR 2 的渲染层 warpui 测试代价太高,把"哪些行该显示"
//!   的逻辑抽出来单测就够覆盖关键判断。
//! - `refresh()` 同步调用 `warp_ssh_manager::load_candidates()`(<10KB 文件,
//!   见 TECH.md §3.1 的取舍),把结果存进 `state`。
//! - `on_tree_changed()` 由 panel 在订阅 `SshTreeChangedNotifier` 后调用 —— 把
//!   保存树里所有 server 的 `host` 字段收集成 `HashSet`,作为 "Added" 徽章的
//!   判定依据(PRODUCT.md decision E)。
//! - "已导入"的判定按 `host == alias` 做。导入逻辑在 panel 侧把 `server.host`
//!   设成候选别名(PRODUCT.md decision I),所以这里的比较语义与导入语义一致。
//!
//! 字段全部 `pub(crate)`,只让 `panel.rs` 看见;`CandidatesViewModel` 本身
//! 通过 `pub` 暴露给 `mod.rs` 的 re-export。

use std::collections::HashSet;

use warpui::{Entity, ModelContext};

use settings::Setting;
use warp_ssh_manager::{LoadOutcome, LoadResult, SshConfigCandidate, load_candidates};
use warpui::SingletonEntity;

use crate::settings::SshSettings;

/// `~/.ssh/config` 一行候选服务器在 UI 中的来源 + 状态视图。
pub struct CandidatesViewModel {
    /// 最近一次加载结果。`None` 表示模型刚创建、尚未触发任何 refresh。
    state: Option<LoadResult>,
    /// 保存树里所有 server 的 `host` 字段集合。`rows()` 用它判断 `added`。
    added_aliases: HashSet<String>,
    /// 区段折叠状态(PRODUCT.md UX 表 "Many candidates")。默认展开。
    expanded: bool,
}

impl Default for CandidatesViewModel {
    fn default() -> Self {
        Self::new()
    }
}

impl CandidatesViewModel {
    /// 全空构造器 —— 模型刚被 `add_model` 进 App 时使用。`refresh()` 必须由
    /// 调用方在合适时机触发(panel `new` 里立刻调一次即可)。
    pub fn new() -> Self {
        Self {
            state: None,
            added_aliases: HashSet::new(),
            expanded: true,
        }
    }

    /// 测试用构造器:把内部状态显式塞进去,避开 runtime / IO,直接驱动
    /// `rows()` 各种分支。
    #[cfg(test)]
    pub fn with_state(
        state: Option<LoadResult>,
        added_aliases: HashSet<String>,
        expanded: bool,
    ) -> Self {
        Self {
            state,
            added_aliases,
            expanded,
        }
    }

    /// 同步重新读 `~/.ssh/config`,把结果存入 `state`。
    ///
    /// 设计上不返回错误 —— `LoadOutcome::Error` 已经把错误信息字符串带回,
    /// UI 用红色错误行展示(见 PRODUCT.md UX 表 "Parse / IO error")。
    ///
    /// 当"自动发现 SSH 主机"设置关闭时,跳过读取并清空状态。
    pub fn refresh(&mut self, ctx: &mut ModelContext<Self>) {
        let auto_discover = *SshSettings::as_ref(ctx).enable_ssh_auto_discovery.value();
        if !auto_discover {
            self.state = None;
            ctx.notify();
            return;
        }
        self.state = Some(load_candidates());
        ctx.notify();
    }

    /// 树变更回调 —— 用传入的 server hosts 重建 `added_aliases`。
    ///
    /// 接收 `impl IntoIterator<Item = String>` 而不是 `&SshRepository` 让测试
    /// 不必塞一个真实的 SQLite 连接;调用方(panel)负责把 `list_nodes` +
    /// `get_server` 的 host 字段收集成迭代器再传入。
    pub fn on_tree_changed<I>(&mut self, hosts: I, ctx: &mut ModelContext<Self>)
    where
        I: IntoIterator<Item = String>,
    {
        self.added_aliases = hosts.into_iter().collect();
        ctx.notify();
    }

    /// 切换"区段折叠"状态。
    pub fn toggle_expanded(&mut self, ctx: &mut ModelContext<Self>) {
        self.expanded = !self.expanded;
        ctx.notify();
    }

    /// 是否展开(panel 渲染时决定是否显示 body 行)。
    pub fn is_expanded(&self) -> bool {
        self.expanded
    }

    /// 按 alias 查找候选 —— `ImportCandidate { alias }` action 处理时用,
    /// 拿到完整字段后调 `SshRepository::create_server`。
    pub fn find_candidate(&self, alias: &str) -> Option<&SshConfigCandidate> {
        let state = self.state.as_ref()?;
        match &state.outcome {
            LoadOutcome::Loaded(v) => v.iter().find(|c| c.alias == alias),
            LoadOutcome::NotFound | LoadOutcome::Error(_) => None,
        }
    }

    /// 当前 `~/.ssh/config` 路径的可读字符串(给 `notes = "Imported from {}"`
    /// 用)。`None` 表示还没加载过、或连 home 都拿不到。
    pub fn path_display(&self) -> Option<String> {
        self.state
            .as_ref()
            .and_then(|s| s.path.as_ref())
            .map(|p| p.display().to_string())
    }

    /// 把当前状态摊平成行列表 —— 见模块文档的"纯函数"约定。
    ///
    /// 输出语义(对应 PRODUCT.md §5 UX 表):
    /// - 还没 refresh:返回空 Vec(panel 在拿到 `state == None` 时不渲染区段)。
    /// - `NotFound`:Header + 一行 `NotFound`。
    /// - `Error`:Header + 一行 `Error`(can_refresh=true 让用户改完 config 后重试)。
    /// - `Loaded(empty)`:Header + 一行 `Empty`。
    /// - `Loaded(non-empty)`:Header(count = N)+ N 行 `Candidate`,每行
    ///   `added` 由 `added_aliases.contains(alias)` 决定。
    pub fn rows(&self) -> Vec<CandidateRow> {
        let Some(state) = self.state.as_ref() else {
            return Vec::new();
        };

        let path_display = state
            .path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default();

        let mut out = Vec::new();
        let count = match &state.outcome {
            LoadOutcome::Loaded(v) => v.len(),
            LoadOutcome::NotFound | LoadOutcome::Error(_) => 0,
        };
        // Header 永远第一行 —— 即便区段折叠了,panel 仍要画 header(那是
        // toggle 入口)。`can_refresh = true` 总成立:任何状态都允许用户点
        // Refresh 重读。
        out.push(CandidateRow::Header {
            path_display: path_display.clone(),
            count,
            can_refresh: true,
        });

        // 区段折叠时只保留 header,body 不渲染。
        if !self.expanded {
            return out;
        }

        match &state.outcome {
            LoadOutcome::NotFound => {
                out.push(CandidateRow::NotFound { path_display });
            }
            LoadOutcome::Error(msg) => {
                out.push(CandidateRow::Error {
                    path_display,
                    message: msg.clone(),
                });
            }
            LoadOutcome::Loaded(v) if v.is_empty() => {
                out.push(CandidateRow::Empty { path_display });
            }
            LoadOutcome::Loaded(v) => {
                for c in v {
                    out.push(CandidateRow::Candidate {
                        alias: c.alias.clone(),
                        hostname: c.hostname.clone(),
                        user: c.user.clone(),
                        port: c.port,
                        identity_file: c.identity_file.as_ref().map(|p| p.display().to_string()),
                        added: self.added_aliases.contains(&c.alias),
                    });
                }
            }
        }

        out
    }
}

/// UI 友好的一行。Header 永远在最前面,后面要么是单条状态行(NotFound /
/// Empty / Error),要么是一串 Candidate。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CandidateRow {
    Header {
        path_display: String,
        count: usize,
        can_refresh: bool,
    },
    NotFound {
        path_display: String,
    },
    Empty {
        path_display: String,
    },
    Error {
        path_display: String,
        message: String,
    },
    Candidate {
        alias: String,
        hostname: Option<String>,
        user: Option<String>,
        port: Option<u16>,
        identity_file: Option<String>,
        added: bool,
    },
}

impl Entity for CandidatesViewModel {
    type Event = ();
}

#[cfg(test)]
#[path = "candidates_tests.rs"]
mod tests;

// 让测试代码不必关心 PathBuf 的具体磁盘路径 —— helper 用 `LoadResult` 拼一个
// 固定的展示串。测试模块里也会用到,所以放在外层以方便 #[cfg(test)] 复用。
#[cfg(test)]
pub(crate) fn fake_load_result_loaded(path: &str, cands: Vec<SshConfigCandidate>) -> LoadResult {
    LoadResult {
        path: Some(std::path::PathBuf::from(path)),
        outcome: LoadOutcome::Loaded(cands),
    }
}

#[cfg(test)]
pub(crate) fn fake_load_result_not_found(path: &str) -> LoadResult {
    LoadResult {
        path: Some(std::path::PathBuf::from(path)),
        outcome: LoadOutcome::NotFound,
    }
}

#[cfg(test)]
pub(crate) fn fake_load_result_error(path: &str, msg: &str) -> LoadResult {
    LoadResult {
        path: Some(std::path::PathBuf::from(path)),
        outcome: LoadOutcome::Error(msg.to_string()),
    }
}

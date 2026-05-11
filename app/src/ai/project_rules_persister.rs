//! `ProjectRulesPersister` — 项目规则(WARP.md / AGENTS.md)持久化桥。
//!
//! 这个 thin singleton model 的责任只有两件:
//!
//! 1. 订阅 [`ProjectContextModel`] 的 [`KnownRulesChanged`] 事件,把
//!    `discovered_rules` / `deleted_rules` 转成 [`ModelEvent::UpsertProjectRules`] /
//!    [`ModelEvent::DeleteProjectRules`] 写到 SQLite `project_rules` 表;
//! 2. 订阅 [`DetectedRepositories`] 的 `DetectedGitRepo` 事件,在用户进入新 git
//!    仓库时触发 [`ProjectContextModel::index_and_store_rules`] 扫描 WARP.md /
//!    AGENTS.md。
//!
//! 这两条逻辑历史上挂在 `PersistedWorkspace::new` 内,与 LSP 启用持久化和"已访问
//! git 仓库历史"紧紧耦合。LSP + workspace 历史下线后这条桥必须独立活下来,
//! 否则 project rules 不再写盘 / 不再随 cd 自动扫描。

use std::sync::mpsc::SyncSender;

use ai::project_context::model::{ProjectContextModel, ProjectContextModelEvent};
use repo_metadata::repositories::{DetectedRepositories, DetectedRepositoriesEvent};
use warpui::{Entity, ModelContext, SingletonEntity};

use crate::persistence::ModelEvent;

/// 详见模块级文档。
pub struct ProjectRulesPersister {
    /// 写入 SQLite 的 channel,`None` 表示当前构建未启用持久化。
    persistence_tx: Option<SyncSender<ModelEvent>>,
}

impl Entity for ProjectRulesPersister {
    type Event = ();
}

impl SingletonEntity for ProjectRulesPersister {}

impl ProjectRulesPersister {
    /// 注册两个订阅:
    /// - `ProjectContextModel` → 把 rule delta 转成 SQLite ModelEvent;
    /// - `DetectedRepositories` → 进入 git 仓库时触发 rule 扫描。
    pub fn new(
        persistence_tx: Option<SyncSender<ModelEvent>>,
        ctx: &mut ModelContext<Self>,
    ) -> Self {
        ctx.subscribe_to_model(&ProjectContextModel::handle(ctx), |me, event, _ctx| {
            let ProjectContextModelEvent::KnownRulesChanged(delta) = event else {
                return;
            };

            let mut events = vec![];

            if !delta.discovered_rules.is_empty() {
                events.push(ModelEvent::UpsertProjectRules {
                    project_rule_paths: delta.discovered_rules.clone(),
                });
            }

            if !delta.deleted_rules.is_empty() {
                events.push(ModelEvent::DeleteProjectRules {
                    path: delta.deleted_rules.clone(),
                });
            }

            if events.is_empty() {
                return;
            }

            let Some(tx) = me.persistence_tx.as_ref() else {
                return;
            };

            for event in events {
                if let Err(err) = tx.send(event) {
                    log::warn!("ProjectRulesPersister: 写入 SQLite 失败: {err}");
                }
            }
        });

        ctx.subscribe_to_model(&DetectedRepositories::handle(ctx), |_me, event, ctx| {
            let DetectedRepositoriesEvent::DetectedGitRepo { repository, .. } = event;
            let repo_path = repository.as_ref(ctx).root_dir().to_local_path_lossy();

            ProjectContextModel::handle(ctx).update(ctx, |model, ctx| {
                let _ = model.index_and_store_rules(repo_path, ctx);
            });
        });

        Self { persistence_tx }
    }

    /// 仅用于测试:不绑定持久化 channel,也不订阅任何 model。
    #[cfg(test)]
    pub fn new_for_test(_ctx: &mut ModelContext<Self>) -> Self {
        Self {
            persistence_tx: None,
        }
    }
}

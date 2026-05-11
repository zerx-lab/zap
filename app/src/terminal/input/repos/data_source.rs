//! Async data source for the inline repos menu.
//!
//! 历史上这里从 `PersistedWorkspace` 拉「之前打开过的 git 仓库」列表。
//! LSP + workspace 历史下线后,这个候选源已不存在,因此本 data source
//! 仅保留 trait 与 view 接线,永远返回空结果 —— 也就是说菜单仍能被
//! 唤出但永远没有候选项。这样可以避免大改上层 view / suggestions mode
//! 的接线,等未来若要接入「当前 pane group 实时 cwd」再补回数据来源。

use warpui::{AppContext, Entity};

use crate::search::data_source::{Query, QueryResult};
use crate::search::mixer::{AsyncDataSource, BoxFuture, DataSourceRunErrorWrapper};
use crate::terminal::input::repos::AcceptRepo;

pub struct RepoMenuDataSource;

impl RepoMenuDataSource {
    pub fn new() -> Self {
        Self
    }
}

impl AsyncDataSource for RepoMenuDataSource {
    type Action = AcceptRepo;

    fn run_query(
        &self,
        _query: &Query,
        _app: &AppContext,
    ) -> BoxFuture<'static, Result<Vec<QueryResult<Self::Action>>, DataSourceRunErrorWrapper>> {
        Box::pin(async move { Ok(Vec::new()) })
    }
}

impl Entity for RepoMenuDataSource {
    type Event = ();
}

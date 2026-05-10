//! OpenWarp:`SearchCodebase` action 的 stub 实现。
//!
//! 历史:原 `SearchCodebaseExecutor` 依赖
//! - `RepoOutlines`(tree-sitter 全仓符号索引)
//! - `GetRelevantFilesController`(走仪家服务端 RAG 或 BYOP one-shot 选文件)
//!
//! OpenWarp BYOP 模式下,这两条路径已彻底移除(启动时不再做 outline 解析,
//! 节省冷启动 ~5 秒后台 CPU)。`SearchCodebase` tool 也已经从 BYOP agent 的
//! tools registry 中移除(`crates/.../tools/codebase.rs`),理论上 LLM
//! 不会调它。这里保留 stub 是为了:
//! 1. 协议层(api / convert / yaml 序列化)中 `SearchCodebase` action 类型仍存在,
//!    上游恢复某些路径/老 conversation 重放等场景下还可能构造出该 action。
//! 2. 维持 `BlocklistAIActionExecutor` 的 dispatch 表完整,避免 panic。
//!
//! 行为:任何 SearchCodebase action 都同步返回 `Failed`,告诉模型改用
//! `read_files` / `grep` / `file_glob`。

use std::path::Path;

use futures::{future::BoxFuture, FutureExt};
use warpui::{Entity, EntityId, ModelContext, ModelHandle};

use crate::{
    ai::agent::{
        AIAgentAction, AIAgentActionId, AIAgentActionResultType, AIAgentActionType,
        SearchCodebaseFailureReason, SearchCodebaseResult,
    },
    terminal::model::session::active_session::ActiveSession,
};

use super::{ActionExecution, ExecuteActionInput, PreprocessActionInput};

pub struct SearchCodebaseExecutor {
    // 字段保留以支持构造接口的兼容性,实际不再使用。
    _active_session: ModelHandle<ActiveSession>,
    _terminal_view_id: EntityId,
}

impl SearchCodebaseExecutor {
    pub fn new(
        active_session: ModelHandle<ActiveSession>,
        terminal_view_id: EntityId,
        _ctx: &mut ModelContext<Self>,
    ) -> Self {
        Self {
            _active_session: active_session,
            _terminal_view_id: terminal_view_id,
        }
    }

    pub(super) fn should_autoexecute(
        &self,
        _input: ExecuteActionInput,
        _ctx: &mut ModelContext<Self>,
    ) -> bool {
        // 直接 autoexecute,会立即同步返回 Failed,告知模型工具不可用。
        true
    }

    pub(super) fn execute(
        &mut self,
        input: ExecuteActionInput,
        _ctx: &mut ModelContext<Self>,
    ) -> ActionExecution<anyhow::Result<SearchCodebaseResult>> {
        let ExecuteActionInput { action, .. } = input;
        let AIAgentAction {
            action: AIAgentActionType::SearchCodebase(_),
            ..
        } = action
        else {
            return ActionExecution::InvalidAction;
        };

        ActionExecution::Sync(AIAgentActionResultType::SearchCodebase(
            SearchCodebaseResult::Failed {
                reason: SearchCodebaseFailureReason::CodebaseNotIndexed,
                message: "Codebase semantic search is not available in this build. \
                     Use `read_files`, `grep`, or `file_glob` to locate code instead."
                    .to_owned(),
            },
        ))
    }

    /// 历史 API:返回某次 SearchCodebase action 对应的 repo 根目录。stub 永远返回 None。
    pub fn root_repo_for_action(&self, _id: &AIAgentActionId) -> Option<&Path> {
        None
    }

    pub(super) fn cancel_execution(
        &mut self,
        _action_id: &AIAgentActionId,
        _ctx: &mut ModelContext<Self>,
    ) {
        // stub 同步返回结果,没有需要取消的异步任务。
    }

    pub(super) fn preprocess_action(
        &mut self,
        _input: PreprocessActionInput,
        _ctx: &mut ModelContext<Self>,
    ) -> BoxFuture<'static, ()> {
        // stub 不做任何预处理。
        futures::future::ready(()).boxed()
    }
}

impl Entity for SearchCodebaseExecutor {
    type Event = ();
}

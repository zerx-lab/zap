use std::path::PathBuf;

/// 已确认属于 linked worktree 的终端工作目录标识。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorktreeIdentity {
    pub repository_path: PathBuf,
    pub worktree_path: PathBuf,
    pub branch: String,
}

/// 归类页签中的终端工作目录。
///
/// 只有全部可识别目录指向同一个 linked worktree 时才返回归属, 避免把混合页签
/// 错误迁移到某个 workspace。
pub fn classify_tab_worktree(
    identities: impl IntoIterator<Item = Option<WorktreeIdentity>>,
) -> Option<WorktreeIdentity> {
    let mut identities = identities.into_iter().flatten();
    let first = identities.next()?;
    identities
        .all(|identity| identity == first)
        .then_some(first)
}

/// repository 或 workspace 在 Zap 外部变更后的可见健康状态。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WorkspaceHealth {
    Ready,
    RepositoryMissing,
    WorktreeMissing,
    BranchMissing,
    WorktreeBranchMismatch { actual: String },
}

use std::{
    ffi::{OsStr, OsString},
    io,
    path::{Component, Path, PathBuf},
    process::Output,
};

use thiserror::Error;

mod ref_transaction;
pub use ref_transaction::RefTransactionError;
use ref_transaction::{LockedRef, PreparedRefDelete};

/// Git 分支引用，保留完整 refname 以避免通过名称前缀猜测类型。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BranchRef {
    Local {
        name: String,
        full_ref: String,
    },
    Remote {
        remote: String,
        name: String,
        full_ref: String,
    },
}

/// 已通过主工作目录、remote 和默认分支校验的 repository 元数据。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedRepository {
    pub root: PathBuf,
    pub primary_branch: String,
    pub remote: String,
    pub remote_url: String,
    pub default_branch: BranchRef,
}

/// `git worktree list --porcelain` 返回的 worktree 信息。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorktreeInfo {
    pub path: PathBuf,
    pub head: Option<String>,
    pub branch: Option<String>,
    pub is_bare: bool,
    pub is_detached: bool,
    pub is_locked: bool,
    pub locked_reason: Option<String>,
    pub is_prunable: bool,
    pub prunable_reason: Option<String>,
}

/// 可作为 repository workspace 接入的已注册 worktree。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExistingWorktreeOption {
    pub path: PathBuf,
    pub branch_name: String,
    pub is_primary: bool,
}

impl ExistingWorktreeOption {
    /// 创建 linked worktree 的接入选项。
    pub fn new(path: PathBuf, branch_name: impl Into<String>) -> Self {
        Self {
            path,
            branch_name: branch_name.into(),
            is_primary: false,
        }
    }

    /// 创建主 worktree 的接入选项。
    pub fn primary(path: PathBuf, branch_name: impl Into<String>) -> Self {
        Self {
            path,
            branch_name: branch_name.into(),
            is_primary: true,
        }
    }
}

/// 删除预检中的不可变合并目标快照。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MergeTargetSnapshot {
    /// 用于合并判断的完整 refname。
    pub full_ref: String,
    /// Preflight 时目标 ref 指向的精确 commit OID。
    pub oid: String,
    /// Preflight 时分支是否已合入目标 commit。
    pub is_merged: bool,
}

/// 删除 linked worktree 前提供给 UI 的只读建议快照。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeletionPreflight {
    /// `list_worktrees` 中已验证的 canonical registered path。
    pub worktree_path: PathBuf,
    /// Worktree 当前检出的本地分支短名称。
    pub branch: String,
    /// Worktree 当前检出的本地分支完整 refname。
    pub branch_ref: String,
    /// Preflight 时本地分支指向的精确 commit OID。
    pub branch_oid: String,
    /// 请求删除分支时捕获的合并目标快照；否则为 `None`。
    pub merge_target: Option<MergeTargetSnapshot>,
}

/// Repository workspace Git 操作的结构化错误。
#[derive(Debug, Error)]
pub enum GitWorkspaceError {
    #[error("failed to prepare branch deletion transaction: {source}")]
    RefTransaction {
        #[source]
        source: RefTransactionError,
    },
    #[error("branch deletion candidate for `{branch_ref}` has no merge target")]
    MissingMergeTarget { branch_ref: String },
    #[error(
        "merge target changed from `{expected}` to `{actual}` while branch deletion was locked"
    )]
    MergeTargetChanged { expected: String, actual: String },
    #[error(
        "ref `{full_ref}` changed while branch deletion was locked: expected {expected_oid}, found {actual_oid}"
    )]
    RefChanged {
        full_ref: String,
        expected_oid: String,
        actual_oid: String,
    },
    #[error(
        "{operation_error}; aborting the branch deletion transaction also failed: {abort_error}"
    )]
    BranchDeleteAbortFailed {
        #[source]
        operation_error: Box<GitWorkspaceError>,
        abort_error: RefTransactionError,
    },
    #[error(
        "worktree `{worktree_path}` was removed, but committing deletion of `{branch_ref}` at {branch_oid} failed: {source}; branch inspection error: {inspection_error:?}"
    )]
    BranchDeleteTransactionFailed {
        worktree_path: PathBuf,
        worktree_removed: bool,
        branch_ref: String,
        branch_oid: String,
        merge_target_ref: Option<String>,
        merge_target_oid: Option<String>,
        #[source]
        source: RefTransactionError,
        inspection_error: Option<Box<GitWorkspaceError>>,
    },
    #[error("failed to canonicalize `{path}`: {source}")]
    Canonicalize {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("selected path `{selected}` is not repository root `{root}`")]
    NotRepositoryRoot { selected: PathBuf, root: PathBuf },
    #[error(
        "linked worktree cannot be registered as a repository: git dir `{git_dir}`, common dir `{common_dir}`"
    )]
    LinkedWorktree {
        git_dir: PathBuf,
        common_dir: PathBuf,
    },
    #[error(
        "repository primary worktree `{path}` is detached or does not check out a local branch"
    )]
    PrimaryWorktreeDetached { path: PathBuf },
    #[error("prunable worktree `{path}` cannot be registered as a workspace")]
    PrunableWorktreeCannotBeWorkspace { path: PathBuf },
    #[error("repository `{repo}` has no configured remote")]
    RemoteNotFound { repo: PathBuf },
    #[error("remote `{remote}` in repository `{repo}` has no default branch: {stderr}")]
    DefaultBranchNotFound {
        repo: PathBuf,
        remote: String,
        stderr: String,
    },
    #[error("failed to execute git for {operation} with arguments {args:?}: {source}")]
    CommandIo {
        operation: &'static str,
        args: Vec<String>,
        #[source]
        source: io::Error,
    },
    #[error("git failed to {operation} with arguments {args:?}: {stderr}")]
    CommandFailed {
        operation: &'static str,
        args: Vec<String>,
        stderr: String,
    },
    #[error("git returned invalid UTF-8 while attempting to {operation}")]
    InvalidUtf8 { operation: &'static str },
    #[error("git returned invalid branch ref `{full_ref}`")]
    InvalidBranchRef { full_ref: String },
    #[error("selected remote ref `{full_ref}` is not a direct ref of a configured remote")]
    InvalidRemoteRef { full_ref: String },
    #[error("branch ref `{branch_ref}` cannot use itself as merge target `{target_ref}")]
    InvalidMergeTarget {
        branch_ref: String,
        target_ref: String,
    },
    #[error("branch ref `{full_ref}` does not exist")]
    BranchNotFound { full_ref: String },
    #[error("branch name `{branch}` is invalid")]
    InvalidBranchName { branch: String },
    #[error("local branch `{branch}` already exists")]
    BranchAlreadyExists { branch: String },
    #[error("local branch `{branch}` is already checked out at `{path}`")]
    BranchAlreadyCheckedOut { branch: String, path: PathBuf },
    #[error("worktree `{path}` is not registered in the repository")]
    WorktreeNotFound { path: PathBuf },
    #[error("worktree `{path}` appears more than once in the repository")]
    AmbiguousWorktree { path: PathBuf },
    #[error("worktree `{path}` does not check out a local branch")]
    WorktreeHasNoLocalBranch { path: PathBuf },
    #[error("worktree `{path}` contains uncommitted changes")]
    DirtyWorktree { path: PathBuf },
    #[error("worktree branch mismatch: expected `{expected}`, found `{actual}`")]
    WorktreeBranchMismatch { expected: String, actual: String },
    #[error("branch `{branch}` is not merged into `{merge_target}`")]
    BranchNotMerged {
        branch: String,
        merge_target: String,
    },
    #[error(
        "branch `{branch}` changed after preflight: expected {expected_oid}, found {actual_oid:?}"
    )]
    BranchChanged {
        branch: String,
        expected_oid: String,
        actual_oid: Option<String>,
        actual_symbolic_target: Option<String>,
    },
    #[error("git returned invalid branch ref record `{record}`")]
    InvalidBranchRefRecord { record: String },
    #[error("remote ref `{full_ref}` matches multiple remotes: {remotes:?}")]
    AmbiguousRemoteRef {
        full_ref: String,
        remotes: Vec<String>,
    },
    #[error("git returned invalid worktree record: {record}")]
    InvalidWorktreeRecord { record: String },
    #[error("target `{path}` already exists")]
    TargetExists { path: PathBuf },
    #[error("failed to claim target directory `{path}`: {source}")]
    TargetClaimFailed {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to inspect claimed target directory `{path}`: {source}")]
    ClaimedTargetInspection {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("claimed target `{path}` is not a directory")]
    ClaimedTargetNotDirectory { path: PathBuf },
    #[error("claimed target `{path}` is not empty")]
    ClaimedTargetNotEmpty { path: PathBuf },
    #[error("failed to clean up claimed target directory `{path}`: {source}")]
    ClaimedTargetCleanupFailed {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error(
        "worktree creation failed for `{branch}` at `{worktree_path}`: {create_error}; branch may remain: {branch_may_remain}; registered: {worktree_registered:?}; claimed directory removed: {claimed_directory_removed}; cleanup error: {cleanup_error:?}"
    )]
    WorktreeCreationFailed {
        worktree_path: PathBuf,
        branch: String,
        branch_may_remain: bool,
        worktree_registered: Option<bool>,
        claimed_directory_removed: bool,
        #[source]
        create_error: Box<GitWorkspaceError>,
        cleanup_error: Option<Box<GitWorkspaceError>>,
    },
    #[error(
        "worktree `{worktree_path}` for branch `{branch}` was created but could not be verified at expected OID {expected_oid}; the worktree and branch may remain: {verification_error}"
    )]
    WorktreeCreationVerificationFailed {
        worktree_path: PathBuf,
        branch: String,
        expected_oid: String,
        #[source]
        verification_error: Box<GitWorkspaceError>,
    },
    #[error("newly created branch `{branch}` unexpectedly tracks `{upstream}")]
    UnexpectedBranchUpstream { branch: String, upstream: String },
    #[error("git returned invalid direct ref record `{record}`")]
    InvalidDirectRefRecord { record: String },
    #[error("failed to create clone target `{path}`: {source}")]
    CreateTarget {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error(
        "clone failed: {clone_error}; target `{path}` could not be cleaned up: {cleanup_source}"
    )]
    CleanupFailed {
        path: PathBuf,
        cleanup_source: io::Error,
        #[source]
        clone_error: Box<GitWorkspaceError>,
    },
    #[error("Git URL `{url}` does not contain a repository name")]
    RepositoryNameMissing { url: String },
    #[error("clone directory name `{name}` must be a single normal path component")]
    InvalidCloneDirectoryName { name: String },
    #[error("background Git operation `{operation}` failed: {message}")]
    BackgroundTaskFailed {
        operation: &'static str,
        message: String,
    },
}

/// 校验路径是 Git 主工作目录，并读取 remote URL 与 remote 默认分支。
pub fn validate_repository(path: &Path) -> Result<ValidatedRepository, GitWorkspaceError> {
    let selected = canonicalize(path)?;
    let root = output_path(
        &selected,
        "find repository root",
        &["rev-parse", "--show-toplevel"],
    )?;
    let root = canonicalize(&root)?;
    if selected != root {
        return Err(GitWorkspaceError::NotRepositoryRoot { selected, root });
    }

    let git_dir = output_path(
        &root,
        "find repository git directory",
        &["rev-parse", "--absolute-git-dir"],
    )?;
    let common_dir = output_path(
        &root,
        "find repository common directory",
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )?;
    let git_dir = canonicalize(&git_dir)?;
    let common_dir = canonicalize(&common_dir)?;
    if git_dir != common_dir {
        return Err(GitWorkspaceError::LinkedWorktree {
            git_dir,
            common_dir,
        });
    }

    let worktrees = list_worktrees(&root)?;
    let primary_branch = resolve_primary_branch(&worktrees, &root)?;
    let remote = primary_remote(&root)?;
    let remote_url = output_string(
        &root,
        "read repository remote URL",
        &["remote", "get-url", &remote],
    )?;
    let default_branch = default_branch(&root, &remote)?;

    Ok(ValidatedRepository {
        root,
        primary_branch,
        remote,
        remote_url,
        default_branch,
    })
}

/// 在后台线程执行 repository 校验，避免在 UI 调用线程运行 blocking Git。
pub async fn validate_repository_async(
    path: PathBuf,
) -> Result<ValidatedRepository, GitWorkspaceError> {
    spawn_git_task("validate repository", move || validate_repository(&path)).await
}

/// Clone repository 到明确目标路径；目标为 `None` 时使用 URL 中的 repository 名。
pub fn clone_repository(
    url: &str,
    target: Option<&Path>,
) -> Result<ValidatedRepository, GitWorkspaceError> {
    let derived_target;
    let target = match target {
        Some(target) => target,
        None => {
            derived_target = PathBuf::from(repository_name_from_url(url)?);
            &derived_target
        }
    };
    clone_to_target(url, target)
}

/// 在指定父目录 Clone repository，可选择覆盖 URL 推导出的目录名。
pub fn clone_repository_into(
    url: &str,
    parent: &Path,
    directory_name: Option<&str>,
) -> Result<ValidatedRepository, GitWorkspaceError> {
    let directory_name = match directory_name {
        Some(directory_name) => directory_name.to_string(),
        None => repository_name_from_url(url)?,
    };
    validate_clone_directory_name(&directory_name)?;
    clone_to_target(url, &parent.join(directory_name))
}

/// 在后台线程 Clone repository，避免长时间 Git 操作阻塞 UI。
pub async fn clone_repository_async(
    url: String,
    target: Option<PathBuf>,
) -> Result<ValidatedRepository, GitWorkspaceError> {
    spawn_git_task("clone repository", move || {
        clone_repository(&url, target.as_deref())
    })
    .await
}

/// 执行 fetch 后列出本地与远端完整分支引用。
pub fn fetch_and_list_refs(repo: &Path) -> Result<Vec<BranchRef>, GitWorkspaceError> {
    let remote = primary_remote(repo)?;
    git_output_for_operation(
        repo,
        "fetch repository refs",
        &["fetch", "--prune", "--quiet", "--no-tags", &remote],
    )?;
    list_branch_refs(repo)
}

/// 在后台线程 fetch 并列出分支引用，避免阻塞 UI。
pub async fn fetch_and_list_refs_async(repo: PathBuf) -> Result<Vec<BranchRef>, GitWorkspaceError> {
    spawn_git_task("fetch repository refs", move || fetch_and_list_refs(&repo)).await
}

/// 使用完整 refname 列出本地与远端分支。
pub fn list_branch_refs(repo: &Path) -> Result<Vec<BranchRef>, GitWorkspaceError> {
    let remotes = list_remotes(repo)?;
    let output = git_output_for_operation(
        repo,
        "list repository refs",
        &[
            "for-each-ref",
            "--format=%(refname)%09%(symref)",
            "refs/heads",
            "refs/remotes",
        ],
    )?;
    let stdout = String::from_utf8(output.stdout).map_err(|_| GitWorkspaceError::InvalidUtf8 {
        operation: "list repository refs",
    })?;

    parse_branch_ref_records(&stdout, &remotes)
}

pub(crate) fn parse_branch_ref_records(
    stdout: &str,
    remotes: &[String],
) -> Result<Vec<BranchRef>, GitWorkspaceError> {
    let mut refs = Vec::new();
    for record in stdout.lines() {
        let Some((full_ref, symref)) = record.split_once('\t') else {
            return Err(GitWorkspaceError::InvalidBranchRefRecord {
                record: record.to_string(),
            });
        };
        if full_ref.is_empty() || symref.contains('\t') {
            return Err(GitWorkspaceError::InvalidBranchRefRecord {
                record: record.to_string(),
            });
        }
        if !symref.is_empty() {
            continue;
        }
        refs.push(parse_branch_ref(full_ref, remotes)?);
    }

    Ok(refs)
}

/// 在后台线程列出分支引用，避免在 UI 调用线程运行 blocking Git。
pub async fn list_branch_refs_async(repo: PathBuf) -> Result<Vec<BranchRef>, GitWorkspaceError> {
    spawn_git_task("list repository refs", move || list_branch_refs(&repo)).await
}

/// 解析 remote HEAD，返回带完整 refname 的默认分支。
pub fn default_branch(repo: &Path, remote: &str) -> Result<BranchRef, GitWorkspaceError> {
    let symbolic_ref = format!("refs/remotes/{remote}/HEAD");
    let output = git_output_for_operation(
        repo,
        "read remote default branch",
        &["symbolic-ref", &symbolic_ref],
    );
    let full_ref = match output {
        Ok(output) => decode_stdout(output, "read remote default branch")?,
        Err(GitWorkspaceError::CommandFailed { stderr, .. }) => {
            return Err(GitWorkspaceError::DefaultBranchNotFound {
                repo: repo.to_path_buf(),
                remote: remote.to_string(),
                stderr,
            });
        }
        Err(error) => return Err(error),
    };
    let prefix = format!("refs/remotes/{remote}/");
    let Some(name) = full_ref.strip_prefix(&prefix) else {
        return Err(GitWorkspaceError::DefaultBranchNotFound {
            repo: repo.to_path_buf(),
            remote: remote.to_string(),
            stderr: format!("unexpected symbolic ref `{full_ref}`"),
        });
    };
    if name.is_empty() || name == "HEAD" {
        return Err(GitWorkspaceError::DefaultBranchNotFound {
            repo: repo.to_path_buf(),
            remote: remote.to_string(),
            stderr: format!("unexpected symbolic ref `{full_ref}`"),
        });
    }

    Ok(BranchRef::Remote {
        remote: remote.to_string(),
        name: name.to_string(),
        full_ref,
    })
}

/// 解析 `git worktree list --porcelain`，保留路径和完整 branch ref。
pub fn list_worktrees(repo: &Path) -> Result<Vec<WorktreeInfo>, GitWorkspaceError> {
    let output = git_output_for_operation(
        repo,
        "list repository worktrees",
        &["worktree", "list", "--porcelain", "-z"],
    )?;
    parse_worktrees(&output.stdout)
}

/// 在后台线程列出 worktree，避免在 UI 调用线程运行 blocking Git。
pub async fn list_worktrees_async(repo: PathBuf) -> Result<Vec<WorktreeInfo>, GitWorkspaceError> {
    spawn_git_task("list repository worktrees", move || list_worktrees(&repo)).await
}

/// 将已注册 worktree 转换为可接入的 repository workspace 候选项。
pub fn existing_worktree_options(
    repository_root: &Path,
    worktrees: impl IntoIterator<Item = WorktreeInfo>,
) -> Vec<ExistingWorktreeOption> {
    let mut options = worktrees
        .into_iter()
        .filter_map(|worktree| {
            let branch_name = worktree.branch.as_deref()?.strip_prefix("refs/heads/")?;
            let is_primary = is_primary_worktree_path(repository_root, &worktree.path);
            (!worktree.is_bare
                && !worktree.is_detached
                && !worktree.is_prunable
                && !branch_name.is_empty())
            .then(|| {
                if is_primary {
                    ExistingWorktreeOption::primary(worktree.path, branch_name)
                } else {
                    ExistingWorktreeOption::new(worktree.path, branch_name)
                }
            })
        })
        .collect::<Vec<_>>();
    options.sort_by(|left, right| {
        right
            .is_primary
            .cmp(&left.is_primary)
            .then_with(|| left.branch_name.cmp(&right.branch_name))
            .then_with(|| left.path.cmp(&right.path))
    });
    options
}

/// 判断 worktree 路径是否指向 repository 主工作目录。
pub(crate) fn is_primary_worktree_path(repository_root: &Path, worktree_path: &Path) -> bool {
    if repository_root == worktree_path {
        return true;
    }

    match (
        dunce::canonicalize(repository_root),
        dunce::canonicalize(worktree_path),
    ) {
        (Ok(repository_root), Ok(worktree_path)) => repository_root == worktree_path,
        _ => false,
    }
}

fn resolve_primary_branch(
    worktrees: &[WorktreeInfo],
    repository_root: &Path,
) -> Result<String, GitWorkspaceError> {
    let Some(primary_worktree) = worktrees
        .iter()
        .find(|worktree| worktree.path == repository_root)
    else {
        return Err(GitWorkspaceError::PrimaryWorktreeDetached {
            path: repository_root.to_path_buf(),
        });
    };
    if primary_worktree.is_bare || primary_worktree.is_detached {
        return Err(GitWorkspaceError::PrimaryWorktreeDetached {
            path: repository_root.to_path_buf(),
        });
    }
    let Some(branch_name) = primary_worktree
        .branch
        .as_deref()
        .and_then(|branch| branch.strip_prefix("refs/heads/"))
        .filter(|branch| !branch.is_empty())
    else {
        return Err(GitWorkspaceError::PrimaryWorktreeDetached {
            path: repository_root.to_path_buf(),
        });
    };
    Ok(branch_name.to_string())
}

/// 校验已注册 worktree 在接入 workspace 前仍存在且检出预期本地分支。
pub fn validate_existing_worktree(
    repository: &Path,
    worktree_path: &Path,
    local_branch: &str,
) -> Result<PathBuf, GitWorkspaceError> {
    let registered_path = canonicalize(worktree_path)?;
    let repository_root = canonicalize(repository)?;
    let is_primary = registered_path == repository_root;

    let mut matches = list_worktrees(repository)?
        .into_iter()
        .filter(|worktree| worktree.path == registered_path);
    let Some(worktree) = matches.next() else {
        return Err(GitWorkspaceError::WorktreeNotFound {
            path: registered_path,
        });
    };
    if matches.next().is_some() {
        return Err(GitWorkspaceError::AmbiguousWorktree {
            path: registered_path,
        });
    }

    let expected_branch = format!("refs/heads/{local_branch}");
    if worktree.is_prunable {
        return Err(GitWorkspaceError::PrunableWorktreeCannotBeWorkspace {
            path: registered_path,
        });
    }
    if is_primary {
        resolve_primary_branch(std::slice::from_ref(&worktree), &registered_path)?;
    }
    if worktree.is_bare
        || worktree.is_detached
        || worktree.branch.as_deref() != Some(&expected_branch)
    {
        return Err(GitWorkspaceError::WorktreeBranchMismatch {
            expected: expected_branch,
            actual: worktree
                .branch
                .unwrap_or_else(|| "<detached or missing>".to_string()),
        });
    }

    validate_ref_exists(repository, &format!("refs/heads/{local_branch}"))?;
    Ok(registered_path)
}

/// 在后台线程校验已注册 linked worktree，避免阻塞 UI 调用线程。
pub async fn validate_existing_worktree_async(
    repository: PathBuf,
    worktree_path: PathBuf,
    local_branch: String,
) -> Result<PathBuf, GitWorkspaceError> {
    spawn_git_task("validate existing worktree", move || {
        validate_existing_worktree(&repository, &worktree_path, &local_branch)
    })
    .await
}

/// 从完整 remote ref 创建不跟踪 upstream 的新本地分支和 linked worktree。
pub fn create_from_remote(
    repository: &Path,
    remote_ref: &str,
    new_branch: &str,
    worktree_path: &Path,
) -> Result<(), GitWorkspaceError> {
    create_from_remote_core(
        repository,
        remote_ref,
        new_branch,
        worktree_path,
        || {},
        create_remote_worktree,
    )
}

#[cfg(test)]
pub(crate) fn create_from_remote_with_after_target_claim_hook<F>(
    repository: &Path,
    remote_ref: &str,
    new_branch: &str,
    worktree_path: &Path,
    after: F,
) -> Result<(), GitWorkspaceError>
where
    F: FnOnce(),
{
    create_from_remote_core(
        repository,
        remote_ref,
        new_branch,
        worktree_path,
        after,
        create_remote_worktree,
    )
}

#[cfg(test)]
pub(crate) fn create_from_remote_with_runner<Runner>(
    repository: &Path,
    remote_ref: &str,
    new_branch: &str,
    worktree_path: &Path,
    runner: Runner,
) -> Result<(), GitWorkspaceError>
where
    Runner: FnOnce(&Path, &str, &str, &Path, &str) -> Result<(), GitWorkspaceError>,
{
    create_from_remote_core(
        repository,
        remote_ref,
        new_branch,
        worktree_path,
        || {},
        runner,
    )
}

fn create_from_remote_core<AfterTargetClaim, Runner>(
    repository: &Path,
    remote_ref: &str,
    new_branch: &str,
    worktree_path: &Path,
    after_target_claim: AfterTargetClaim,
    runner: Runner,
) -> Result<(), GitWorkspaceError>
where
    AfterTargetClaim: FnOnce(),
    Runner: FnOnce(&Path, &str, &str, &Path, &str) -> Result<(), GitWorkspaceError>,
{
    let expected_oid = validate_remote_ref(repository, remote_ref)?;
    validate_new_branch(repository, new_branch)?;
    let claim = TargetDirectoryClaim::acquire(worktree_path)?;
    after_target_claim();
    claim.ensure_directory()?;

    match runner(
        repository,
        remote_ref,
        new_branch,
        &claim.canonical_path,
        &expected_oid,
    ) {
        Ok(()) => {
            claim.ensure_directory()?;
            verify_remote_worktree_creation(
                repository,
                &claim.canonical_path,
                new_branch,
                &expected_oid,
            )
            .map_err(|verification_error| {
                GitWorkspaceError::WorktreeCreationVerificationFailed {
                    worktree_path: claim.requested_path,
                    branch: new_branch.to_string(),
                    expected_oid,
                    verification_error: Box::new(verification_error),
                }
            })
        }
        Err(create_error) => Err(worktree_creation_failed(
            repository,
            &claim,
            new_branch,
            true,
            create_error,
        )),
    }
}

fn create_remote_worktree(
    repository: &Path,
    remote_ref: &str,
    new_branch: &str,
    claimed_path: &Path,
    _: &str,
) -> Result<(), GitWorkspaceError> {
    let args = [
        OsStr::new("worktree"),
        OsStr::new("add"),
        OsStr::new("--no-track"),
        OsStr::new("-b"),
        OsStr::new(new_branch),
        claimed_path.as_os_str(),
        OsStr::new(remote_ref),
    ];
    git_output_with_os_args_for_operation(repository, "create worktree from remote", &args)?;
    Ok(())
}

fn verify_remote_worktree_creation(
    repository: &Path,
    worktree_path: &Path,
    branch: &str,
    expected_oid: &str,
) -> Result<(), GitWorkspaceError> {
    let registered_path = canonicalize(worktree_path)?;
    let mut matches = list_worktrees(repository)?
        .into_iter()
        .filter(|worktree| worktree.path == registered_path);
    let Some(worktree) = matches.next() else {
        return Err(GitWorkspaceError::WorktreeNotFound {
            path: registered_path,
        });
    };
    if matches.next().is_some() {
        return Err(GitWorkspaceError::AmbiguousWorktree {
            path: registered_path,
        });
    }

    let expected_branch = format!("refs/heads/{branch}");
    if worktree.is_bare
        || worktree.is_detached
        || worktree.branch.as_deref() != Some(&expected_branch)
    {
        return Err(GitWorkspaceError::WorktreeBranchMismatch {
            expected: expected_branch,
            actual: worktree
                .branch
                .unwrap_or_else(|| "<detached or missing>".to_string()),
        });
    }

    let actual_snapshot = direct_ref_snapshot(repository, &expected_branch)?;
    if actual_snapshot.direct_oid.as_deref() != Some(expected_oid)
        || actual_snapshot.symbolic_target.is_some()
    {
        return Err(GitWorkspaceError::BranchChanged {
            branch: branch.to_string(),
            expected_oid: expected_oid.to_string(),
            actual_oid: actual_snapshot.direct_oid,
            actual_symbolic_target: actual_snapshot.symbolic_target,
        });
    }
    if let Some(upstream) = branch_upstream(repository, &expected_branch)? {
        return Err(GitWorkspaceError::UnexpectedBranchUpstream {
            branch: branch.to_string(),
            upstream,
        });
    }
    Ok(())
}

/// 在后台线程从 remote ref 创建 linked worktree，避免阻塞 UI。
pub async fn create_from_remote_async(
    repository: PathBuf,
    remote_ref: String,
    new_branch: String,
    worktree_path: PathBuf,
) -> Result<(), GitWorkspaceError> {
    spawn_git_task("create worktree from remote", move || {
        create_from_remote(&repository, &remote_ref, &new_branch, &worktree_path)
    })
    .await
}

/// 从现有本地分支创建 linked worktree。
pub fn create_from_local(
    repository: &Path,
    local_branch: &str,
    worktree_path: &Path,
) -> Result<(), GitWorkspaceError> {
    create_from_local_core(
        repository,
        local_branch,
        worktree_path,
        || {},
        create_local_worktree,
    )
}

#[cfg(test)]
pub(crate) fn create_from_local_with_after_target_claim_hook<F>(
    repository: &Path,
    branch: &str,
    path: &Path,
    after: F,
) -> Result<(), GitWorkspaceError>
where
    F: FnOnce(),
{
    create_from_local_core(repository, branch, path, after, create_local_worktree)
}

#[cfg(test)]
pub(crate) fn create_from_local_with_runner<Runner>(
    repository: &Path,
    branch: &str,
    path: &Path,
    runner: Runner,
) -> Result<(), GitWorkspaceError>
where
    Runner: FnOnce(&Path, &str, &Path) -> Result<(), GitWorkspaceError>,
{
    create_from_local_core(repository, branch, path, || {}, runner)
}

fn create_from_local_core<AfterTargetClaim, Runner>(
    repository: &Path,
    local_branch: &str,
    worktree_path: &Path,
    after_target_claim: AfterTargetClaim,
    runner: Runner,
) -> Result<(), GitWorkspaceError>
where
    AfterTargetClaim: FnOnce(),
    Runner: FnOnce(&Path, &str, &Path) -> Result<(), GitWorkspaceError>,
{
    let full_ref = format!("refs/heads/{local_branch}");
    validate_ref_exists(repository, &full_ref)?;
    if let Some(worktree) = list_worktrees(repository)?
        .into_iter()
        .find(|worktree| worktree.branch.as_deref() == Some(&full_ref))
    {
        return Err(GitWorkspaceError::BranchAlreadyCheckedOut {
            branch: local_branch.to_string(),
            path: worktree.path,
        });
    }
    let claim = TargetDirectoryClaim::acquire(worktree_path)?;
    after_target_claim();
    claim.ensure_directory()?;

    match runner(repository, local_branch, &claim.canonical_path) {
        Ok(()) => claim.ensure_directory(),
        Err(create_error) => Err(worktree_creation_failed(
            repository,
            &claim,
            local_branch,
            false,
            create_error,
        )),
    }
}

fn create_local_worktree(
    repository: &Path,
    local_branch: &str,
    claimed_path: &Path,
) -> Result<(), GitWorkspaceError> {
    let args = [
        OsStr::new("worktree"),
        OsStr::new("add"),
        claimed_path.as_os_str(),
        OsStr::new(local_branch),
    ];
    git_output_with_os_args_for_operation(repository, "create worktree from local branch", &args)?;
    Ok(())
}

/// 在后台线程从本地分支创建 linked worktree，避免阻塞 UI。
pub async fn create_from_local_async(
    repository: PathBuf,
    local_branch: String,
    worktree_path: PathBuf,
) -> Result<(), GitWorkspaceError> {
    spawn_git_task("create worktree from local branch", move || {
        create_from_local(&repository, &local_branch, &worktree_path)
    })
    .await
}

/// 只读校验 linked worktree 是否可删除，并捕获供 UI 使用的建议快照。
pub fn deletion_preflight(
    repository: &Path,
    worktree_path: &Path,
    delete_branch: bool,
) -> Result<DeletionPreflight, GitWorkspaceError> {
    capture_deletion_candidate(repository, worktree_path, delete_branch)
}

fn capture_deletion_candidate(
    repository: &Path,
    worktree_path: &Path,
    include_merge_target: bool,
) -> Result<DeletionPreflight, GitWorkspaceError> {
    let worktree_path = canonicalize(worktree_path)?;
    let mut matches = list_worktrees(repository)?
        .into_iter()
        .filter(|worktree| worktree.path == worktree_path);
    let Some(worktree) = matches.next() else {
        return Err(GitWorkspaceError::WorktreeNotFound {
            path: worktree_path,
        });
    };
    if matches.next().is_some() {
        return Err(GitWorkspaceError::AmbiguousWorktree {
            path: worktree_path,
        });
    }
    if worktree.is_bare || worktree.is_detached {
        return Err(GitWorkspaceError::WorktreeHasNoLocalBranch {
            path: worktree_path,
        });
    }
    let Some(full_ref) = worktree.branch else {
        return Err(GitWorkspaceError::WorktreeHasNoLocalBranch {
            path: worktree_path,
        });
    };
    let Some(branch) = full_ref.strip_prefix("refs/heads/") else {
        return Err(GitWorkspaceError::WorktreeHasNoLocalBranch {
            path: worktree_path,
        });
    };
    if branch.is_empty() {
        return Err(GitWorkspaceError::WorktreeHasNoLocalBranch {
            path: worktree_path,
        });
    }
    let branch_snapshot = direct_ref_snapshot(repository, &full_ref)?;
    if branch_snapshot.symbolic_target.is_some() {
        return Err(GitWorkspaceError::WorktreeHasNoLocalBranch {
            path: worktree_path,
        });
    }
    let Some(branch_oid) = branch_snapshot.direct_oid else {
        return Err(GitWorkspaceError::BranchNotFound { full_ref });
    };

    let status = git_output_for_operation(
        &worktree_path,
        "check worktree status",
        &["status", "--porcelain", "--untracked-files=all"],
    )?;
    if !status.stdout.is_empty() {
        return Err(GitWorkspaceError::DirtyWorktree {
            path: worktree_path,
        });
    }

    if !include_merge_target {
        return Ok(DeletionPreflight {
            worktree_path,
            branch: branch.to_string(),
            branch_ref: full_ref,
            branch_oid,
            merge_target: None,
        });
    }

    let merge_target = resolve_merge_target_ref(repository, &full_ref)?;
    if merge_target == full_ref {
        return Err(GitWorkspaceError::InvalidMergeTarget {
            branch_ref: full_ref,
            target_ref: merge_target,
        });
    }
    let target_oid = resolve_commit_oid(repository, &merge_target)?;
    let is_merged = is_ancestor(repository, &branch_oid, &target_oid)?;

    Ok(DeletionPreflight {
        worktree_path,
        branch: branch.to_string(),
        branch_ref: full_ref,
        branch_oid,
        merge_target: Some(MergeTargetSnapshot {
            full_ref: merge_target,
            oid: target_oid,
            is_merged,
        }),
    })
}

fn resolve_merge_target_ref(
    repository: &Path,
    branch_ref: &str,
) -> Result<String, GitWorkspaceError> {
    if let Some(upstream) = branch_upstream(repository, branch_ref)? {
        if ref_exists(repository, &upstream)? {
            return Ok(upstream);
        }
    }
    let remote = primary_remote(repository)?;
    match default_branch(repository, &remote)? {
        BranchRef::Remote { full_ref, .. } => Ok(full_ref),
        BranchRef::Local { full_ref, .. } => Err(GitWorkspaceError::InvalidRemoteRef { full_ref }),
    }
}

fn is_ancestor(
    repository: &Path,
    branch_oid: &str,
    target_oid: &str,
) -> Result<bool, GitWorkspaceError> {
    let args = ["merge-base", "--is-ancestor", branch_oid, target_oid];
    let output =
        git_output_allow_failure_for_operation(repository, "check branch merge status", &args)?;
    match output.status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        Some(exit_code) => {
            debug_assert_ne!(exit_code, 0);
            debug_assert_ne!(exit_code, 1);
            return Err(command_failed("check branch merge status", &args, &output));
        }
        None => {
            return Err(command_failed("check branch merge status", &args, &output));
        }
    }
}

/// 在后台线程执行 linked worktree 删除预检，避免阻塞 UI。
pub async fn deletion_preflight_async(
    repository: PathBuf,
    worktree_path: PathBuf,
    delete_branch: bool,
) -> Result<DeletionPreflight, GitWorkspaceError> {
    spawn_git_task("preflight worktree deletion", move || {
        deletion_preflight(&repository, &worktree_path, delete_branch)
    })
    .await
}

/// 删除已通过完整预检的 linked worktree，并按需删除对应本地分支。
pub fn remove_workspace(
    repository: &Path,
    worktree_path: &Path,
    branch: &str,
    delete_branch: bool,
    force_branch: bool,
) -> Result<(), GitWorkspaceError> {
    remove_workspace_inner(
        repository,
        worktree_path,
        branch,
        delete_branch,
        force_branch,
        || {},
        || {},
        remove_registered_worktree,
        |_| {},
    )
}

#[cfg(test)]
pub(crate) fn remove_workspace_with_transaction_hooks<AfterCandidate, AfterPrepared>(
    repository: &Path,
    worktree_path: &Path,
    branch: &str,
    delete_branch: bool,
    force_branch: bool,
    after_candidate: AfterCandidate,
    after_prepared: AfterPrepared,
) -> Result<(), GitWorkspaceError>
where
    AfterCandidate: FnOnce(),
    AfterPrepared: FnOnce(),
{
    remove_workspace_inner(
        repository,
        worktree_path,
        branch,
        delete_branch,
        force_branch,
        after_candidate,
        after_prepared,
        remove_registered_worktree,
        |_| {},
    )
}

#[cfg(test)]
pub(crate) fn remove_workspace_with_hook<AfterCandidate>(
    repository: &Path,
    worktree_path: &Path,
    branch: &str,
    delete_branch: bool,
    force_branch: bool,
    after_candidate: AfterCandidate,
) -> Result<(), GitWorkspaceError>
where
    AfterCandidate: FnOnce(),
{
    remove_workspace_inner(
        repository,
        worktree_path,
        branch,
        delete_branch,
        force_branch,
        after_candidate,
        || {},
        remove_registered_worktree,
        |_| {},
    )
}

#[cfg(test)]
pub(crate) fn remove_workspace_with_remove_runner<RemoveRunner>(
    repository: &Path,
    worktree_path: &Path,
    branch: &str,
    delete_branch: bool,
    force_branch: bool,
    remove_runner: RemoveRunner,
) -> Result<(), GitWorkspaceError>
where
    RemoveRunner: FnOnce(&Path, &Path) -> Result<(), GitWorkspaceError>,
{
    remove_workspace_inner(
        repository,
        worktree_path,
        branch,
        delete_branch,
        force_branch,
        || {},
        || {},
        remove_runner,
        |_| {},
    )
}

#[cfg(test)]
pub(crate) fn remove_workspace_with_after_remove_hook<AfterRemove>(
    repository: &Path,
    worktree_path: &Path,
    branch: &str,
    delete_branch: bool,
    force_branch: bool,
    after_remove: AfterRemove,
) -> Result<(), GitWorkspaceError>
where
    AfterRemove: FnOnce(&mut PreparedRefDelete),
{
    remove_workspace_inner(
        repository,
        worktree_path,
        branch,
        delete_branch,
        force_branch,
        || {},
        || {},
        remove_registered_worktree,
        after_remove,
    )
}

fn remove_workspace_inner<AfterCandidate, AfterPrepared, RemoveRunner, AfterRemove>(
    repository: &Path,
    worktree_path: &Path,
    branch: &str,
    delete_branch: bool,
    force_branch: bool,
    after_candidate: AfterCandidate,
    after_prepared: AfterPrepared,
    remove_runner: RemoveRunner,
    after_remove: AfterRemove,
) -> Result<(), GitWorkspaceError>
where
    AfterCandidate: FnOnce(),
    AfterPrepared: FnOnce(),
    RemoveRunner: FnOnce(&Path, &Path) -> Result<(), GitWorkspaceError>,
    AfterRemove: FnOnce(&mut PreparedRefDelete),
{
    let candidate =
        capture_deletion_candidate(repository, worktree_path, delete_branch && !force_branch)?;
    validate_requested_branch(branch, &candidate)?;
    if !delete_branch {
        return remove_registered_worktree(repository, &candidate.worktree_path);
    }
    after_candidate();
    let target = if force_branch {
        None
    } else {
        let target = candidate.merge_target.as_ref().ok_or_else(|| {
            GitWorkspaceError::MissingMergeTarget {
                branch_ref: candidate.branch_ref.clone(),
            }
        })?;
        Some(LockedRef {
            full_ref: &target.full_ref,
            oid: &target.oid,
        })
    };
    let mut transaction = PreparedRefDelete::prepare(
        repository,
        &candidate.branch_ref,
        &candidate.branch_oid,
        target,
    )
    .map_err(|source| GitWorkspaceError::RefTransaction { source })?;
    after_prepared();
    let registered_path = match validate_locked_deletion(
        repository,
        worktree_path,
        branch,
        &candidate.branch_oid,
        candidate.merge_target.as_ref(),
        force_branch,
    ) {
        Ok(path) => path,
        Err(error) => return abort_after_failed_operation(transaction, error),
    };
    if let Err(error) = remove_runner(repository, &registered_path) {
        return abort_after_failed_operation(transaction, error);
    }
    after_remove(&mut transaction);
    if let Err(source) = transaction.commit() {
        let branch_ref = candidate.branch_ref.clone();
        let branch_oid = candidate.branch_oid.clone();
        let merge_target_ref = candidate
            .merge_target
            .as_ref()
            .map(|target| target.full_ref.clone());
        let merge_target_oid = candidate
            .merge_target
            .as_ref()
            .map(|target| target.oid.clone());
        let (inspection_error, actual_snapshot) = match direct_ref_snapshot(repository, &branch_ref)
        {
            Ok(snapshot) => (None, Some(snapshot)),
            Err(error) => (Some(Box::new(error)), None),
        };
        return Err(GitWorkspaceError::BranchDeleteTransactionFailed {
            worktree_path: registered_path,
            worktree_removed: true,
            branch_ref,
            branch_oid: branch_oid.clone(),
            merge_target_ref,
            merge_target_oid,
            source,
            inspection_error: inspection_error.or_else(|| {
                actual_snapshot.and_then(|snapshot| {
                    (snapshot.direct_oid.is_some() || snapshot.symbolic_target.is_some()).then(
                        || {
                            Box::new(GitWorkspaceError::BranchChanged {
                                branch: branch.to_string(),
                                expected_oid: branch_oid,
                                actual_oid: snapshot.direct_oid,
                                actual_symbolic_target: snapshot.symbolic_target,
                            })
                        },
                    )
                })
            }),
        });
    }
    Ok(())
}

fn validate_requested_branch(
    branch: &str,
    candidate: &DeletionPreflight,
) -> Result<(), GitWorkspaceError> {
    if candidate.branch == branch {
        Ok(())
    } else {
        Err(GitWorkspaceError::WorktreeBranchMismatch {
            expected: branch.to_string(),
            actual: candidate.branch.clone(),
        })
    }
}

fn remove_registered_worktree(repository: &Path, path: &Path) -> Result<(), GitWorkspaceError> {
    let args = [
        OsStr::new("worktree"),
        OsStr::new("remove"),
        path.as_os_str(),
    ];
    git_output_with_os_args_for_operation(repository, "remove worktree", &args)?;
    Ok(())
}

fn validate_locked_deletion(
    repository: &Path,
    path: &Path,
    branch: &str,
    expected_oid: &str,
    expected_target: Option<&MergeTargetSnapshot>,
    force: bool,
) -> Result<PathBuf, GitWorkspaceError> {
    let locked = capture_deletion_candidate(repository, path, !force)?;
    validate_requested_branch(branch, &locked)?;
    if locked.branch_oid != expected_oid {
        let snapshot = direct_ref_snapshot(repository, &locked.branch_ref)?;
        return Err(GitWorkspaceError::BranchChanged {
            branch: locked.branch,
            expected_oid: expected_oid.to_string(),
            actual_oid: snapshot.direct_oid,
            actual_symbolic_target: snapshot.symbolic_target,
        });
    }
    if !force {
        let expected = expected_target.ok_or_else(|| GitWorkspaceError::MissingMergeTarget {
            branch_ref: locked.branch_ref.clone(),
        })?;
        let actual = locked
            .merge_target
            .ok_or_else(|| GitWorkspaceError::MissingMergeTarget {
                branch_ref: locked.branch_ref.clone(),
            })?;
        if actual.full_ref != expected.full_ref {
            return Err(GitWorkspaceError::MergeTargetChanged {
                expected: expected.full_ref.clone(),
                actual: actual.full_ref,
            });
        }
        if actual.oid != expected.oid {
            return Err(GitWorkspaceError::RefChanged {
                full_ref: expected.full_ref.clone(),
                expected_oid: expected.oid.clone(),
                actual_oid: actual.oid,
            });
        }
        if !actual.is_merged {
            return Err(GitWorkspaceError::BranchNotMerged {
                branch: locked.branch,
                merge_target: expected.full_ref.clone(),
            });
        }
    }
    Ok(locked.worktree_path)
}

fn abort_after_failed_operation(
    transaction: PreparedRefDelete,
    operation_error: GitWorkspaceError,
) -> Result<(), GitWorkspaceError> {
    match transaction.abort() {
        Ok(()) => Err(operation_error),
        Err(abort_error) => Err(GitWorkspaceError::BranchDeleteAbortFailed {
            operation_error: Box::new(operation_error),
            abort_error,
        }),
    }
}

/// 在后台线程删除 linked worktree 和可选本地分支，避免阻塞 UI。
pub async fn remove_workspace_async(
    repository: PathBuf,
    worktree_path: PathBuf,
    branch: String,
    delete_branch: bool,
    force_branch: bool,
) -> Result<(), GitWorkspaceError> {
    spawn_git_task("remove worktree", move || {
        remove_workspace(
            &repository,
            &worktree_path,
            &branch,
            delete_branch,
            force_branch,
        )
    })
    .await
}

/// 从标准 Git URL、SCP 风格地址或本地路径解析 repository 名。
pub fn repository_name_from_url(url: &str) -> Result<String, GitWorkspaceError> {
    let trimmed = url.trim();
    let name = if is_windows_drive_absolute(trimmed) {
        repository_name_from_local_path(trimmed)
    } else {
        match url::Url::parse(trimmed) {
            Ok(parsed) => parsed
                .path_segments()
                .and_then(|segments| segments.filter(|segment| !segment.is_empty()).next_back())
                .map(str::to_string),
            Err(_) => repository_name_from_local_path(trimmed),
        }
    }
    .map(|name| name.strip_suffix(".git").unwrap_or(&name).to_string())
    .filter(|name| !name.is_empty());

    name.ok_or_else(|| GitWorkspaceError::RepositoryNameMissing {
        url: url.to_string(),
    })
}

fn is_windows_drive_absolute(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'/' | b'\\')
}

fn repository_name_from_local_path(path: &str) -> Option<String> {
    path.trim_end_matches(['/', '\\'])
        .rsplit(['/', '\\', ':'])
        .find(|segment| !segment.is_empty())
        .map(str::to_string)
}

fn validate_clone_directory_name(name: &str) -> Result<(), GitWorkspaceError> {
    let mut components = Path::new(name).components();
    let valid = matches!(
        (components.next(), components.next()),
        (Some(Component::Normal(component)), None) if component == OsStr::new(name)
    );
    if valid {
        Ok(())
    } else {
        Err(GitWorkspaceError::InvalidCloneDirectoryName {
            name: name.to_string(),
        })
    }
}

/// 将分支名转换为安全目录 slug，并追加 workspace ID 的前 8 位。
pub fn workspace_dir_name(branch: &str, workspace_id: &str) -> String {
    let mut slug = String::new();
    let mut last_was_separator = false;
    for character in branch.chars() {
        let safe = character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.');
        if safe {
            slug.push(character);
            last_was_separator = false;
        } else if !slug.is_empty() && !last_was_separator {
            slug.push('-');
            last_was_separator = true;
        }
    }
    let slug = slug.trim_matches('-');
    let slug = if slug.is_empty() { "workspace" } else { slug };
    let short_id: String = workspace_id.chars().take(8).collect();
    if short_id.is_empty() {
        slug.to_string()
    } else {
        format!("{slug}-{short_id}")
    }
}

fn clone_to_target(url: &str, target: &Path) -> Result<ValidatedRepository, GitWorkspaceError> {
    if target.exists() {
        return Err(GitWorkspaceError::TargetExists {
            path: target.to_path_buf(),
        });
    }
    std::fs::create_dir(target).map_err(|source| GitWorkspaceError::CreateTarget {
        path: target.to_path_buf(),
        source,
    })?;

    let result = git_output_for_operation(target, "clone repository", &["clone", "--", url, "."])
        .and_then(|_| validate_repository(target));
    match result {
        Ok(repository) => Ok(repository),
        Err(clone_error) => {
            if let Err(cleanup_source) = std::fs::remove_dir_all(target) {
                return Err(GitWorkspaceError::CleanupFailed {
                    path: target.to_path_buf(),
                    cleanup_source,
                    clone_error: Box::new(clone_error),
                });
            }
            Err(clone_error)
        }
    }
}

fn primary_remote(repo: &Path) -> Result<String, GitWorkspaceError> {
    let remotes = list_remotes(repo)?;
    remotes
        .iter()
        .find(|remote| remote.as_str() == "origin")
        .or_else(|| remotes.first())
        .cloned()
        .ok_or_else(|| GitWorkspaceError::RemoteNotFound {
            repo: repo.to_path_buf(),
        })
}

fn list_remotes(repo: &Path) -> Result<Vec<String>, GitWorkspaceError> {
    let stdout = output_string(repo, "list repository remotes", &["remote"])?;
    Ok(stdout
        .lines()
        .map(str::trim)
        .filter(|remote| !remote.is_empty())
        .map(str::to_string)
        .collect())
}

fn validate_remote_ref(repository: &Path, remote_ref: &str) -> Result<String, GitWorkspaceError> {
    let remotes = list_remotes(repository)?;
    match parse_branch_ref(remote_ref, &remotes) {
        Ok(BranchRef::Remote { name, .. }) if name != "HEAD" => {}
        Ok(BranchRef::Local { .. })
        | Ok(BranchRef::Remote { .. })
        | Err(GitWorkspaceError::InvalidBranchRef { .. })
        | Err(GitWorkspaceError::AmbiguousRemoteRef { .. }) => {
            return Err(GitWorkspaceError::InvalidRemoteRef {
                full_ref: remote_ref.to_string(),
            });
        }
        Err(error) => return Err(error),
    }
    validate_ref_exists(repository, remote_ref)?;

    let output = git_output_allow_failure_for_operation(
        repository,
        "check whether remote ref is symbolic",
        &["symbolic-ref", "--quiet", remote_ref],
    )?;
    match output.status.code() {
        Some(0) => Err(GitWorkspaceError::InvalidRemoteRef {
            full_ref: remote_ref.to_string(),
        }),
        Some(1) => resolve_commit_oid(repository, remote_ref),
        Some(exit_code) => {
            debug_assert_ne!(exit_code, 0);
            debug_assert_ne!(exit_code, 1);
            Err(command_failed(
                "check whether remote ref is symbolic",
                &["symbolic-ref", "--quiet", remote_ref],
                &output,
            ))
        }
        None => Err(command_failed(
            "check whether remote ref is symbolic",
            &["symbolic-ref", "--quiet", remote_ref],
            &output,
        )),
    }
}

fn validate_new_branch(repository: &Path, branch: &str) -> Result<(), GitWorkspaceError> {
    let output = git_output_allow_failure_for_operation(
        repository,
        "validate new branch name",
        &["check-ref-format", "--branch", branch],
    )?;
    if !output.status.success() {
        return Err(GitWorkspaceError::InvalidBranchName {
            branch: branch.to_string(),
        });
    }

    let full_ref = format!("refs/heads/{branch}");
    if ref_exists(repository, &full_ref)? {
        return Err(GitWorkspaceError::BranchAlreadyExists {
            branch: branch.to_string(),
        });
    }
    Ok(())
}

fn validate_ref_exists(repository: &Path, full_ref: &str) -> Result<(), GitWorkspaceError> {
    if ref_exists(repository, full_ref)? {
        Ok(())
    } else {
        Err(GitWorkspaceError::BranchNotFound {
            full_ref: full_ref.to_string(),
        })
    }
}

fn ref_exists(repository: &Path, full_ref: &str) -> Result<bool, GitWorkspaceError> {
    let args = ["show-ref", "--verify", "--quiet", full_ref];
    let output =
        git_output_allow_failure_for_operation(repository, "check branch ref existence", &args)?;
    match output.status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        Some(exit_code) => {
            debug_assert_ne!(exit_code, 0);
            debug_assert_ne!(exit_code, 1);
            Err(command_failed("check branch ref existence", &args, &output))
        }
        None => Err(command_failed("check branch ref existence", &args, &output)),
    }
}

struct TargetDirectoryClaim {
    requested_path: PathBuf,
    canonical_path: PathBuf,
}

impl TargetDirectoryClaim {
    fn acquire(path: &Path) -> Result<Self, GitWorkspaceError> {
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent).map_err(|source| {
                GitWorkspaceError::TargetClaimFailed {
                    path: path.to_path_buf(),
                    source,
                }
            })?;
        }
        std::fs::create_dir(path).map_err(|source| match source.kind() {
            io::ErrorKind::AlreadyExists => GitWorkspaceError::TargetExists {
                path: path.to_path_buf(),
            },
            _ => GitWorkspaceError::TargetClaimFailed {
                path: path.to_path_buf(),
                source,
            },
        })?;
        let canonical_path = canonicalize(path)?;
        Ok(Self {
            requested_path: path.to_path_buf(),
            canonical_path,
        })
    }

    fn ensure_directory(&self) -> Result<(), GitWorkspaceError> {
        let metadata = std::fs::symlink_metadata(&self.requested_path).map_err(|source| {
            GitWorkspaceError::ClaimedTargetInspection {
                path: self.requested_path.clone(),
                source,
            }
        })?;
        if metadata.file_type().is_dir() {
            Ok(())
        } else {
            Err(GitWorkspaceError::ClaimedTargetNotDirectory {
                path: self.requested_path.clone(),
            })
        }
    }
}

fn worktree_creation_failed(
    repository: &Path,
    claim: &TargetDirectoryClaim,
    branch: &str,
    branch_may_remain: bool,
    create_error: GitWorkspaceError,
) -> GitWorkspaceError {
    let (worktree_registered, mut cleanup_error) = match list_worktrees(repository) {
        Ok(worktrees) => (
            Some(
                worktrees
                    .into_iter()
                    .any(|worktree| worktree.path == claim.canonical_path),
            ),
            None,
        ),
        Err(error) => (None, Some(Box::new(error))),
    };
    let mut claimed_directory_removed = false;
    if worktree_registered == Some(false) {
        match remove_claimed_directory_if_empty(claim) {
            Ok(()) => claimed_directory_removed = true,
            Err(error) => cleanup_error = Some(Box::new(error)),
        }
    }

    GitWorkspaceError::WorktreeCreationFailed {
        worktree_path: claim.requested_path.clone(),
        branch: branch.to_string(),
        branch_may_remain,
        worktree_registered,
        claimed_directory_removed,
        create_error: Box::new(create_error),
        cleanup_error,
    }
}

fn remove_claimed_directory_if_empty(
    claim: &TargetDirectoryClaim,
) -> Result<(), GitWorkspaceError> {
    let mut entries = std::fs::read_dir(&claim.requested_path).map_err(|source| {
        GitWorkspaceError::ClaimedTargetCleanupFailed {
            path: claim.requested_path.clone(),
            source,
        }
    })?;
    match entries.next() {
        None => {}
        Some(Ok(_)) => {
            return Err(GitWorkspaceError::ClaimedTargetNotEmpty {
                path: claim.requested_path.clone(),
            });
        }
        Some(Err(source)) => {
            return Err(GitWorkspaceError::ClaimedTargetCleanupFailed {
                path: claim.requested_path.clone(),
                source,
            });
        }
    }
    std::fs::remove_dir(&claim.requested_path).map_err(|source| {
        GitWorkspaceError::ClaimedTargetCleanupFailed {
            path: claim.requested_path.clone(),
            source,
        }
    })
}

pub fn workspace_upstream_ref(
    worktree: &Path,
    branch: &str,
) -> Result<Option<String>, GitWorkspaceError> {
    let full_ref = format!("refs/heads/{branch}");
    branch_upstream(worktree, &full_ref)
}

fn branch_upstream(repository: &Path, full_ref: &str) -> Result<Option<String>, GitWorkspaceError> {
    let upstream = output_string(
        repository,
        "read branch upstream",
        &["for-each-ref", "--format=%(upstream)", full_ref],
    )?;
    if upstream.is_empty() {
        Ok(None)
    } else {
        Ok(Some(upstream))
    }
}

fn resolve_commit_oid(repository: &Path, full_ref: &str) -> Result<String, GitWorkspaceError> {
    let commit_ref = format!("{full_ref}^{{commit}}");
    output_string(
        repository,
        "resolve branch commit OID",
        &["rev-parse", "--verify", &commit_ref],
    )
}

#[derive(Debug, Default)]
struct DirectRefSnapshot {
    direct_oid: Option<String>,
    symbolic_target: Option<String>,
}

fn direct_ref_snapshot(
    repository: &Path,
    full_ref: &str,
) -> Result<DirectRefSnapshot, GitWorkspaceError> {
    if let Some(symbolic_target) = symbolic_ref_target(repository, full_ref)? {
        return Ok(DirectRefSnapshot {
            direct_oid: None,
            symbolic_target: Some(symbolic_target),
        });
    }
    let stdout = output_string(
        repository,
        "read direct branch ref",
        &[
            "for-each-ref",
            "--format=%(refname)%09%(objectname)",
            full_ref,
        ],
    )?;
    let mut direct_oid = None;
    for record in stdout.lines() {
        let mut fields = record.split('\t');
        let refname = fields.next();
        let objectname = fields.next();
        let trailing = fields.next();
        let (Some(refname), Some(objectname), None) = (refname, objectname, trailing) else {
            return Err(GitWorkspaceError::InvalidDirectRefRecord {
                record: record.to_string(),
            });
        };
        if refname != full_ref {
            continue;
        }
        if direct_oid.is_some() || objectname.is_empty() {
            return Err(GitWorkspaceError::InvalidDirectRefRecord {
                record: record.to_string(),
            });
        }
        direct_oid = Some(objectname.to_string());
    }
    if let Some(symbolic_target) = symbolic_ref_target(repository, full_ref)? {
        return Ok(DirectRefSnapshot {
            direct_oid: None,
            symbolic_target: Some(symbolic_target),
        });
    }
    Ok(DirectRefSnapshot {
        direct_oid,
        symbolic_target: None,
    })
}

fn symbolic_ref_target(
    repository: &Path,
    full_ref: &str,
) -> Result<Option<String>, GitWorkspaceError> {
    let args = ["symbolic-ref", "--quiet", full_ref];
    let output =
        git_output_allow_failure_for_operation(repository, "inspect branch symbolic ref", &args)?;
    match output.status.code() {
        Some(0) => decode_stdout(output, "inspect branch symbolic ref").map(Some),
        Some(1) => Ok(None),
        Some(exit_code) => {
            debug_assert_ne!(exit_code, 0);
            debug_assert_ne!(exit_code, 1);
            Err(command_failed(
                "inspect branch symbolic ref",
                &args,
                &output,
            ))
        }
        None => Err(command_failed(
            "inspect branch symbolic ref",
            &args,
            &output,
        )),
    }
}

pub(crate) fn parse_branch_ref(
    full_ref: &str,
    remotes: &[String],
) -> Result<BranchRef, GitWorkspaceError> {
    if let Some(name) = full_ref.strip_prefix("refs/heads/") {
        if !name.is_empty() {
            return Ok(BranchRef::Local {
                name: name.to_string(),
                full_ref: full_ref.to_string(),
            });
        }
    }

    if let Some(remote_ref) = full_ref.strip_prefix("refs/remotes/") {
        let mut matches = Vec::new();
        for remote in remotes {
            let prefix = format!("{remote}/");
            if let Some(name) = remote_ref.strip_prefix(&prefix) {
                if !name.is_empty() {
                    matches.push((remote.clone(), name.to_string()));
                }
            }
        }
        return match matches.as_slice() {
            [(remote, name)] => Ok(BranchRef::Remote {
                remote: remote.clone(),
                name: name.clone(),
                full_ref: full_ref.to_string(),
            }),
            [] => Err(GitWorkspaceError::InvalidBranchRef {
                full_ref: full_ref.to_string(),
            }),
            matches => Err(GitWorkspaceError::AmbiguousRemoteRef {
                full_ref: full_ref.to_string(),
                remotes: matches.iter().map(|(remote, _)| remote.clone()).collect(),
            }),
        };
    }

    Err(GitWorkspaceError::InvalidBranchRef {
        full_ref: full_ref.to_string(),
    })
}

#[derive(Default)]
struct WorktreeBuilder {
    path: Option<PathBuf>,
    head: Option<String>,
    branch: Option<String>,
    is_bare: bool,
    is_detached: bool,
    is_locked: bool,
    locked_reason: Option<String>,
    is_prunable: bool,
    prunable_reason: Option<String>,
}

impl WorktreeBuilder {
    fn finish(self) -> Result<WorktreeInfo, GitWorkspaceError> {
        let path = self
            .path
            .ok_or_else(|| GitWorkspaceError::InvalidWorktreeRecord {
                record: "missing worktree path".to_string(),
            })?;
        let path = match path.canonicalize() {
            Ok(path) => path,
            Err(source) if self.is_prunable && source.kind() == io::ErrorKind::NotFound => {
                normalize_missing_path(&path)?
            }
            Err(source) => {
                return Err(GitWorkspaceError::Canonicalize { path, source });
            }
        };
        Ok(WorktreeInfo {
            path,
            head: self.head,
            branch: self.branch,
            is_bare: self.is_bare,
            is_detached: self.is_detached,
            is_locked: self.is_locked,
            locked_reason: self.locked_reason,
            is_prunable: self.is_prunable,
            prunable_reason: self.prunable_reason,
        })
    }
}

fn normalize_missing_path(path: &Path) -> Result<PathBuf, GitWorkspaceError> {
    let mut ancestor = path;
    let mut missing_components = Vec::new();
    while !ancestor.exists() {
        let Some(file_name) = ancestor.file_name() else {
            return canonicalize(path);
        };
        missing_components.push(file_name.to_os_string());
        let Some(parent) = ancestor.parent() else {
            return canonicalize(path);
        };
        ancestor = parent;
    }

    let mut normalized = canonicalize(ancestor)?;
    for component in missing_components.into_iter().rev() {
        normalized.push(component);
    }
    Ok(normalized)
}

pub(crate) fn parse_worktrees(stdout: &[u8]) -> Result<Vec<WorktreeInfo>, GitWorkspaceError> {
    let mut worktrees = Vec::new();
    let mut current = WorktreeBuilder::default();
    let mut has_record = false;

    for field in stdout.split(|byte| *byte == 0) {
        if field.is_empty() {
            if has_record {
                worktrees.push(current.finish()?);
                current = WorktreeBuilder::default();
                has_record = false;
            }
            continue;
        }
        has_record = true;
        if let Some(path) = field.strip_prefix(b"worktree ") {
            current.path = Some(path_from_git_bytes(path)?);
        } else if let Some(head) = field.strip_prefix(b"HEAD ") {
            current.head = Some(decode_worktree_text(head)?);
        } else if let Some(branch) = field.strip_prefix(b"branch ") {
            current.branch = Some(decode_worktree_text(branch)?);
        } else if field == b"bare" {
            current.is_bare = true;
        } else if field == b"detached" {
            current.is_detached = true;
        } else if field == b"locked" {
            current.is_locked = true;
        } else if let Some(reason) = field.strip_prefix(b"locked ") {
            current.is_locked = true;
            current.locked_reason = Some(decode_worktree_text(reason)?);
        } else if field == b"prunable" {
            current.is_prunable = true;
        } else if let Some(reason) = field.strip_prefix(b"prunable ") {
            current.is_prunable = true;
            current.prunable_reason = Some(decode_worktree_text(reason)?);
        } else {
            return Err(GitWorkspaceError::InvalidWorktreeRecord {
                record: format!("{field:?}"),
            });
        }
    }
    if has_record {
        worktrees.push(current.finish()?);
    }
    Ok(worktrees)
}

fn decode_worktree_text(field: &[u8]) -> Result<String, GitWorkspaceError> {
    String::from_utf8(field.to_vec()).map_err(|_| GitWorkspaceError::InvalidWorktreeRecord {
        record: format!("non-UTF-8 text field {field:?}"),
    })
}

#[cfg(unix)]
fn path_from_git_bytes(path: &[u8]) -> Result<PathBuf, GitWorkspaceError> {
    use std::os::unix::ffi::OsStringExt;

    Ok(PathBuf::from(OsString::from_vec(path.to_vec())))
}

#[cfg(not(unix))]
fn path_from_git_bytes(path: &[u8]) -> Result<PathBuf, GitWorkspaceError> {
    String::from_utf8(path.to_vec())
        .map(PathBuf::from)
        .map_err(|_| GitWorkspaceError::InvalidWorktreeRecord {
            record: format!("non-UTF-8 worktree path {path:?}"),
        })
}

fn canonicalize(path: &Path) -> Result<PathBuf, GitWorkspaceError> {
    path.canonicalize()
        .map_err(|source| GitWorkspaceError::Canonicalize {
            path: path.to_path_buf(),
            source,
        })
}

fn output_path(
    repo: &Path,
    operation: &'static str,
    args: &[&str],
) -> Result<PathBuf, GitWorkspaceError> {
    let mut path_args = vec!["-c", "core.quotePath=false"];
    path_args.extend_from_slice(args);
    let output = git_output_for_operation(repo, operation, &path_args)?;
    decode_git_path_output(&output.stdout, operation)
}

pub(crate) fn decode_git_path_output(
    stdout: &[u8],
    operation: &'static str,
) -> Result<PathBuf, GitWorkspaceError> {
    #[cfg(unix)]
    let path = stdout.strip_suffix(b"\n").unwrap_or(stdout);
    #[cfg(not(unix))]
    let path = stdout
        .strip_suffix(b"\r\n")
        .or_else(|| stdout.strip_suffix(b"\n"))
        .unwrap_or(stdout);
    path_from_git_bytes(path).map_err(|error| match error {
        GitWorkspaceError::InvalidWorktreeRecord { .. } => {
            GitWorkspaceError::InvalidUtf8 { operation }
        }
        error => error,
    })
}

fn output_string(
    repo: &Path,
    operation: &'static str,
    args: &[&str],
) -> Result<String, GitWorkspaceError> {
    let output = git_output_for_operation(repo, operation, args)?;
    decode_stdout(output, operation)
}

fn decode_stdout(output: Output, operation: &'static str) -> Result<String, GitWorkspaceError> {
    String::from_utf8(output.stdout)
        .map(|stdout| stdout.trim().to_string())
        .map_err(|_| GitWorkspaceError::InvalidUtf8 { operation })
}

fn git_output(repo: &Path, args: &[&str]) -> Result<Output, GitWorkspaceError> {
    git_output_for_operation(repo, "run git command", args)
}

fn git_output_for_operation(
    repo: &Path,
    operation: &'static str,
    args: &[&str],
) -> Result<Output, GitWorkspaceError> {
    let display_args = args.iter().map(|arg| (*arg).to_string()).collect();
    let os_args: Vec<&OsStr> = args.iter().map(OsStr::new).collect();
    git_output_with_display_args_for_operation(repo, operation, &os_args, display_args)
}

pub(crate) fn git_output_with_os_args_for_operation(
    repo: &Path,
    operation: &'static str,
    args: &[&OsStr],
) -> Result<Output, GitWorkspaceError> {
    let display_args = args.iter().map(|arg| format!("{arg:?}")).collect();
    git_output_with_display_args_for_operation(repo, operation, args, display_args)
}

fn git_output_with_display_args_for_operation(
    repo: &Path,
    operation: &'static str,
    args: &[&OsStr],
    display_args: Vec<String>,
) -> Result<Output, GitWorkspaceError> {
    let output = execute_git(repo, operation, args, display_args.clone())?;
    if output.status.success() {
        Ok(output)
    } else {
        Err(GitWorkspaceError::CommandFailed {
            operation,
            args: display_args,
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        })
    }
}

fn git_output_allow_failure_for_operation(
    repo: &Path,
    operation: &'static str,
    args: &[&str],
) -> Result<Output, GitWorkspaceError> {
    let display_args = args.iter().map(|arg| (*arg).to_string()).collect();
    let os_args: Vec<&OsStr> = args.iter().map(OsStr::new).collect();
    execute_git(repo, operation, &os_args, display_args)
}

fn execute_git(
    repo: &Path,
    operation: &'static str,
    args: &[&OsStr],
    display_args: Vec<String>,
) -> Result<Output, GitWorkspaceError> {
    let output = command::blocking::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args.iter().copied())
        .output()
        .map_err(|source| GitWorkspaceError::CommandIo {
            operation,
            args: display_args,
            source,
        })?;
    Ok(output)
}

fn command_failed(operation: &'static str, args: &[&str], output: &Output) -> GitWorkspaceError {
    GitWorkspaceError::CommandFailed {
        operation,
        args: args.iter().map(|arg| (*arg).to_string()).collect(),
        stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
    }
}

async fn spawn_git_task<T, F>(operation: &'static str, task: F) -> Result<T, GitWorkspaceError>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, GitWorkspaceError> + Send + 'static,
{
    tokio::task::spawn_blocking(task).await.map_err(|error| {
        GitWorkspaceError::BackgroundTaskFailed {
            operation,
            message: error.to_string(),
        }
    })?
}

# Worktree Git Ownership Safety Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 关闭 worktree 创建目标目录的 TOCTOU、创建失败时的 branch 误清理，以及删除校验完成后的 branch/merge-target 漂移窗口。

**Architecture:** 创建流程通过共享 `TargetDirectoryClaim` 原子创建最终目录；remote 创建由单个 `git worktree add --no-track -b` 创建 branch，失败时只清理未注册且为空的 claimed 目录。删除流程通过 prepared `git update-ref --stdin` transaction 锁定 branch 和 merge target，在持锁后重新执行 worktree、branch、dirty、target selection 和 merge 权威校验，再跨越 `git worktree remove` 提交 branch delete。

**Tech Stack:** Rust 2021、`command::blocking::Command`、Git `worktree` / `update-ref --stdin`、`tempfile`、Tokio `spawn_blocking`。

---

## File Map

- Create `app/src/project_organization/git/ref_transaction.rs`: `update-ref --stdin` 协议、stderr drain、prepare/abort/commit 生命周期。
- Create `app/src/project_organization/git/ref_transaction_tests.rs`: 真实 Git transaction prototype、ref lock、abort 和 target drift 测试。
- Modify `app/src/project_organization/git.rs`: 目标目录 claim、创建残留、preflight snapshot、锁内删除编排和结构化错误。
- Modify `app/src/project_organization/git_tests.rs`: 创建 TOCTOU、残留、锁前/锁后竞态、部分 mutation 和 source chain 回归测试。
- Modify `specs/repository-workspaces/TECH.md`: 同步最终创建与删除语义。

### Task 1: Add the prepared ref transaction protocol

**Files:**
- Create: `app/src/project_organization/git/ref_transaction.rs`
- Create: `app/src/project_organization/git/ref_transaction_tests.rs`
- Modify: `app/src/project_organization/git.rs:1-10`

- [ ] **Step 1: Add the private module and write the failing real-Git prototype**

在 `git.rs` 顶部加入：

```rust
mod ref_transaction;
```

在 `ref_transaction.rs` 末尾接入测试文件：

```rust
#[cfg(test)]
#[path = "ref_transaction_tests.rs"]
mod tests;
```

测试 fixture 必须使用 `command::blocking::Command`，并提供这些具名 helper：

```rust
struct TransactionFixture {
    _tempdir: tempfile::TempDir,
    root: PathBuf,
}

impl TransactionFixture {
    fn new() -> Self;
    fn add_worktree(&self, branch: &str) -> PathBuf;
    fn rev_parse(&self, full_ref: &str) -> String;
    fn ref_exists(&self, full_ref: &str) -> bool;
    fn advance_ref(&self, full_ref: &str) -> String;
}

fn run_git(repository: &Path, args: &[&str]);
```

`new` 创建 `main` repository、配置 test identity、提交 `README.md`；`add_worktree` 执行真实 `git worktree add -b`。先写 prototype：

```rust
#[test]
fn prepared_delete_transaction_spans_worktree_remove() {
    let fixture = TransactionFixture::new();
    let worktree = fixture.add_worktree("feature/transaction");
    let branch_ref = "refs/heads/feature/transaction";
    let branch_oid = fixture.rev_parse(branch_ref);
    let target_ref = "refs/heads/main";
    let target_oid = fixture.rev_parse(target_ref);

    let transaction = PreparedRefDelete::prepare(
        &fixture.root,
        branch_ref,
        &branch_oid,
        Some(LockedRef {
            full_ref: target_ref,
            oid: &target_oid,
        }),
    )
    .unwrap();

    let status = command::blocking::Command::new("git")
        .arg("-C")
        .arg(&fixture.root)
        .args(["worktree", "remove"])
        .arg(&worktree)
        .status()
        .unwrap();
    assert!(status.success());

    transaction.commit().unwrap();
    assert!(!worktree.exists());
    assert!(!fixture.ref_exists(branch_ref));
}
```

- [ ] **Step 2: Run the prototype and verify RED**

```bash
cargo test -p warp --lib project_organization::git::ref_transaction::tests::prepared_delete_transaction_spans_worktree_remove -- --nocapture
```

Expected: compile failure because `PreparedRefDelete`, `LockedRef`, and transaction errors do not exist.

- [ ] **Step 3: Implement the exact line protocol and process lifecycle**

Define the module-private diagnostic types and transaction API:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum RefTransactionStage {
    Start,
    Prepare,
    Commit,
    Abort,
    Wait,
    ReadStderr,
}

#[derive(Debug, thiserror::Error)]
pub(super) enum RefTransactionError {
    #[error("failed to start git update-ref transaction: {source}")]
    Start {
        #[source]
        source: io::Error,
    },
    #[error("git update-ref transaction is missing its {pipe} pipe")]
    MissingPipe { pipe: &'static str },
    #[error("failed to communicate with git update-ref during {stage:?}: {source}")]
    Io {
        stage: RefTransactionStage,
        #[source]
        source: io::Error,
    },
    #[error("git update-ref returned `{response}` during {stage:?}, expected `{expected}`")]
    UnexpectedResponse {
        stage: RefTransactionStage,
        expected: &'static str,
        response: String,
    },
    #[error("git update-ref exited during {stage:?}: {stderr}")]
    ProcessExited {
        stage: RefTransactionStage,
        stderr: String,
    },
    #[error("failed to join git update-ref stderr reader during {stage:?}")]
    StderrReaderPanicked { stage: RefTransactionStage },
}

pub(super) struct LockedRef<'a> {
    pub(super) full_ref: &'a str,
    pub(super) oid: &'a str,
}

pub(super) struct PreparedRefDelete {
    child: std::process::Child,
    stdin: Option<std::io::BufWriter<std::process::ChildStdin>>,
    stdout: std::io::BufReader<std::process::ChildStdout>,
    stderr_reader: Option<std::thread::JoinHandle<io::Result<Vec<u8>>>>,
    completed: bool,
}
```

`prepare` 必须使用下面的命令和顺序：

```rust
let mut child = command::blocking::Command::new("git")
    .arg("-C")
    .arg(repository)
    .args(["update-ref", "--stdin"])
    .stdin(command::Stdio::piped())
    .stdout(command::Stdio::piped())
    .stderr(command::Stdio::piped())
    .spawn()
    .map_err(|source| RefTransactionError::Start { source })?;
```

stderr pipe 立即交给独立 `std::thread::spawn` 执行 `read_to_end`，避免错误输出填满 pipe。协议只等待 Git 实际返回的 control response：

```text
send: start
read: start: ok
send: verify <target-ref> <target-oid>  # only when target exists
send: delete <branch-ref> <branch-oid>
send: prepare
read: prepare: ok
```

不得为 branch ref 再发送 `verify`；`delete <ref> <old-oid>` 已执行 compare-and-delete，Git 会拒绝同一 transaction 中对同 ref 的重复更新。实现 `send_line`、`expect_response`、`finish_process`，response 使用 `trim_end_matches(['\r', '\n'])`，未知 response 或 EOF 均返回结构化错误。

`commit(mut self)` 发送 `commit`、等待 `commit: ok`、关闭 stdin、等待 child exit 并 join stderr reader。`abort(mut self)` 同理发送 `abort` 并等待 `abort: ok`。`Drop` 仅在 `completed == false` 时 best-effort 发送 `abort`、关闭 stdin 并 wait，不覆盖调用方已有错误。

- [ ] **Step 4: Add lock, abort, and target-drift tests**

```rust
#[test]
fn prepared_transaction_blocks_branch_updates_until_abort() {
    let fixture = TransactionFixture::new();
    let branch_ref = "refs/heads/feature/locked";
    let worktree = fixture.add_worktree("feature/locked");
    let branch_oid = fixture.rev_parse(branch_ref);
    let changed_oid = fixture.advance_ref("refs/heads/main");
    let transaction = PreparedRefDelete::prepare(
        &fixture.root,
        branch_ref,
        &branch_oid,
        None,
    )
    .unwrap();

    let output = command::blocking::Command::new("git")
        .arg("-C")
        .arg(&fixture.root)
        .args(["update-ref", branch_ref, &changed_oid, &branch_oid])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("cannot lock ref"));

    transaction.abort().unwrap();
    assert!(worktree.exists());
    assert_eq!(fixture.rev_parse(branch_ref), branch_oid);
}

#[test]
fn prepare_rejects_changed_merge_target() {
    let fixture = TransactionFixture::new();
    fixture.add_worktree("feature/target-drift");
    let branch_ref = "refs/heads/feature/target-drift";
    let branch_oid = fixture.rev_parse(branch_ref);
    let target_ref = "refs/heads/main";
    let stale_target_oid = fixture.rev_parse(target_ref);
    fixture.advance_ref(target_ref);

    let error = PreparedRefDelete::prepare(
        &fixture.root,
        branch_ref,
        &branch_oid,
        Some(LockedRef {
            full_ref: target_ref,
            oid: &stale_target_oid,
        }),
    )
    .unwrap_err();
    assert!(matches!(
        error,
        RefTransactionError::ProcessExited {
            stage: RefTransactionStage::Prepare,
            ..
        } | RefTransactionError::UnexpectedResponse {
            stage: RefTransactionStage::Prepare,
            ..
        }
    ));
}
```

- [ ] **Step 5: Run GREEN, format only touched Rust files, and commit**

```bash
rustfmt --edition 2021 app/src/project_organization/git.rs app/src/project_organization/git/ref_transaction.rs app/src/project_organization/git/ref_transaction_tests.rs
cargo test -p warp --lib project_organization::git::ref_transaction::tests -- --nocapture
git add app/src/project_organization/git.rs app/src/project_organization/git/ref_transaction.rs app/src/project_organization/git/ref_transaction_tests.rs
git commit -m "feat: add prepared git ref transactions"
```

Expected: all transaction tests pass with non-zero test count. If the real prototype fails on a supported platform, stop and redesign; do not fall back to post-remove OID-only deletion.

#### Task 1 Execution Evidence

- 初始 RED 命令为 `cargo test -p warp --lib project_organization::git::ref_transaction::tests::prepared_delete_transaction_spans_worktree_remove -- --nocapture`，因事务类型尚未定义而失败，错误为 `E0432`。
- P2 的 Drop lock-release 与 stale branch-OID 测试添加时立即 GREEN，因为实现已具备 Drop abort/wait 与 expected-old-OID CAS；没有为制造 RED 而破坏协议。
- 最终 transaction 模块测试为 6/6 通过。
- 质量审查后收窄无消费者的 re-export；Task 4 接入时再公开必要诊断类型。

### Task 2: Atomically claim worktree target directories

**Files:**
- Modify: `app/src/project_organization/git.rs:187-220, 473-710, 1359-1490`
- Modify: `app/src/project_organization/git_tests.rs:689-1289`

- [ ] **Step 1: Replace old branch-cleanup expectations with failing target-claim tests**

Add test-only hooks that call the same private core used by production:

```rust
#[cfg(test)]
pub(crate) fn create_from_remote_with_after_target_claim_hook<F>(
    repository: &Path,
    remote_ref: &str,
    new_branch: &str,
    worktree_path: &Path,
    after_target_claim: F,
) -> Result<(), GitWorkspaceError>
where
    F: FnOnce(),
```

```rust
#[cfg(test)]
pub(crate) fn create_from_local_with_after_target_claim_hook<F>(
    repository: &Path,
    local_branch: &str,
    worktree_path: &Path,
    after_target_claim: F,
) -> Result<(), GitWorkspaceError>
where
    F: FnOnce(),
```

```rust
#[test]
fn remote_creation_claims_target_before_git_command() {
    let fixture = GitFixture::new();
    let target = fixture.tempdir.path().join("claimed remote target");
    let mut saw_existing_claim = false;

    create_from_remote_with_after_target_claim_hook(
        &fixture.root,
        "refs/remotes/origin/main",
        "feature/claimed-remote",
        &target,
        || {
            let error = std::fs::create_dir(&target).unwrap_err();
            saw_existing_claim = error.kind() == std::io::ErrorKind::AlreadyExists;
        },
    )
    .unwrap();

    assert!(saw_existing_claim);
    assert_eq!(current_branch(&target), "feature/claimed-remote");
}

#[test]
fn local_creation_claims_target_before_git_command() {
    let fixture = GitFixture::new();
    run_git(&fixture.root, &["branch", "feature/claimed-local"]);
    let target = fixture.tempdir.path().join("claimed local target");
    let mut saw_existing_claim = false;

    create_from_local_with_after_target_claim_hook(
        &fixture.root,
        "feature/claimed-local",
        &target,
        || {
            let error = std::fs::create_dir(&target).unwrap_err();
            saw_existing_claim = error.kind() == std::io::ErrorKind::AlreadyExists;
        },
    )
    .unwrap();

    assert!(saw_existing_claim);
    assert_eq!(current_branch(&target), "feature/claimed-local");
}

#[test]
fn creation_rejects_claim_replaced_by_file_before_git_command() {
    let fixture = GitFixture::new();
    let target = fixture.tempdir.path().join("replaced claimed target");
    let error = create_from_remote_with_after_target_claim_hook(
        &fixture.root,
        "refs/remotes/origin/main",
        "feature/replaced-claim",
        &target,
        || {
            std::fs::remove_dir(&target).unwrap();
            std::fs::write(&target, "replacement").unwrap();
        },
    )
    .unwrap_err();

    assert!(matches!(error, GitWorkspaceError::ClaimedTargetNotDirectory { .. }));
    assert!(!ref_exists(
        &fixture.root,
        "refs/heads/feature/replaced-claim"
    ));
}
```

- [ ] **Step 2: Run RED**

```bash
cargo test -p warp --lib remote_creation_claims_target_before_git_command -- --nocapture
cargo test -p warp --lib local_creation_claims_target_before_git_command -- --nocapture
cargo test -p warp --lib creation_rejects_claim_replaced_by_file_before_git_command -- --nocapture
```

Expected: compile failure because the target-claim hooks and `TargetDirectoryClaim` do not exist.

- [ ] **Step 3: Implement the shared target claim and exact errors**

Replace `validate_target_missing` in both local and remote creation with:

```rust
struct TargetDirectoryClaim {
    requested_path: PathBuf,
    canonical_path: PathBuf,
}

impl TargetDirectoryClaim {
    fn acquire(path: &Path) -> Result<Self, GitWorkspaceError> {
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
```

Add the error variant:

```rust
#[error("failed to claim worktree target `{path}`: {source}")]
TargetClaimFailed {
    path: PathBuf,
    #[source]
    source: io::Error,
},

#[error("failed to inspect claimed worktree target `{path}`: {source}")]
ClaimedTargetInspection {
    path: PathBuf,
    #[source]
    source: io::Error,
},

#[error("claimed worktree target `{path}` is no longer a directory")]
ClaimedTargetNotDirectory { path: PathBuf },

#[error("claimed worktree target `{path}` is not empty")]
ClaimedTargetNotEmpty { path: PathBuf },

#[error("failed to clean up claimed worktree target `{path}`: {source}")]
ClaimedTargetCleanupFailed {
    path: PathBuf,
    #[source]
    source: io::Error,
},
```

Remote creation must execute exactly:

```rust
let args = [
    OsStr::new("worktree"),
    OsStr::new("add"),
    OsStr::new("--no-track"),
    OsStr::new("-b"),
    OsStr::new(new_branch),
    claim.canonical_path.as_os_str(),
    OsStr::new(remote_ref),
];
```

Local creation keeps its existing branch validation, then uses the same claim and executes `git worktree add <claimed-path> <local-branch>`. Do not use production `to_str()` or lossy path conversion.

Git runners and remote success verification must receive `claim.canonical_path`, so an input relative to the calling process has the same meaning as the already-created claim when Git runs with `-C repository`. Keep `claim.requested_path` for claim inspection, cleanup, and error display.

Both creation paths call `claim.ensure_directory()` immediately before `git worktree add` and again after a successful command. A failed local `worktree add` uses the same registration/empty-directory inspection as remote creation with `branch_may_remain: false`; a failed remote `worktree add -b` uses `branch_may_remain: true`.

Make production and test paths share these two private cores; the runner owns only the Git command so residual handling remains common:

```rust
fn create_from_remote_inner<AfterTargetClaim, Runner>(
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
            .map_err(|verification_error| GitWorkspaceError::WorktreeCreationVerificationFailed {
                worktree_path: claim.requested_path,
                branch: new_branch.to_string(),
                expected_oid,
                verification_error: Box::new(verification_error),
            })
        }
        Err(create_error) => worktree_creation_failure(
            repository,
            claim,
            new_branch,
            true,
            create_error,
        ),
    }
}

fn create_from_local_inner<AfterTargetClaim, Runner>(
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
        Ok(()) => {
            claim.ensure_directory()?;
            Ok(())
        }
        Err(create_error) => worktree_creation_failure(
            repository,
            claim,
            local_branch,
            false,
            create_error,
        ),
    }
}
```

Implement the production command runners and hook wrappers as follows:

```rust
fn run_remote_worktree_add(
    repository: &Path,
    remote_ref: &str,
    new_branch: &str,
    worktree_path: &Path,
) -> Result<(), GitWorkspaceError> {
    let args = [
        OsStr::new("worktree"),
        OsStr::new("add"),
        OsStr::new("--no-track"),
        OsStr::new("-b"),
        OsStr::new(new_branch),
        worktree_path.as_os_str(),
        OsStr::new(remote_ref),
    ];
    git_output_with_os_args_for_operation(repository, "create worktree from remote", &args)?;
    Ok(())
}

fn run_local_worktree_add(
    repository: &Path,
    local_branch: &str,
    worktree_path: &Path,
) -> Result<(), GitWorkspaceError> {
    let args = [
        OsStr::new("worktree"),
        OsStr::new("add"),
        worktree_path.as_os_str(),
        OsStr::new(local_branch),
    ];
    git_output_with_os_args_for_operation(repository, "create worktree from local branch", &args)?;
    Ok(())
}

pub fn create_from_remote(
    repository: &Path,
    remote_ref: &str,
    new_branch: &str,
    worktree_path: &Path,
) -> Result<(), GitWorkspaceError> {
    create_from_remote_inner(
        repository,
        remote_ref,
        new_branch,
        worktree_path,
        || {},
        |repository, remote_ref, new_branch, worktree_path, _| {
            run_remote_worktree_add(repository, remote_ref, new_branch, worktree_path)
        },
    )
}

pub fn create_from_local(
    repository: &Path,
    local_branch: &str,
    worktree_path: &Path,
) -> Result<(), GitWorkspaceError> {
    create_from_local_inner(
        repository,
        local_branch,
        worktree_path,
        || {},
        run_local_worktree_add,
    )
}

#[cfg(test)]
pub(crate) fn create_from_remote_with_after_target_claim_hook<F>(
    repository: &Path,
    remote_ref: &str,
    new_branch: &str,
    worktree_path: &Path,
    after_target_claim: F,
) -> Result<(), GitWorkspaceError>
where
    F: FnOnce(),
{
    create_from_remote_inner(
        repository,
        remote_ref,
        new_branch,
        worktree_path,
        after_target_claim,
        |repository, remote_ref, new_branch, worktree_path, _| {
            run_remote_worktree_add(repository, remote_ref, new_branch, worktree_path)
        },
    )
}

#[cfg(test)]
pub(crate) fn create_from_local_with_after_target_claim_hook<F>(
    repository: &Path,
    local_branch: &str,
    worktree_path: &Path,
    after_target_claim: F,
) -> Result<(), GitWorkspaceError>
where
    F: FnOnce(),
{
    create_from_local_inner(
        repository,
        local_branch,
        worktree_path,
        after_target_claim,
        run_local_worktree_add,
    )
}
```

`create_from_remote_with_runner` calls the same remote core with a no-op hook and its caller-supplied runner. `create_from_local` and `create_from_local_with_runner` follow the matching local core. Each core acquires the claim, invokes its hook, checks `ensure_directory`, runs the runner, checks `ensure_directory` after success, and routes every runner error through the common residual inspection.

- [ ] **Step 4: Write failing remote residual and directory-cleanup tests**

Define a test-only runner whose closure receives the expected OID and may create real Git state before returning an injected error:

```rust
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
    create_from_remote_inner(
        repository,
        remote_ref,
        new_branch,
        worktree_path,
        || {},
        runner,
    )
}
```

The runner arguments are, in order: repository, remote ref, short new branch name, claimed path, and expected OID. Define the reusable test error before the tests:

```rust
fn injected_command_error(operation: &'static str) -> GitWorkspaceError {
    GitWorkspaceError::CommandFailed {
        operation,
        args: vec!["injected".to_string()],
        stderr: "injected failure".to_string(),
    }
}
```

```rust
#[test]
fn remote_creation_failure_preserves_possible_branch_residual() {
    let fixture = GitFixture::new();
    let target = fixture.tempdir.path().join("branch residual target");

    let error = create_from_remote_with_runner(
        &fixture.root,
        "refs/remotes/origin/main",
        "feature/residual",
        &target,
        |repository, _remote_ref, branch, _target, expected_oid| {
            let full_ref = format!("refs/heads/{branch}");
            run_git(repository, &["update-ref", &full_ref, expected_oid]);
            Err(GitWorkspaceError::CommandFailed {
                operation: "injected remote worktree creation",
                args: vec!["worktree".into(), "add".into()],
                stderr: "injected failure".into(),
            })
        },
    )
    .unwrap_err();

    assert!(matches!(
        error,
        GitWorkspaceError::WorktreeCreationFailed {
            branch_may_remain: true,
            worktree_registered: Some(false),
            claimed_directory_removed: true,
            ..
        }
    ));
    assert!(ref_exists(&fixture.root, "refs/heads/feature/residual"));
    assert!(!target.exists());
}

#[test]
fn failed_creation_keeps_nonempty_unregistered_claim() {
    let fixture = GitFixture::new();
    let target = fixture.tempdir.path().join("nonempty claimed target");
    let error = create_from_remote_with_runner(
        &fixture.root,
        "refs/remotes/origin/main",
        "feature/nonempty-residual",
        &target,
        |_repository, _remote_ref, _branch, target, _expected_oid| {
            std::fs::write(target.join("keep.txt"), "keep").unwrap();
            Err(injected_command_error("remote creation"))
        },
    )
    .unwrap_err();

    assert!(matches!(
        error,
        GitWorkspaceError::WorktreeCreationFailed {
            worktree_registered: Some(false),
            claimed_directory_removed: false,
            cleanup_error: Some(_),
            ..
        }
    ));
    assert_eq!(std::fs::read_to_string(target.join("keep.txt")).unwrap(), "keep");
}

#[test]
fn failed_creation_never_removes_registered_claim() {
    let fixture = GitFixture::new();
    let target = fixture.tempdir.path().join("registered claimed target");
    let error = create_from_remote_with_runner(
        &fixture.root,
        "refs/remotes/origin/main",
        "feature/registered-residual",
        &target,
        |repository, remote_ref, branch, target, _expected_oid| {
            let output = command::blocking::Command::new("git")
                .arg("-C")
                .arg(repository)
                .args(["worktree", "add", "--no-track", "-b", branch])
                .arg(target)
                .arg(remote_ref)
                .output()
                .unwrap();
            assert!(output.status.success());
            Err(injected_command_error("remote creation after registration"))
        },
    )
    .unwrap_err();

    assert!(matches!(
        error,
        GitWorkspaceError::WorktreeCreationFailed {
            worktree_registered: Some(true),
            claimed_directory_removed: false,
            ..
        }
    ));
    assert!(target.exists());
    assert!(ref_exists(
        &fixture.root,
        "refs/heads/feature/registered-residual"
    ));
}

#[test]
fn local_creation_failure_removes_empty_unregistered_claim() {
    let fixture = GitFixture::new();
    run_git(&fixture.root, &["branch", "feature/local-failure"]);
    let target = fixture.tempdir.path().join("local failure target");
    let error = create_from_local_with_runner(
        &fixture.root,
        "feature/local-failure",
        &target,
        |_repository, _branch, _target| Err(injected_command_error("local creation")),
    )
    .unwrap_err();

    assert!(matches!(
        error,
        GitWorkspaceError::WorktreeCreationFailed {
            branch_may_remain: false,
            worktree_registered: Some(false),
            claimed_directory_removed: true,
            ..
        }
    ));
    assert!(!target.exists());
    assert!(ref_exists(&fixture.root, "refs/heads/feature/local-failure"));
}
```

- [ ] **Step 5: Implement branch-preserving residual inspection**

Add the local test runner and replace `cleanup_failed_remote_creation*` and all branch compare-delete cleanup with:

```rust
#[cfg(test)]
pub(crate) fn create_from_local_with_runner<Runner>(
    repository: &Path,
    local_branch: &str,
    worktree_path: &Path,
    runner: Runner,
) -> Result<(), GitWorkspaceError>
where
    Runner: FnOnce(&Path, &str, &Path) -> Result<(), GitWorkspaceError>,
{
    create_from_local_inner(repository, local_branch, worktree_path, || {}, runner)
}
```

```rust
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
```

Failure handling must follow this exact order:

```rust
let registration = inspect_claimed_worktree_registration(repository, &claim.canonical_path);
let (worktree_registered, mut cleanup_error) = match registration {
    Ok(registered) => (Some(registered), None),
    Err(error) => (None, Some(Box::new(error))),
};

let claimed_directory_removed = if worktree_registered == Some(false) {
    match remove_claimed_directory_if_empty(&claim.requested_path) {
        Ok(removed) => removed,
        Err(error) => {
            cleanup_error = Some(Box::new(error));
            false
        }
    }
} else {
    false
};
```

Implement the helper and both leaf inspections exactly once:

```rust
fn worktree_creation_failure(
    repository: &Path,
    claim: TargetDirectoryClaim,
    branch: &str,
    branch_may_remain: bool,
    create_error: GitWorkspaceError,
) -> Result<(), GitWorkspaceError> {
    let registration = inspect_claimed_worktree_registration(repository, &claim.canonical_path);
    let (worktree_registered, mut cleanup_error) = match registration {
        Ok(registered) => (Some(registered), None),
        Err(error) => (None, Some(Box::new(error))),
    };
    let claimed_directory_removed = if worktree_registered == Some(false) {
        match remove_claimed_directory_if_empty(&claim.requested_path) {
            Ok(removed) => removed,
            Err(error) => {
                cleanup_error = Some(Box::new(error));
                false
            }
        }
    } else {
        false
    };
    Err(GitWorkspaceError::WorktreeCreationFailed {
        worktree_path: claim.requested_path,
        branch: branch.to_string(),
        branch_may_remain,
        worktree_registered,
        claimed_directory_removed,
        create_error: Box::new(create_error),
        cleanup_error,
    })
}

fn inspect_claimed_worktree_registration(
    repository: &Path,
    claimed_path: &Path,
) -> Result<bool, GitWorkspaceError> {
    Ok(list_worktrees(repository)?
        .into_iter()
        .any(|worktree| worktree.path == claimed_path))
}

fn remove_claimed_directory_if_empty(path: &Path) -> Result<bool, GitWorkspaceError> {
    let mut entries = std::fs::read_dir(path).map_err(|source| {
        GitWorkspaceError::ClaimedTargetCleanupFailed {
            path: path.to_path_buf(),
            source,
        }
    })?;
    if entries.next().is_some() {
        return Err(GitWorkspaceError::ClaimedTargetNotEmpty {
            path: path.to_path_buf(),
        });
    }
    std::fs::remove_dir(path).map_err(|source| GitWorkspaceError::ClaimedTargetCleanupFailed {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(true)
}
```

`remove_claimed_directory_if_empty` maps a non-empty directory to `ClaimedTargetNotEmpty`; it maps `read_dir` and `remove_dir` I/O failures to `ClaimedTargetCleanupFailed`. It calls `remove_dir` only when `next().is_none()` and never calls `remove_dir_all`. Remote failure sets `branch_may_remain` to `true`, because the service deliberately does not infer whether Git created or another process recreated the new branch. Local failure sets it to `false`, because it only attaches an already validated existing branch.

- [ ] **Step 6: Run creation tests and commit**

Delete tests that require automatic branch cleanup: `atomic_claim_preserves_*`, `remote_creation_failure_cleans_branch_created_at_expected_oid`, `remote_creation_cleanup_*`, and `remote_creation_failure_preserves_concurrently_changed_branch`. Preserve validation, path, direct/symbolic verification, and async coverage.

```bash
rustfmt --edition 2021 app/src/project_organization/git.rs app/src/project_organization/git_tests.rs
cargo test -p warp --lib project_organization::git_tests -- --nocapture
git add app/src/project_organization/git.rs app/src/project_organization/git_tests.rs
git commit -m "fix: claim worktree targets atomically"
```

Expected: all Git service tests pass; failures never delete a branch and only remove an empty, unregistered claimed directory.

#### Task 2 Execution Evidence

- Relative-path quality fix: `TargetDirectoryClaim::acquire` creates and canonicalizes the target in the caller's current directory; both production runners and remote verification now receive `claim.canonical_path`, while claim inspection and cleanup retain `claim.requested_path`.
- Regression coverage: real remote and local Git creation tests pass unique relative targets without changing the global CWD, verify the branch at the canonical target, verify no repository-relative target was created, and remove the created worktree through `git -C <repository> worktree remove <canonical-target>`.

### Task 3: Complete creation postcondition verification and preflight snapshots

**Files:**
- Modify: `app/src/project_organization/git.rs:47-60, 610-655, 716-826`
- Modify: `app/src/project_organization/git_tests.rs:689-856, 1311-1542`

- [ ] **Step 1: Write failing upstream and immutable target-OID tests**

```rust
#[test]
fn successful_remote_creation_rejects_new_upstream() {
    let fixture = GitFixture::new();
    let target = fixture.tempdir.path().join("unexpected upstream target");

    let error = create_from_remote_with_success_hook(
        &fixture.root,
        "refs/remotes/origin/main",
        "feature/unexpected-upstream",
        &target,
        || {
            run_git(
                &fixture.root,
                &[
                    "branch",
                    "--set-upstream-to=origin/main",
                    "feature/unexpected-upstream",
                ],
            );
        },
    )
    .unwrap_err();

    assert!(matches!(
        error,
        GitWorkspaceError::WorktreeCreationVerificationFailed {
            verification_error,
            ..
        } if matches!(
            verification_error.as_ref(),
            GitWorkspaceError::UnexpectedBranchUpstream { upstream, .. }
                if upstream == "refs/remotes/origin/main"
        )
    ));
}

#[test]
fn deletion_preflight_captures_merge_target_oid() {
    let fixture = GitFixture::new();
    let worktree = fixture.add_linked_worktree("feature/preflight-target");
    let preflight = deletion_preflight(&fixture.root, &worktree, true).unwrap();
    let target = preflight.merge_target.unwrap();

    assert_eq!(target.full_ref, "refs/remotes/origin/main");
    assert_eq!(target.oid, ref_oid(&fixture.root, &target.full_ref));
}

#[test]
fn deletion_preflight_rejects_self_merge_target() {
    let fixture = GitFixture::new();
    let worktree = fixture.add_linked_worktree("feature/self-target");
    run_git(
        &fixture.root,
        &[
            "config",
            "branch.feature/self-target.remote",
            ".",
        ],
    );
    run_git(
        &fixture.root,
        &[
            "config",
            "branch.feature/self-target.merge",
            "refs/heads/feature/self-target",
        ],
    );

    let error = deletion_preflight(&fixture.root, &worktree, true).unwrap_err();
    assert!(matches!(error, GitWorkspaceError::InvalidMergeTarget { .. }));
}
```

- [ ] **Step 2: Run RED**

```bash
cargo test -p warp --lib successful_remote_creation_rejects_new_upstream -- --nocapture
cargo test -p warp --lib deletion_preflight_captures_merge_target_oid -- --nocapture
cargo test -p warp --lib deletion_preflight_rejects_self_merge_target -- --nocapture
```

Expected: missing upstream validation, `MergeTargetSnapshot`, and `InvalidMergeTarget` failures.

- [ ] **Step 3: Add exact creation and preflight types**

Add:

```rust
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MergeTargetSnapshot {
    pub full_ref: String,
    pub oid: String,
    pub is_merged: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeletionPreflight {
    pub worktree_path: PathBuf,
    pub branch: String,
    pub branch_ref: String,
    pub branch_oid: String,
    pub merge_target: Option<MergeTargetSnapshot>,
}
```

Extract the existing read-only checks into a shared private capture function, then keep the public UI preflight as a thin wrapper:

```rust
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
    let Some(branch_ref) = worktree.branch else {
        return Err(GitWorkspaceError::WorktreeHasNoLocalBranch {
            path: worktree_path,
        });
    };
    let Some(branch) = branch_ref.strip_prefix("refs/heads/") else {
        return Err(GitWorkspaceError::WorktreeHasNoLocalBranch {
            path: worktree_path,
        });
    };
    if branch.is_empty() {
        return Err(GitWorkspaceError::WorktreeHasNoLocalBranch {
            path: worktree_path,
        });
    }
    let branch_snapshot = direct_ref_snapshot(repository, &branch_ref)?;
    if branch_snapshot.symbolic_target.is_some() {
        return Err(GitWorkspaceError::WorktreeHasNoLocalBranch {
            path: worktree_path,
        });
    }
    let Some(branch_oid) = branch_snapshot.direct_oid else {
        return Err(GitWorkspaceError::BranchNotFound { full_ref: branch_ref });
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
    let merge_target = if include_merge_target {
        let target_ref = resolve_merge_target_ref(repository, &branch_ref)?;
        if target_ref == branch_ref {
            return Err(GitWorkspaceError::InvalidMergeTarget {
                branch_ref,
                target_ref,
            });
        }
        let target_oid = resolve_commit_oid(repository, &target_ref)?;
        Some(MergeTargetSnapshot {
            is_merged: is_ancestor(repository, &branch_oid, &target_oid)?,
            full_ref: target_ref,
            oid: target_oid,
        })
    } else {
        None
    };
    Ok(DeletionPreflight {
        worktree_path,
        branch: branch.to_string(),
        branch_ref,
        branch_oid,
        merge_target,
    })
}

pub fn deletion_preflight(
    repository: &Path,
    worktree_path: &Path,
    delete_branch: bool,
) -> Result<DeletionPreflight, GitWorkspaceError> {
    capture_deletion_candidate(repository, worktree_path, delete_branch)
}
```

For `include_merge_target=false`, set `merge_target=None` and do not query remote/upstream. For `include_merge_target=true`:

```rust
let target_ref = resolve_merge_target_ref(repository, &full_ref)?;
if target_ref == full_ref {
    return Err(GitWorkspaceError::InvalidMergeTarget {
        branch_ref: full_ref,
        target_ref,
    });
}
let target_oid = resolve_commit_oid(repository, &target_ref)?;
let is_merged = is_ancestor(repository, &branch_oid, &target_oid)?;
```

Define both helpers beside the existing `branch_upstream` and `resolve_commit_oid` helpers:

```rust
fn resolve_merge_target_ref(
    repository: &Path,
    branch_ref: &str,
) -> Result<String, GitWorkspaceError> {
    if let Some(upstream) = branch_upstream(repository, branch_ref)? {
        return Ok(upstream);
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
    let output = git_output_allow_failure_for_operation(
        repository,
        "check branch merge status",
        &args,
    )?;
    match output.status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        Some(_) | None => Err(command_failed("check branch merge status", &args, &output)),
    }
}
```

Add errors:

```rust
#[error("merge target `{target_ref}` must differ from branch `{branch_ref}`")]
InvalidMergeTarget {
    branch_ref: String,
    target_ref: String,
},

#[error("branch `{branch}` unexpectedly tracks `{upstream}`")]
UnexpectedBranchUpstream {
    branch: String,
    upstream: String,
},
```

`verify_remote_worktree_creation` calls `branch_upstream(repository, &expected_branch)?` after direct OID validation and returns `UnexpectedBranchUpstream` for `Some(upstream)`. Add `#[source]` to `verification_error` in `WorktreeCreationVerificationFailed`.

- [ ] **Step 4: Update existing preflight assertions and run GREEN**

Replace `preflight.is_merged` / string sentinel assertions with:

```rust
let target = preflight.merge_target.as_ref().unwrap();
assert!(target.is_merged);
assert_eq!(target.full_ref, "refs/remotes/origin/main");
assert_eq!(target.oid, ref_oid(&fixture.root, &target.full_ref));
```

Keep-branch tests assert `preflight.merge_target.is_none()`.

```bash
rustfmt --edition 2021 app/src/project_organization/git.rs app/src/project_organization/git_tests.rs
cargo test -p warp --lib project_organization::git_tests -- --nocapture
git add app/src/project_organization/git.rs app/src/project_organization/git_tests.rs
git commit -m "feat: capture deletion target snapshots"
```

### Task 4: Move authoritative deletion validation under ref locks

**Files:**
- Modify: `app/src/project_organization/git.rs:840-1140`
- Modify: `app/src/project_organization/git_tests.rs:1543-1951`

- [ ] **Step 1: Write failing lock-before-authority race tests**

Expose one test-only entrypoint around the production core:

```rust
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
    )
}
```

```rust
#[test]
fn locked_deletion_accepts_same_oid_recreation_before_prepare() {
    let fixture = GitFixture::new();
    let worktree = fixture.add_linked_worktree("feature/same-oid-current");
    let branch_ref = "refs/heads/feature/same-oid-current";
    let oid = ref_oid(&fixture.root, branch_ref);

    remove_workspace_with_transaction_hooks(
        &fixture.root,
        &worktree,
        "feature/same-oid-current",
        true,
        false,
        || {
            run_git(&fixture.root, &["update-ref", "-d", branch_ref, &oid]);
            run_git(&fixture.root, &["update-ref", branch_ref, &oid]);
        },
        || {},
    )
    .unwrap();

    assert!(!worktree.exists());
    assert!(!ref_exists(&fixture.root, branch_ref));
}

#[test]
fn locked_deletion_rejects_different_oid_before_prepare() {
    let fixture = GitFixture::new();
    let worktree = fixture.add_linked_worktree("feature/different-oid");
    let branch_ref = "refs/heads/feature/different-oid";
    let old_oid = ref_oid(&fixture.root, branch_ref);
    let changed_oid = fixture_commit_oid(&fixture.root, "changed branch");

    let error = remove_workspace_with_transaction_hooks(
        &fixture.root,
        &worktree,
        "feature/different-oid",
        true,
        false,
        || run_git(&fixture.root, &["update-ref", branch_ref, &changed_oid, &old_oid]),
        || {},
    )
    .unwrap_err();

    assert!(matches!(error, GitWorkspaceError::RefTransaction { .. }));
    assert!(worktree.exists());
    assert_eq!(ref_oid(&fixture.root, branch_ref), changed_oid);
}

#[test]
fn locked_deletion_rejects_target_drift_before_prepare() {
    let fixture = GitFixture::new();
    let worktree = fixture.add_linked_worktree("feature/target-drift");
    let error = remove_workspace_with_transaction_hooks(
        &fixture.root,
        &worktree,
        "feature/target-drift",
        true,
        false,
        || advance_remote_main(&fixture),
        || {},
    )
    .unwrap_err();

    assert!(matches!(error, GitWorkspaceError::RefTransaction { .. }));
    assert!(worktree.exists());
    assert!(ref_exists(&fixture.root, "refs/heads/feature/target-drift"));
}

#[test]
fn force_deletion_does_not_require_remote_metadata() {
    let fixture = GitFixture::new();
    run_git(&fixture.root, &["remote", "remove", "origin"]);
    let worktree = fixture.add_linked_worktree("feature/force-without-remote");

    remove_workspace(
        &fixture.root,
        &worktree,
        "feature/force-without-remote",
        true,
        true,
    )
    .unwrap();

    assert!(!worktree.exists());
    assert!(!ref_exists(
        &fixture.root,
        "refs/heads/feature/force-without-remote"
    ));
}
```

`fixture_commit_oid` uses `git commit-tree` with `HEAD` as its parent. `advance_remote_main` captures the current `refs/remotes/origin/main` OID, creates a new commit with `fixture_commit_oid`, and runs `git update-ref refs/remotes/origin/main <new-oid> <old-oid>`. Define each helper once in `git_tests.rs`.

- [ ] **Step 2: Run RED**

```bash
cargo test -p warp --lib locked_deletion_ -- --nocapture
```

Expected: missing transaction hook/core and current post-remove compare-delete behavior fails the new assertions.

- [ ] **Step 3: Implement candidate capture, prepare, and lock-internal validation**

Add the wrapper error:

```rust
#[error("failed to prepare branch deletion transaction: {source}")]
RefTransaction {
    #[source]
    source: RefTransactionError,
},
```

Production flow must be structurally equivalent to:

```rust
let candidate = capture_deletion_candidate(
    repository,
    worktree_path,
    delete_branch && !force_branch,
)?;
validate_requested_branch(branch, &candidate)?;

if !delete_branch {
    return remove_registered_worktree(repository, &candidate.worktree_path);
}

after_candidate();
let branch_ref = candidate.branch_ref.clone();
let locked_target = if force_branch {
    None
} else {
    let target = candidate.merge_target.as_ref().ok_or_else(|| {
        GitWorkspaceError::MissingMergeTarget {
            branch_ref: branch_ref.clone(),
        }
    })?;
    Some(LockedRef {
        full_ref: &target.full_ref,
        oid: &target.oid,
    })
};

let mut transaction = PreparedRefDelete::prepare(
    repository,
    &branch_ref,
    &candidate.branch_oid,
    locked_target,
)
.map_err(|source| GitWorkspaceError::RefTransaction { source })?;
after_prepared();

let registered_path = validate_locked_deletion(
    repository,
    worktree_path,
    branch,
    &candidate.branch_oid,
    candidate.merge_target.as_ref(),
    force_branch,
)?;
```

Add the unreachable-state error rather than using an empty sentinel:

```rust
#[error("branch deletion candidate for `{branch_ref}` has no merge target")]
MissingMergeTarget { branch_ref: String },
```

The public function calls one generic private core with no-op hooks. The test entrypoint uses the same core with its closures:

```rust
fn remove_workspace_inner<AfterCandidate, AfterPrepared>(
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
    let candidate = capture_deletion_candidate(
        repository,
        worktree_path,
        delete_branch && !force_branch,
    )?;
    validate_requested_branch(branch, &candidate)?;
    if !delete_branch {
        return remove_registered_worktree(repository, &candidate.worktree_path);
    }
    after_candidate();
    let locked_target = if force_branch {
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
        locked_target,
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
        Err(operation_error) => return abort_after_failed_operation(transaction, operation_error),
    };
    if let Err(operation_error) = remove_registered_worktree(repository, &registered_path) {
        return abort_after_failed_operation(transaction, operation_error);
    }
    transaction
        .commit()
        .map_err(|source| GitWorkspaceError::RefTransaction { source })
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
```

Define these concrete helpers before the core:

```rust
fn validate_requested_branch(
    requested_branch: &str,
    candidate: &DeletionPreflight,
) -> Result<(), GitWorkspaceError> {
    if candidate.branch == requested_branch {
        Ok(())
    } else {
        Err(GitWorkspaceError::WorktreeBranchMismatch {
            expected: requested_branch.to_string(),
            actual: candidate.branch.clone(),
        })
    }
}

fn remove_registered_worktree(
    repository: &Path,
    registered_path: &Path,
) -> Result<(), GitWorkspaceError> {
    let args = [
        OsStr::new("worktree"),
        OsStr::new("remove"),
        registered_path.as_os_str(),
    ];
    git_output_with_os_args_for_operation(repository, "remove worktree", &args)?;
    Ok(())
}

fn validate_locked_deletion(
    repository: &Path,
    requested_path: &Path,
    requested_branch: &str,
    expected_branch_oid: &str,
    expected_target: Option<&MergeTargetSnapshot>,
    force_branch: bool,
) -> Result<PathBuf, GitWorkspaceError> {
    let locked = capture_deletion_candidate(repository, requested_path, !force_branch)?;
    validate_requested_branch(requested_branch, &locked)?;
    if locked.branch_oid != expected_branch_oid {
        let actual_snapshot = direct_ref_snapshot(repository, &locked.branch_ref)?;
        return Err(GitWorkspaceError::BranchChanged {
            branch: locked.branch,
            expected_oid: expected_branch_oid.to_string(),
            actual_oid: actual_snapshot.direct_oid,
            actual_symbolic_target: actual_snapshot.symbolic_target,
        });
    }
    if !force_branch {
        let expected_target = expected_target.ok_or_else(|| {
            GitWorkspaceError::MissingMergeTarget {
                branch_ref: locked.branch_ref.clone(),
            }
        })?;
        let actual_target = locked.merge_target.ok_or_else(|| {
            GitWorkspaceError::MissingMergeTarget {
                branch_ref: locked.branch_ref.clone(),
            }
        })?;
        if actual_target.full_ref != expected_target.full_ref {
            return Err(GitWorkspaceError::MergeTargetChanged {
                expected: expected_target.full_ref.clone(),
                actual: actual_target.full_ref,
            });
        }
        if actual_target.oid != expected_target.oid {
            return Err(GitWorkspaceError::RefChanged {
                full_ref: expected_target.full_ref.clone(),
                expected_oid: expected_target.oid.clone(),
                actual_oid: actual_target.oid,
            });
        }
        if !actual_target.is_merged {
            return Err(GitWorkspaceError::BranchNotMerged {
                branch: locked.branch,
                merge_target: expected_target.full_ref.clone(),
            });
        }
    }
    Ok(locked.worktree_path)
}
```

`validate_locked_deletion` must rerun, in order:

1. canonicalize requested path and find exactly one matching `list_worktrees` record;
2. reject bare/detached/missing/non-local branch and requested branch mismatch;
3. require direct branch ref at expected OID with no symbolic target;
4. run `status --porcelain --untracked-files=all` and reject any output;
5. for non-force deletion, re-resolve target selection and require the same full ref;
6. resolve the locked target OID and require the expected OID;
7. run `merge-base --is-ancestor <branch-oid> <target-oid>` and return `BranchNotMerged` when false.

Do not reject based on the earlier `candidate.merge_target.is_merged`; that value is advisory only.

- [ ] **Step 4: Add prepared-state authority and abort tests**

```rust
#[test]
fn locked_deletion_aborts_when_worktree_becomes_dirty_after_prepare() {
    let fixture = GitFixture::new();
    let worktree = fixture.add_linked_worktree("feature/dirty-after-lock");
    let error = remove_workspace_with_transaction_hooks(
        &fixture.root,
        &worktree,
        "feature/dirty-after-lock",
        true,
        false,
        || {},
        || std::fs::write(worktree.join("keep.txt"), "keep").unwrap(),
    )
    .unwrap_err();

    assert!(matches!(error, GitWorkspaceError::DirtyWorktree { .. }));
    assert!(worktree.exists());
    assert!(ref_exists(&fixture.root, "refs/heads/feature/dirty-after-lock"));
}

#[test]
fn locked_deletion_aborts_when_target_selection_changes_after_prepare() {
    let fixture = GitFixture::new();
    let worktree = fixture.add_linked_worktree("feature/target-selection");
    run_git(&fixture.root, &["push", "origin", "main:refs/heads/other"]);
    run_git(&fixture.root, &["fetch", "origin"]);

    let error = remove_workspace_with_transaction_hooks(
        &fixture.root,
        &worktree,
        "feature/target-selection",
        true,
        false,
        || {},
        || {
            run_git(
                &fixture.root,
                &[
                    "branch",
                    "--set-upstream-to=origin/other",
                    "feature/target-selection",
                ],
            );
        },
    )
    .unwrap_err();

    assert!(matches!(error, GitWorkspaceError::MergeTargetChanged { .. }));
    assert!(worktree.exists());
    assert!(ref_exists(
        &fixture.root,
        "refs/heads/feature/target-selection"
    ));
}
```

Add:

```rust
#[error("merge target changed from `{expected}` to `{actual}` while branch deletion was locked")]
MergeTargetChanged { expected: String, actual: String },

#[error("ref `{full_ref}` changed while branch deletion was locked: expected {expected_oid}, found {actual_oid}")]
RefChanged {
    full_ref: String,
    expected_oid: String,
    actual_oid: String,
},
```

When any lock-internal validation or `worktree remove` fails, call `transaction.abort()`. If abort succeeds, return the original operation error. If abort fails, preserve the operation error as the primary source:

```rust
#[error("{operation_error}; aborting the branch deletion transaction also failed: {abort_error}")]
BranchDeleteAbortFailed {
    #[source]
    operation_error: Box<GitWorkspaceError>,
    abort_error: RefTransactionError,
},
```

- [ ] **Step 5: Run GREEN and commit the locked deletion path**

```bash
rustfmt --edition 2021 app/src/project_organization/git.rs app/src/project_organization/git_tests.rs
cargo test -p warp --lib locked_deletion_ -- --nocapture
cargo test -p warp --lib force_deletion_does_not_require_remote_metadata -- --nocapture
cargo test -p warp --lib project_organization::git_tests -- --nocapture
git add app/src/project_organization/git.rs app/src/project_organization/git_tests.rs
git commit -m "fix: validate worktree deletion under ref locks"
```

### Task 5: Preserve partial mutation and error sources

**Files:**
- Modify: `app/src/project_organization/git.rs:62-245, 840-1140`
- Modify: `app/src/project_organization/git_tests.rs:1666-1926`
- Modify: `specs/repository-workspaces/TECH.md`

- [ ] **Step 1: Write failing remove/commit/source-chain tests**

Add test-only hooks around the same private deletion core: a `remove_runner` closure replacing only `git worktree remove`, and an `after_remove` closure receiving `&mut PreparedRefDelete`. Add `#[cfg(test)] pub(super) fn terminate_for_test(&mut self)` to the transaction module so commit failure can be injected after real worktree removal.

Refactor the Task 4 core once, retaining the same production behavior:

```rust
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
```

After `validate_locked_deletion` succeeds, replace the Task 4 `remove_registered_worktree` call with:

```rust
if let Err(operation_error) = remove_runner(repository, &registered_path) {
    return abort_after_failed_operation(transaction, operation_error);
}
after_remove(&mut transaction);
```

Production calls the core with `remove_registered_worktree` and `|_| {}`. Add the test wrappers:

```rust
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
```

Update the Task 4 `remove_workspace_with_transaction_hooks` call to append `remove_registered_worktree` and `|_| {}` after `after_prepared`.

```rust
#[test]
fn worktree_remove_failure_aborts_and_preserves_branch() {
    let fixture = GitFixture::new();
    let worktree = fixture.add_linked_worktree("feature/remove-failure");
    let error = remove_workspace_with_remove_runner(
        &fixture.root,
        &worktree,
        "feature/remove-failure",
        true,
        false,
        |_repository, _path| Err(injected_command_error("remove worktree")),
    )
    .unwrap_err();

    assert!(matches!(error, GitWorkspaceError::CommandFailed { .. }));
    assert!(worktree.exists());
    assert!(ref_exists(&fixture.root, "refs/heads/feature/remove-failure"));
}

#[test]
fn commit_failure_reports_removed_worktree_partial_state() {
    let fixture = GitFixture::new();
    let worktree = fixture.add_linked_worktree("feature/commit-failure");
    let error = remove_workspace_with_after_remove_hook(
        &fixture.root,
        &worktree,
        "feature/commit-failure",
        true,
        false,
        |transaction| transaction.terminate_for_test(),
    )
    .unwrap_err();

    assert!(matches!(
        error,
        GitWorkspaceError::BranchDeleteTransactionFailed {
            worktree_removed: true,
            ..
        }
    ));
    assert!(!worktree.exists());
}

#[test]
fn wrapper_errors_expose_primary_sources() {
    use std::error::Error;

    let fixture = GitFixture::new();
    let target = fixture.tempdir.path().join("source chain target");
    let creation = create_from_remote_with_runner(
        &fixture.root,
        "refs/remotes/origin/main",
        "feature/source-chain",
        &target,
        |_repository, _remote_ref, _branch, _target, _expected_oid| {
            Err(injected_command_error("create worktree"))
        },
    )
    .unwrap_err();
    assert!(matches!(creation.source(), Some(source) if source.to_string().contains("create worktree")));
}
```

- [ ] **Step 2: Run RED**

```bash
cargo test -p warp --lib worktree_remove_failure_aborts_and_preserves_branch -- --nocapture
cargo test -p warp --lib commit_failure_reports_removed_worktree_partial_state -- --nocapture
cargo test -p warp --lib wrapper_errors_expose_primary_sources -- --nocapture
```

- [ ] **Step 3: Implement commit-failure context and source annotations**

Add:

```rust
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
```

After real `worktree remove`, run the test hook and call `transaction.commit()`. On failure, inspect `direct_ref_snapshot` only for diagnostics; inspection failure is secondary and must not replace the transaction source.

Replace the Task 4 commit mapping with:

```rust
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
    let (inspection_error, actual_snapshot) = match direct_ref_snapshot(repository, &branch_ref) {
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
                (snapshot.direct_oid.is_some() || snapshot.symbolic_target.is_some()).then(|| {
                    Box::new(GitWorkspaceError::BranchChanged {
                        branch: branch.to_string(),
                        expected_oid: branch_oid,
                        actual_oid: snapshot.direct_oid,
                        actual_symbolic_target: snapshot.symbolic_target,
                    })
                })
            })
        }),
    });
}
Ok(())
```

Add `#[source]` to the single primary wrapped error in `WorktreeCreationVerificationFailed`. Remove `#[source]` from secondary cleanup fields. For wrappers with operation plus cleanup failure, annotate the operation error, not cleanup.

- [ ] **Step 4: Remove superseded ownership and compare-delete code**

Delete these variants and helpers after all call sites are migrated:

```text
BranchClaimFailed
BranchCleanupFailed
BranchDeleteFailed
BranchDeleteNotCompleted
WorktreeCreationBranchChanged
WorktreeCreationCleanupFailed
cleanup_failed_remote_creation*
run_branch_compare_delete
remove_workspace_with_delete_runner
remove_workspace_with_inspection_runner
```

Keep `BranchChanged` because creation verification and lock-internal validation still use it. Keep `DirectRefSnapshot` for diagnostics. Every test-only hook remains `#[cfg(test)] pub(crate)` or `pub(super)`.

- [ ] **Step 5: Synchronize TECH.md**

Update the repository Git service and deletion sections with these exact semantics:

```text
- Local and remote worktree creation atomically claim the final target directory.
- Remote creation uses one `worktree add --no-track -b` command.
- Failed remote creation never automatically deletes a branch; it only removes an empty, unregistered claimed directory.
- Deletion preflight is advisory. Authoritative validation runs after a prepared ref transaction locks branch and merge target.
- The transaction queues target `verify` plus branch `delete <expected-oid>`; it never queues branch `verify` and `delete` together.
- Worktree remove failure aborts the transaction. Commit failure after worktree removal returns explicit partial state.
```

Remove stale OID-only cleanup/delete promises.

- [ ] **Step 6: Run focused verification and commit**

```bash
rustfmt --edition 2021 app/src/project_organization/git.rs app/src/project_organization/git_tests.rs app/src/project_organization/git/ref_transaction.rs app/src/project_organization/git/ref_transaction_tests.rs
cargo test -p warp --lib project_organization::git::ref_transaction::tests -- --nocapture
cargo test -p warp --lib project_organization::git_tests -- --nocapture
git add app/src/project_organization specs/repository-workspaces/TECH.md
git commit -m "fix: preserve worktree deletion partial state"
```

### Task 6: Final verification and review

**Files:**
- Verify: `app/src/project_organization/git.rs`
- Verify: `app/src/project_organization/git_tests.rs`
- Verify: `app/src/project_organization/git/ref_transaction.rs`
- Verify: `app/src/project_organization/git/ref_transaction_tests.rs`
- Verify: `docs/superpowers/specs/2026-07-13-worktree-git-ownership-design.md`
- Verify: `specs/repository-workspaces/TECH.md`

- [ ] **Step 1: Verify module and API boundaries**

```bash
rg -n 'std::process::Command|Command::new\("(?:sh|bash|zsh|cmd|powershell)' app/src/project_organization/git.rs app/src/project_organization/git
rg -n 'to_str\(|to_string_lossy' app/src/project_organization/git.rs app/src/project_organization/git
rg -n 'cleanup_failed_remote_creation|run_branch_compare_delete|BranchClaimFailed|BranchDeleteFailed|reflog' app/src/project_organization/git.rs app/src/project_organization/git_tests.rs app/src/project_organization/git
```

Expected: no direct process launcher, shell launcher, production lossy path conversion, obsolete ownership helper, or reflog generation implementation. `std::process::Child*` type names inside `ref_transaction.rs` are allowed; process creation must still use `command::blocking::Command`.

- [ ] **Step 2: Run fresh focused tests**

```bash
cargo test -p warp --lib project_organization::git::ref_transaction::tests -- --nocapture
cargo test -p warp --lib project_organization::git_tests -- --nocapture
```

Expected: both commands execute non-zero test counts and pass.

- [ ] **Step 3: Run build and diff hygiene checks**

```bash
cargo check -p warp
git diff --check 3e88797d..HEAD
git status --short
```

Expected: `cargo check` exits 0 with only known baseline warnings; diff check prints nothing; worktree is clean.

- [ ] **Step 4: Run fresh specification review**

Use `superpowers:requesting-code-review` with a fresh spec reviewer. Provide:

```text
Review commits 3e88797d..HEAD against:
- docs/superpowers/specs/2026-07-13-worktree-git-ownership-design.md
- specs/repository-workspaces/PRODUCT.md behaviors 15, 18, 29-33, 35-36
- specs/repository-workspaces/TECH.md Git creation/deletion sections
Check every required creation residual, prepared transaction, lock-internal validation, force-delete, partial mutation, and source-chain behavior.
```

If the reviewer reports findings, present all findings to the user and wait for confirmation before modifying code.

- [ ] **Step 5: Run fresh quality review**

After spec review passes, use a different fresh reviewer and request review of concurrency safety, child-process lifecycle, stderr draining, error sources, path handling, test determinism, and obsolete-code removal. If findings exist, present all findings to the user and wait for confirmation before fixes.

- [ ] **Step 6: Independently rerun verification after approved review fixes**

```bash
cargo test -p warp --lib project_organization::git::ref_transaction::tests -- --nocapture
cargo test -p warp --lib project_organization::git_tests -- --nocapture
cargo check -p warp
git diff --check 3e88797d..HEAD
git status --short
```

Do not claim completion unless all commands have fresh successful output and the worktree is clean. Commit approved review fixes with a scoped English message; do not create an empty commit.

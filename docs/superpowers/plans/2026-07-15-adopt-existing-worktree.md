# Adopt Existing Worktree Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 让用户从当前 repository 的已注册 linked worktree 中选择一个，将其安全注册为 workspace，而不创建或删除 Git worktree。

**Architecture:** Git 层将已有 `WorktreeInfo` 转换为可接入候选项，并在提交时重新校验 canonical path 与本地 branch。`CreateWorkspaceModal` 增加独立的 existing-worktree mode、picker 与加载错误状态。`Workspace` 并行加载 remote refs 和 worktree 候选项；adoption source 只执行只读验证，随后复用已有的 SQLite、tab 切换与 terminal 创建逻辑。

**Tech Stack:** Rust 2021、WarpUI `FilterableDropdown`、Git CLI `worktree list --porcelain`、Cargo tests。

---

## 文件结构

- 修改：`app/src/project_organization/git.rs`
  - 定义 worktree candidate 映射和不检查脏状态的 adoption validation。
- 修改：`app/src/project_organization/git_tests.rs`
  - 覆盖候选过滤和提交前注册/branch 校验。
- 修改：`app/src/project_organization/view/create_workspace_modal.rs`
  - 增加 existing-worktree mode、picker、路径只读渲染与 Retry event。
- 修改：`app/src/project_organization/view/create_workspace_modal_tests.rs`
  - 覆盖候选转换、request source 与 submit disable 策略。
- 修改：`app/src/workspace/view.rs`
  - 加载已有 worktree；只读验证 adoption source；避免持久化失败时删除用户已有 worktree。

### Task 1: 建立已有 worktree 候选与提交前校验

**Files:**
- Modify: `app/src/project_organization/git_tests.rs:480-555,2100-2210`
- Modify: `app/src/project_organization/git.rs:37-61,486-500,720-800`

- [ ] **Step 1: 编写候选映射与验证的失败测试**

在 `git_tests.rs` imports 中加入 `existing_worktree_options`、`validate_existing_worktree` 和 `WorktreeInfo`。新增：

```rust
#[test]
fn existing_worktree_options_exclude_primary_detached_and_prunable_worktrees() {
    let repository_root = PathBuf::from("/tmp/repository");
    let options = existing_worktree_options(
        &repository_root,
        [
            WorktreeInfo {
                path: repository_root.clone(),
                head: Some("a".to_string()),
                branch: Some("refs/heads/main".to_string()),
                is_bare: false,
                is_detached: false,
                is_locked: false,
                locked_reason: None,
                is_prunable: false,
                prunable_reason: None,
            },
            WorktreeInfo {
                path: PathBuf::from("/tmp/repository-feature"),
                head: Some("b".to_string()),
                branch: Some("refs/heads/feature/existing".to_string()),
                is_bare: false,
                is_detached: false,
                is_locked: false,
                locked_reason: None,
                is_prunable: false,
                prunable_reason: None,
            },
            WorktreeInfo {
                path: PathBuf::from("/tmp/repository-detached"),
                head: Some("c".to_string()),
                branch: None,
                is_bare: false,
                is_detached: true,
                is_locked: false,
                locked_reason: None,
                is_prunable: false,
                prunable_reason: None,
            },
            WorktreeInfo {
                path: PathBuf::from("/tmp/repository-prunable"),
                head: Some("d".to_string()),
                branch: Some("refs/heads/feature/prunable".to_string()),
                is_bare: false,
                is_detached: false,
                is_locked: false,
                locked_reason: None,
                is_prunable: true,
                prunable_reason: Some("missing".to_string()),
            },
        ],
    );

    assert_eq!(
        options,
        vec![ExistingWorktreeOption::new(
            PathBuf::from("/tmp/repository-feature"),
            "feature/existing",
        )],
    );
}

#[test]
fn validates_registered_existing_worktree_without_rejecting_dirty_contents() {
    let fixture = GitFixture::new();
    let worktree_path = fixture.add_linked_worktree("feature/adopt");
    std::fs::write(worktree_path.join("untracked.txt"), "dirty").unwrap();

    assert_eq!(
        validate_existing_worktree(&fixture.root, &worktree_path, "feature/adopt").unwrap(),
        worktree_path.canonicalize().unwrap(),
    );
}
```

- [ ] **Step 2: 运行测试并验证 RED**

Run:

```bash
cargo test -p warp existing_worktree_options_exclude_primary_detached_and_prunable_worktrees --lib -- --nocapture
cargo test -p warp validates_registered_existing_worktree_without_rejecting_dirty_contents --lib -- --nocapture
```

Expected: 编译失败，因为 `ExistingWorktreeOption`、`existing_worktree_options` 和 `validate_existing_worktree` 不存在。

- [ ] **Step 3: 实现候选映射与只读 validation**

在 `git.rs` 添加：

```rust
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExistingWorktreeOption {
    pub path: PathBuf,
    pub branch_name: String,
}

impl ExistingWorktreeOption {
    pub fn new(path: PathBuf, branch_name: impl Into<String>) -> Self {
        Self {
            path,
            branch_name: branch_name.into(),
        }
    }
}

pub fn existing_worktree_options(
    repository_root: &Path,
    worktrees: impl IntoIterator<Item = WorktreeInfo>,
) -> Vec<ExistingWorktreeOption> {
    let mut options = worktrees
        .into_iter()
        .filter_map(|worktree| {
            let branch_name = worktree.branch?.strip_prefix("refs/heads/")?.to_string();
            (!worktree.is_bare
                && !worktree.is_detached
                && !worktree.is_prunable
                && worktree.path != repository_root
                && !branch_name.is_empty())
                .then_some(ExistingWorktreeOption::new(worktree.path, branch_name))
        })
        .collect::<Vec<_>>();
    options.sort_by(|left, right| {
        (&left.branch_name, &left.path).cmp(&(&right.branch_name, &right.path))
    });
    options
}
```

新增 `validate_existing_worktree(repository, worktree_path, local_branch) -> Result<PathBuf, GitWorkspaceError>`：

1. `canonicalize(worktree_path)`。
2. 在 `GitWorkspaceError` 添加：

```rust
#[error("repository primary worktree `{path}` cannot be registered as a workspace")]
PrimaryWorktreeCannotBeWorkspace { path: PathBuf },
```

若 canonical path 等于 `canonicalize(repository)`，返回该 error。
3. 重新 `list_worktrees(repository)`，要求路径恰好出现一次，否则返回已有 `WorktreeNotFound` 或 `AmbiguousWorktree`。
4. 要求该项非 bare、非 detached，且 `branch == Some(format!("refs/heads/{local_branch}"))`；否则返回已有 `WorktreeBranchMismatch`。
5. 调用已有 `validate_ref_exists(repository, &expected_branch)`，返回 canonical path。

不要调用 `deletion_preflight`，因为 adoption 必须允许 dirty worktree。增加：

```rust
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
```

- [ ] **Step 4: 运行测试并验证 GREEN**

Run:

```bash
cargo test -p warp existing_worktree_options_exclude_primary_detached_and_prunable_worktrees --lib -- --nocapture
cargo test -p warp validates_registered_existing_worktree_without_rejecting_dirty_contents --lib -- --nocapture
```

Expected: 两项通过。候选只保留 linked local branch worktree，dirty linked worktree 可通过只读 adoption validation。

- [ ] **Step 5: 提交 Git 层改动**

```bash
git add app/src/project_organization/git.rs app/src/project_organization/git_tests.rs
git commit -m "feat: validate existing worktrees"
```

### Task 2: 在创建弹窗选择已有 worktree

**Files:**
- Modify: `app/src/project_organization/view/create_workspace_modal_tests.rs:1-190`
- Modify: `app/src/project_organization/view/create_workspace_modal.rs:20-920`

- [ ] **Step 1: 编写 source 和 submit 策略的失败测试**

在 modal tests imports 中加入 `ExistingWorktreeOption`。新增：

```rust
#[test]
fn existing_worktree_form_builds_a_workspace_creation_request() {
    let repository_id = RepositoryId(uuid::Uuid::from_u128(1));
    let workspace_id = RepositoryWorkspaceId(uuid::Uuid::from_u128(2));
    let mut form = CreateWorkspaceForm::new();
    form.set_mode(CreateWorkspaceMode::ExistingWorktree);
    form.set_existing_worktree_branch("feature/adopt".to_string());

    let request = form
        .build_request(
            repository_id,
            workspace_id,
            "feature/adopt".to_string(),
            PathBuf::from("/tmp/repository-adopt"),
        )
        .unwrap();

    assert!(matches!(
        request.source,
        CreateWorkspaceSource::ExistingWorktree { local_branch }
            if local_branch == "feature/adopt"
    ));
}

#[test]
fn existing_worktree_submit_is_disabled_until_a_selection_is_available() {
    assert!(submit_is_disabled(
        CreateWorkspaceMode::ExistingWorktree,
        false,
        false,
        false,
    ));
    assert!(!submit_is_disabled(
        CreateWorkspaceMode::ExistingWorktree,
        false,
        false,
        true,
    ));
}
```

- [ ] **Step 2: 运行测试并验证 RED**

Run:

```bash
cargo test -p warp existing_worktree_form_builds_a_workspace_creation_request --lib -- --nocapture
cargo test -p warp existing_worktree_submit_is_disabled_until_a_selection_is_available --lib -- --nocapture
```

Expected: 因 `ExistingWorktree` mode/source、form setter 或新的 submit 策略签名不存在而失败。

- [ ] **Step 3: 增加 mode、picker 与只读 path 渲染**

导入 `ExistingWorktreeOption`，然后：

1. 添加 `CreateWorkspaceMode::ExistingWorktree` 和 `CreateWorkspaceSource::ExistingWorktree { local_branch: String }`。
2. 在 `CreateWorkspaceForm` 添加 `existing_worktree_branch: Option<String>`、`set_existing_worktree_branch`，并让 mode 切换清理不兼容 selection；`can_submit` 对这个 mode 要求非空且不以 `refs/` 开头；`build_request` 生成 new source。
3. 在 action 加 `SelectExistingWorktree(ExistingWorktreeOption)` 与 `RetryExistingWorktrees`；event 加 `RetryExistingWorktrees { repository_id, workspace_id }`，由 `CreateWorkspaceTarget` production helper 构造。
4. 添加 `existing_worktree_picker`、`existing_worktree_mode_button`、`retry_existing_worktree_button`、`existing_worktree_options`、`selected_existing_worktree`、`existing_worktree_fetch_error`。所有 picker 继续使用 `set_top_bar_max_width(480.)` 和 `set_menu_width(480., ctx)`。
5. `configure` 调用 `begin_existing_worktree_fetch`，用禁用的 `Fetching existing worktrees...` 占位项重置 picker/error/selection。
6. `set_existing_worktrees` 使用 `existing_worktree_options(repository_root, worktrees)` 构造 `DropdownItem`，成功后启用 picker、缓存第一项但只在 existing mode 激活时应用它；`set_existing_worktree_fetch_error` 禁用 picker并保存 error。
7. `select_existing_worktree` 设置 form branch、保存完整 option、将 `Workspace name` reset 为 branch name。不要写入 `worktree_path_editor`。
8. `set_mode` 的 existing 分支应用缓存的当前 option；`try_submit` 从 option 直接取得 canonical `PathBuf`，remote/local 才继续读取 `worktree_path_editor`。existing mode 没有 option 时显示 `Select an existing worktree before creating a workspace.`。
9. 把 `submit_is_disabled` 扩展为四个参数 `(mode, has_remote_fetch_error, has_existing_worktree_fetch_error, has_existing_worktree_selection)`；remote 保持已有行为，existing mode 在加载、错误或无选择时禁用，local 保持可用。所有 selection/error/mode transition 后调用 `sync_submit_button_disabled_state`。
10. 渲染第三个 mode button。existing mode section 渲染 picker、其 error/Retry，并把 Worktree path section 替换为受 480px 约束的只读 `Text`，值为选项 path 的 `to_string_lossy()`；remote/local 继续显示现有 editor。不要用 `Clipped` 包裹 picker。

同时把现有 `remote_fetch_error_disables_submit_only_in_remote_mode` 测试的两次调用补齐为四个参数，existing fetch error 与 selection 参数都传 `false`，确保 remote 既有禁用行为仍被断言。

- [ ] **Step 4: 运行测试并验证 GREEN**

Run:

```bash
cargo test -p warp existing_worktree_form_builds_a_workspace_creation_request --lib -- --nocapture
cargo test -p warp existing_worktree_submit_is_disabled_until_a_selection_is_available --lib -- --nocapture
cargo test -p warp create_workspace_modal --lib -- --nocapture
```

Expected: 新 source 能构建请求；existing mode 仅在有合法 selection 时启用 Create；现有 modal tests 全部通过。

- [ ] **Step 5: 提交 modal 改动**

```bash
git add app/src/project_organization/view/create_workspace_modal.rs app/src/project_organization/view/create_workspace_modal_tests.rs
git commit -m "feat: select existing worktrees"
```

### Task 3: 协调加载并安全接入已有 worktree

**Files:**
- Modify: `app/src/workspace/view.rs:75-85,5630-5770,5950-6060`
- Modify: `app/src/workspace/view_test.rs`

- [ ] **Step 1: 编写 adoption cleanup 策略的失败测试**

在现有同模块测试文件 `app/src/workspace/view_test.rs` 中新增纯 helper 的测试：

```rust
#[test]
fn existing_worktree_source_never_requests_git_cleanup() {
    assert!(!source_creates_worktree(&CreateWorkspaceSource::ExistingWorktree {
        local_branch: "feature/adopt".to_string(),
    }));
    assert!(source_creates_worktree(&CreateWorkspaceSource::ExistingLocalBranch {
        local_branch: "feature/create".to_string(),
    }));
}
```

- [ ] **Step 2: 运行测试并验证 RED**

Run:

```bash
cargo test -p warp existing_worktree_source_never_requests_git_cleanup --lib -- --nocapture
```

Expected: 因 `source_creates_worktree` 或新 source variant 不存在而失败。

- [ ] **Step 3: 加载候选、Retry 与 adoption validation**

在 `workspace/view.rs` imports 中增加 `list_worktrees_async` 与 `validate_existing_worktree_async`。新增：

```rust
fn fetch_existing_worktrees(
    &mut self,
    repository_id: RepositoryId,
    workspace_id: RepositoryWorkspaceId,
    repository_path: PathBuf,
    ctx: &mut ViewContext<Self>,
) {
    self.create_workspace_modal.view.update(ctx, |modal, ctx| {
        modal.body().update(ctx, |body, ctx| {
            if body.matches_target(repository_id, workspace_id) {
                body.begin_existing_worktree_fetch(ctx);
            }
        });
    });
    ctx.spawn(list_worktrees_async(repository_path), move |workspace, result, ctx| {
        workspace.create_workspace_modal.view.update(ctx, |modal, ctx| {
            modal.body().update(ctx, |body, ctx| {
                if !body.matches_target(repository_id, workspace_id) {
                    return;
                }
                match result {
                    Ok(worktrees) => body.set_existing_worktrees(worktrees, ctx),
                    Err(error) => body.set_existing_worktree_fetch_error(
                        format!("Failed to list existing worktrees: {error}"),
                        ctx,
                    ),
                }
            });
        });
    });
}
```

`open_create_workspace_modal` 在 existing modal 打开后并行调用这个 helper 与 `fetch_create_workspace_branch_refs`。`handle_create_workspace_modal_body_event` 对 RetryExistingWorktrees 查找 repository 后重新调用 helper。所有 callback 保留已有 `(repository_id, workspace_id)` guard。

新增纯 helper：

```rust
fn source_creates_worktree(source: &CreateWorkspaceSource) -> bool {
    !matches!(source, CreateWorkspaceSource::ExistingWorktree { .. })
}
```

在 `create_repository_workspace`：

1. 新 source 从 `local_branch` 生成 record branch，`delete_branch_on_cleanup` 保持 `false`。
2. Git operation 对 new source 调用：

```rust
CreateWorkspaceSource::ExistingWorktree { local_branch } => {
    validate_existing_worktree_async(repository_path, worktree_path, local_branch)
        .await
        .map(|_| ())
}
```

不调用 create functions。
3. 在 spawn 前计算：

```rust
let should_cleanup_on_persistence_failure = source_creates_worktree(&request.source);
```

持久化失败时，只有这个值为 `true` 才调用 `remove_workspace_async`；否则直接显示 `Failed to save workspace: {error}`，不删除或修改用户 worktree。
4. validation 成功后继续既有 record 持久化、`switch_repository_workspace` 和 terminal tab 创建。

- [ ] **Step 4: 运行测试并验证 GREEN**

Run:

```bash
cargo test -p warp existing_worktree_source_never_requests_git_cleanup --lib -- --nocapture
cargo test -p warp create_workspace_modal --lib -- --nocapture
cargo test -p warp validates_registered_existing_worktree_without_rejecting_dirty_contents --lib -- --nocapture
cargo check -p warp
```

Expected: adoption source 从不触发 cleanup；modal 与 Git validation 回归测试通过；crate 编译成功。

- [ ] **Step 5: 提交 Workspace 协调改动**

```bash
git add app/src/workspace/view.rs app/src/workspace/view_test.rs
git commit -m "feat: adopt existing worktrees"
```

### Task 4: 复核与人工验证构建

**Files:**
- Modify: `app/src/project_organization/git.rs`
- Modify: `app/src/project_organization/git_tests.rs`
- Modify: `app/src/project_organization/view/create_workspace_modal.rs`
- Modify: `app/src/project_organization/view/create_workspace_modal_tests.rs`
- Modify: `app/src/workspace/view.rs`
- Modify: `app/src/workspace/view_test.rs`

- [ ] **Step 1: 运行格式、diff 和相关测试**

Run:

```bash
cargo test -p warp existing_worktree_options_exclude_primary_detached_and_prunable_worktrees --lib -- --nocapture
cargo test -p warp validates_registered_existing_worktree_without_rejecting_dirty_contents --lib -- --nocapture
cargo test -p warp create_workspace_modal --lib -- --nocapture
cargo test -p warp existing_worktree_source_never_requests_git_cleanup --lib -- --nocapture
rustfmt --check app/src/project_organization/git.rs app/src/project_organization/git_tests.rs app/src/project_organization/view/create_workspace_modal.rs app/src/project_organization/view/create_workspace_modal_tests.rs
git diff --check
cargo check -p warp
```

Expected: 全部 exit code 为 0。若 `app/src/workspace/view.rs` 全文件 format check 被既有无关差异阻断，报告首个差异，不格式化整份文件。

- [ ] **Step 2: 生成 macOS 人工验证 bundle**

Run:

```bash
./script/run --dont-open
codesign --verify --deep --strict --verbose=2 target/debug/bundle/osx/Zap.app
```

Expected: `Zap.app` 生成且签名有效。手工验证：在带至少一个 linked worktree 的 repository 打开 Create workspace；切换到 existing mode；主目录和 detached worktree 不出现；选择 linked worktree 后 name 是 branch、path 只读；Create 后 Git worktree 数量不变且 terminal 位于已选路径；将该 worktree 切换 branch 或删除后再创建，操作失败且不会产生 SQLite record；worktree 列表失败时 remote/local mode 仍可用且 Retry 可恢复。

- [ ] **Step 3: 提交最终验证改动**

```bash
git status --short
```

Expected: 不存在未提交的 implementation 文件；若只剩已提交规格/计划文档则无需额外 commit。建议最终 commit message：

```text
feat: adopt existing worktrees as workspaces
```

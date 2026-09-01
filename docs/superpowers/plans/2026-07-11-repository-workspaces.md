# Repository Workspaces Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在 Dogfood Feature Flag 下实现 repository → workspace 双层项目组织、独立 Git worktree、workspace 级完整页签集合和安全删除流程。

**Architecture:** 新增 `app/src/project_organization/` 领域模块和 SQLite 表；Git 操作集中到结构化服务。窗口根保留当前 `tabs: Vec<TabData>` 作为活动 workspace 页签，通过活动/非活动集合整体交换复用现有 Tab、PaneGroup 和 Terminal 能力。

**Tech Stack:** Rust 2021、WarpUI Entity/View、Diesel + SQLite、`crates/command`、现有 `crates/integration` Builder/TestStep、Cargo nextest。

---

## Execution Preconditions

- 执行前阅读并遵循 `specs/repository-workspaces/PRODUCT.md` 与 `TECH.md`。
- 使用 `superpowers:using-git-worktrees` 在 `.worktrees/repository-workspaces` 创建隔离 worktree；若当前已经是 linked worktree，则继续使用当前隔离环境。
- 基线验证运行 `cargo check`。若失败，记录原始失败并在获得用户确认前不继续实现。
- UI 任务开始前重新读取 `warp-ui-guidelines`；逻辑任务使用 `superpowers:test-driven-development`；Feature Flag 使用 `add-feature-flag`；单测使用 `rust-unit-tests`；集成测试使用 `warp-integration-test`。

### Task 1: Add the Dogfood feature flag

**Files:**
- Modify: `crates/warp_features/src/lib.rs`
- Create: `crates/warp_features/src/lib_tests.rs`

- [ ] **Step 1: Write the failing flag placement test**

```rust
use super::{FeatureFlag, DOGFOOD_FLAGS, PREVIEW_FLAGS, RELEASE_FLAGS};

#[test]
fn repository_workspaces_is_dogfood_only() {
    assert!(DOGFOOD_FLAGS.contains(&FeatureFlag::RepositoryWorkspaces));
    assert!(!PREVIEW_FLAGS.contains(&FeatureFlag::RepositoryWorkspaces));
    assert!(!RELEASE_FLAGS.contains(&FeatureFlag::RepositoryWorkspaces));
}
```

在 `lib.rs` 末尾注册：

```rust
#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
```

- [ ] **Step 2: Run the test and verify RED**

Run: `cargo test -p warp_features repository_workspaces_is_dogfood_only`

Expected: FAIL，因为 `FeatureFlag::RepositoryWorkspaces` 尚不存在。

- [ ] **Step 3: Add the enum variant and Dogfood placement**

在 `FeatureFlag` 中按现有字母/主题顺序加入：

```rust
RepositoryWorkspaces,
```

在 `DOGFOOD_FLAGS` 中加入：

```rust
FeatureFlag::RepositoryWorkspaces,
```

- [ ] **Step 4: Run the test and check the crate**

Run:

```bash
cargo test -p warp_features repository_workspaces_is_dogfood_only
cargo check -p warp_features
```

Expected: PASS，且无 warning。

- [ ] **Step 5: Commit**

```bash
git add crates/warp_features/src/lib.rs crates/warp_features/src/lib_tests.rs
git commit -m "feat: add repository workspaces flag"
```

### Task 2: Add persistence schema and row models

**Files:**
- Create: `crates/persistence/migrations/2026-07-11-000000_add_repository_workspaces/up.sql`
- Create: `crates/persistence/migrations/2026-07-11-000000_add_repository_workspaces/down.sql`
- Modify: `crates/persistence/src/model.rs`
- Modify: `crates/persistence/src/schema.rs` (generated)
- Modify: `app/src/persistence/mod.rs`
- Modify: `app/src/persistence/sqlite.rs`
- Modify: `app/src/persistence/sqlite_tests.rs`

- [ ] **Step 1: Add a failing repository row round-trip test**

扩展 `sqlite_tests.rs`，断言 repository row 可通过新的 SQLite helper round-trip：

```rust
#[test]
fn repository_rows_round_trip() {
    let now = chrono::Utc::now().naive_utc();
    let repository = model::Repository {
        id: uuid::Uuid::from_u128(7).to_string(),
        display_name: "zap".to_string(),
        path: "/tmp/zap".to_string(),
        remote_url: None,
        source: "local".to_string(),
        created_at: now,
        last_opened_at: now,
    };
    let tempdir = tempfile::tempdir().expect("tempdir should be created");
    let database_path = tempdir.path().join("warp.sqlite");
    let mut conn = setup_database(&database_path).expect("database should initialize");
    save_repository(&mut conn, repository.clone()).expect("repository should save");
    assert_eq!(get_all_repositories(&mut conn).unwrap(), vec![repository]);
}
```

- [ ] **Step 2: Run the test and verify RED**

Run: `cargo test -p warp --lib repository_rows_round_trip`

Expected: FAIL，因为 repository 表和 row API 尚不存在。

- [ ] **Step 3: Create the migration**

`up.sql` 使用以下结构：

```sql
CREATE TABLE repositories (
    id TEXT PRIMARY KEY NOT NULL,
    display_name TEXT NOT NULL,
    path TEXT NOT NULL UNIQUE,
    remote_url TEXT,
    source TEXT NOT NULL CHECK (source IN ('local', 'cloned')),
    created_at TIMESTAMP NOT NULL,
    last_opened_at TIMESTAMP NOT NULL
);

INSERT INTO repositories (id, display_name, path, source, created_at, last_opened_at)
SELECT lower(hex(randomblob(4))) || '-' || lower(hex(randomblob(2))) || '-4' ||
       substr(lower(hex(randomblob(2))), 2) || '-' ||
       substr('89ab', abs(random()) % 4 + 1, 1) ||
       substr(lower(hex(randomblob(2))), 2) || '-' || lower(hex(randomblob(6))),
       path,
       path,
       'local',
       added_ts,
       coalesce(last_opened_ts, added_ts)
FROM projects;

CREATE TABLE repository_workspaces (
    id TEXT PRIMARY KEY NOT NULL,
    repository_id TEXT NOT NULL REFERENCES repositories(id) ON DELETE RESTRICT,
    display_name TEXT NOT NULL,
    branch TEXT NOT NULL,
    worktree_path TEXT NOT NULL UNIQUE,
    created_at TIMESTAMP NOT NULL,
    last_opened_at TIMESTAMP NOT NULL,
    UNIQUE(repository_id, branch)
);

CREATE TABLE repository_workspace_window_states (
    window_id INTEGER NOT NULL REFERENCES windows(id) ON DELETE CASCADE,
    repository_workspace_id TEXT NOT NULL REFERENCES repository_workspaces(id) ON DELETE CASCADE,
    active_tab_index INTEGER NOT NULL,
    PRIMARY KEY(window_id, repository_workspace_id)
);

ALTER TABLE tabs ADD COLUMN repository_workspace_id TEXT
    REFERENCES repository_workspaces(id) ON DELETE SET NULL;
ALTER TABLE windows ADD COLUMN active_repository_workspace_id TEXT
    REFERENCES repository_workspaces(id) ON DELETE SET NULL;
```

首次加载时，Rust 模型把 `display_name == path` 的迁移行更新为目录 basename。第一条 migration 暂时保留 `projects`，让旧模型和新表在迁移提交中同时可编译。`down.sql` 使用项目 SQLite 版本支持的 `DROP COLUMN`：

```sql
ALTER TABLE tabs DROP COLUMN repository_workspace_id;
ALTER TABLE windows DROP COLUMN active_repository_workspace_id;
DROP TABLE repository_workspace_window_states;
DROP TABLE repository_workspaces;
DROP TABLE repositories;
```

- [ ] **Step 4: Add Diesel row types and persistence events**

在 `crates/persistence/src/model.rs` 添加：

```rust
#[derive(Clone, Debug, Eq, Identifiable, Insertable, PartialEq, Queryable, AsChangeset)]
#[diesel(table_name = repositories)]
pub struct Repository {
    pub id: String,
    pub display_name: String,
    pub path: String,
    pub remote_url: Option<String>,
    pub source: String,
    pub created_at: NaiveDateTime,
    pub last_opened_at: NaiveDateTime,
}

#[derive(Clone, Debug, Eq, Identifiable, Insertable, PartialEq, Queryable, AsChangeset)]
#[diesel(table_name = repository_workspaces)]
pub struct RepositoryWorkspace {
    pub id: String,
    pub repository_id: String,
    pub display_name: String,
    pub branch: String,
    pub worktree_path: String,
    pub created_at: NaiveDateTime,
    pub last_opened_at: NaiveDateTime,
}
```

在 `ModelEvent` 添加显式 CRUD 变体，不使用通用 JSON：

```rust
UpsertRepository { repository: model::Repository },
DeleteRepository { repository_id: String },
UpsertRepositoryWorkspace { workspace: model::RepositoryWorkspace },
DeleteRepositoryWorkspace { workspace_id: String },
```

- [ ] **Step 5: Regenerate schema and implement SQLite handlers**

Run:

```bash
tmp_db="$(mktemp -t zap-repository-workspaces.XXXXXX.db)"
DATABASE_URL="$tmp_db" diesel migration run
DATABASE_URL="$tmp_db" diesel print-schema
rm -f "$tmp_db"
```

实现 `save_repository`、`delete_repository`、`save_repository_workspace`、`delete_repository_workspace`，并在加载结构中返回两类行。

- [ ] **Step 6: Run persistence tests and check generated diff**

Run:

```bash
cargo test -p warp --lib repository_rows_round_trip
cargo test -p warp --lib persistence::sqlite_tests
git diff --check
```

Expected: PASS；`schema.rs` 只包含 migration 对应的生成变化。

- [ ] **Step 7: Commit**

```bash
git add crates/persistence app/src/persistence
git commit -m "feat: persist repository workspaces"
```

### Task 3: Add domain types and repository model

**Files:**
- Create: `app/src/project_organization/mod.rs`
- Create: `app/src/project_organization/domain.rs`
- Create: `app/src/project_organization/model.rs`
- Create: `app/src/project_organization/model_tests.rs`
- Modify: `app/src/lib.rs`
- Modify: `app/src/search/command_search/projects/project_data_source.rs`
- Modify: `app/src/pane_group/pane/welcome_view.rs`
- Modify: `app/src/terminal/view.rs`
- Create: `crates/persistence/migrations/2026-07-11-010000_drop_legacy_projects/up.sql`
- Create: `crates/persistence/migrations/2026-07-11-010000_drop_legacy_projects/down.sql`
- Modify: `crates/persistence/src/model.rs`
- Modify: `crates/persistence/src/schema.rs` (generated)
- Modify: `app/src/persistence/mod.rs`
- Modify: `app/src/persistence/sqlite.rs`

- [ ] **Step 1: Write failing CRUD and uniqueness tests**

```rust
#[test]
fn repository_paths_and_workspace_branches_are_unique() {
    let tempdir = tempfile::tempdir().unwrap();
    let mut model = ProjectOrganizationModel::new(Vec::new(), Vec::new(), None);
    let repository = model.add_local_repository(tempdir.path().to_path_buf()).unwrap();
    assert!(matches!(
        model.add_local_repository(tempdir.path().to_path_buf()),
        Err(ProjectOrganizationError::RepositoryAlreadyExists { .. })
    ));

    let workspace = RepositoryWorkspace {
        id: RepositoryWorkspaceId(uuid::Uuid::from_u128(2)),
        repository_id: repository.id,
        display_name: "feature/a".to_string(),
        branch: "feature/a".to_string(),
        worktree_path: tempdir.path().join("worktree-a"),
        created_at: chrono::Utc::now().naive_utc(),
        last_opened_at: chrono::Utc::now().naive_utc(),
    };
    model.insert_workspace(workspace.clone()).unwrap();
    assert!(matches!(
        model.insert_workspace(RepositoryWorkspace {
            id: RepositoryWorkspaceId(uuid::Uuid::from_u128(3)),
            ..workspace
        }),
        Err(ProjectOrganizationError::WorkspaceBranchAlreadyExists { .. })
    ));
}
```

- [ ] **Step 2: Run and verify RED**

Run: `cargo test -p warp --lib project_organization::model_tests`

Expected: FAIL，模块不存在。

- [ ] **Step 3: Implement stable IDs, entities, and errors**

```rust
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct RepositoryId(pub uuid::Uuid);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct RepositoryWorkspaceId(pub uuid::Uuid);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RepositorySource {
    Local,
    Cloned,
}

#[derive(Debug, thiserror::Error)]
pub enum ProjectOrganizationError {
    #[error("repository already exists at {path}")]
    RepositoryAlreadyExists { path: PathBuf },
    #[error("branch {branch} already has a workspace")]
    WorkspaceBranchAlreadyExists { branch: String },
    #[error("repository {repository_id:?} still has workspaces")]
    RepositoryHasWorkspaces { repository_id: RepositoryId },
    #[error(transparent)]
    Persistence(#[from] anyhow::Error),
}
```

- [ ] **Step 4: Implement `ProjectOrganizationModel` and replace project consumers**

模型使用 canonical path 索引 repository，使用 `(RepositoryId, branch)` 索引 workspace，并通过明确事件通知 UI：

```rust
pub enum ProjectOrganizationEvent {
    RepositoryAdded(RepositoryId),
    RepositoryUpdated(RepositoryId),
    RepositoryRemoved(RepositoryId),
    WorkspaceAdded(RepositoryWorkspaceId),
    WorkspaceUpdated(RepositoryWorkspaceId),
    WorkspaceRemoved(RepositoryWorkspaceId),
}
```

将搜索、欢迎页和终端的 `ProjectManagementModel::upsert_project` 调用改为新模型的 `touch_repository_path`。Flag 关闭时这些消费者仍读取新 repository 数据，不保留双写旧 projects 表。

所有消费者切换后添加第二条 migration：

```sql
-- up.sql
DROP TABLE projects;

-- down.sql
CREATE TABLE projects (
    path TEXT NOT NULL PRIMARY KEY,
    added_ts DATETIME NOT NULL,
    last_opened_ts DATETIME
);
INSERT INTO projects (path, added_ts, last_opened_ts)
SELECT path, created_at, last_opened_at FROM repositories;
```

删除 `model::Project`、`ModelEvent::UpsertProject/DeleteProject` 和旧 SQLite handlers，然后重新运行 `diesel print-schema`。

- [ ] **Step 5: Run tests and check app**

Run:

```bash
tmp_db="$(mktemp -t zap-repository-workspaces.XXXXXX.db)"
DATABASE_URL="$tmp_db" diesel migration run
DATABASE_URL="$tmp_db" diesel print-schema
rm -f "$tmp_db"
cargo test -p warp --lib project_organization::model_tests
cargo check -p warp
```

- [ ] **Step 6: Commit**

```bash
git add app/src/project_organization app/src/lib.rs app/src/search app/src/pane_group/pane/welcome_view.rs app/src/terminal/view.rs app/src/persistence crates/persistence
git commit -m "feat: add repository workspace model"
```

### Task 4: Implement repository validation, clone, and ref discovery

**Files:**
- Create: `app/src/project_organization/git.rs`
- Create: `app/src/project_organization/git_tests.rs`
- Modify: `app/src/project_organization/mod.rs`

- [ ] **Step 1: Write failing tests using temporary Git repositories**

```rust
struct GitFixture {
    tempdir: tempfile::TempDir,
    root: PathBuf,
}

impl GitFixture {
    fn new() -> Self {
        let tempdir = tempfile::tempdir().unwrap();
        let root = tempdir.path().join("repo");
        std::fs::create_dir(&root).unwrap();
        run_git(&root, &["init", "-b", "main"]);
        std::fs::write(root.join("README.md"), "fixture").unwrap();
        run_git(&root, &["add", "README.md"]);
        run_git(&root, &["-c", "user.name=Zap Tests", "-c", "user.email=zap@example.com", "commit", "-m", "init"]);
        Self { tempdir, root }
    }

    fn add_linked_worktree(&self, branch: &str) -> PathBuf {
        let path = self.tempdir.path().join(branch.replace('/', "-"));
        run_git(&self.root, &["worktree", "add", "-b", branch, path.to_str().unwrap()]);
        path
    }
}

fn run_git(cwd: &Path, args: &[&str]) {
    let status = command::blocking::Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args(args)
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?} failed");
}

#[test]
fn rejects_linked_worktree_as_repository() {
    let fixture = GitFixture::new();
    let worktree_path = fixture.add_linked_worktree("feature/a");
    let error = validate_repository(&worktree_path).unwrap_err();
    assert!(matches!(error, GitWorkspaceError::LinkedWorktree { .. }));
}

#[test]
fn classifies_local_and_remote_refs_without_prefix_guessing() {
    let fixture = GitFixture::new();
    run_git(&fixture.root, &["branch", "origin/foo"]);
    let refs = list_branch_refs(&fixture.root).unwrap();
    assert!(refs.iter().any(|r| matches!(r, BranchRef::Local { name } if name == "origin/foo")));
}
```

- [ ] **Step 2: Run and verify RED**

Run: `cargo test -p warp --lib project_organization::git_tests`

- [ ] **Step 3: Implement typed command execution and errors**

```rust
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BranchRef {
    Local { name: String, full_ref: String },
    Remote { remote: String, name: String, full_ref: String },
}

fn git_output(repo: &Path, args: &[&str]) -> Result<Output, GitWorkspaceError> {
    let output = command::blocking::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()?;
    if output.status.success() {
        Ok(output)
    } else {
        Err(GitWorkspaceError::CommandFailed {
            args: args.iter().map(|arg| (*arg).to_string()).collect(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        })
    }
}
```

实现 `validate_repository`、`clone_repository`、`fetch_and_list_refs`、`list_worktrees` 和默认分支解析。异步 UI API 包装 blocking 实现，不在 UI 线程执行 Git。

- [ ] **Step 4: Add clone cleanup and safe path tests**

覆盖已存在目标目录不删除、本次创建空目录失败后清理、带空格/引号路径、URL repository 名解析，以及 safe branch slug：

```rust
assert_eq!(workspace_dir_name("feature/a b", "12345678"), "feature-a-b-12345678");
```

- [ ] **Step 5: Run tests**

Run:

```bash
cargo test -p warp --lib project_organization::git_tests
cargo check -p warp
```

- [ ] **Step 6: Commit**

```bash
git add app/src/project_organization
git commit -m "feat: add repository git service"
```

### Task 5: Implement worktree creation and deletion preflight

**Files:**
- Modify: `app/src/project_organization/git.rs`
- Modify: `app/src/project_organization/git_tests.rs`

- [ ] **Step 1: Write failing creation tests**

```rust
#[test]
fn creates_new_branch_from_remote_ref_without_tracking() {
    let fixture = GitFixture::new();
    let bare = fixture.tempdir.path().join("remote.git");
    run_git(fixture.tempdir.path(), &["init", "--bare", bare.to_str().unwrap()]);
    run_git(&fixture.root, &["remote", "add", "origin", bare.to_str().unwrap()]);
    run_git(&fixture.root, &["push", "-u", "origin", "main"]);
    run_git(&fixture.root, &["fetch", "origin"]);
    let path = fixture.tempdir.path().join("worktree");
    create_from_remote(&fixture.root, "refs/remotes/origin/main", "feature/a", &path).unwrap();
    assert_eq!(current_branch(&path).unwrap(), "feature/a");
    assert_eq!(branch_upstream(&fixture.root, "feature/a").unwrap(), None);
}

#[test]
fn reports_the_path_that_already_checks_out_a_local_branch() {
    let fixture = GitFixture::new();
    let occupied = fixture.add_linked_worktree("feature/a");
    let error = create_from_local(&fixture.root, "feature/a", fixture.tempdir.path().join("second"))
        .unwrap_err();
    assert!(matches!(
        error,
        GitWorkspaceError::BranchAlreadyCheckedOut { path, .. } if path == occupied
    ));
}
```

- [ ] **Step 2: Verify RED, then implement minimal creation APIs**

```rust
pub fn create_from_remote(
    repository: &Path,
    remote_ref: &str,
    new_branch: &str,
    worktree_path: &Path,
) -> Result<(), GitWorkspaceError>;

pub fn create_from_local(
    repository: &Path,
    local_branch: &str,
    worktree_path: &Path,
) -> Result<(), GitWorkspaceError>;
```

Commands must be argument arrays equivalent to:

```text
git -C <repo> worktree add --no-track -b <new> <path> <remote-ref>
git -C <repo> worktree add <path> refs/heads/<branch>
```

- [ ] **Step 3: Write failing deletion safety tests**

```rust
#[test]
fn dirty_worktree_blocks_deletion_before_mutation() {
    let fixture = GitFixture::new();
    let worktree_path = fixture.add_linked_worktree("feature/a");
    std::fs::write(worktree_path.join("dirty.txt"), "dirty").unwrap();
    assert!(matches!(
        deletion_preflight(&fixture.root, &worktree_path, true),
        Err(GitWorkspaceError::DirtyWorktree { .. })
    ));
    assert!(worktree_path.exists());
}
```

- [ ] **Step 4: Implement preflight and delete APIs**

```rust
pub struct DeletionPreflight {
    pub branch: String,
    pub is_merged: bool,
    pub merge_target: String,
}

pub fn remove_workspace(
    repository: &Path,
    worktree_path: &Path,
    branch: &str,
    delete_branch: bool,
    force_branch: bool,
) -> Result<(), GitWorkspaceError>;
```

先执行 `status --porcelain`、worktree/branch 一致性和 merge-base 检查；只有调用 `remove_workspace` 时才执行 `worktree remove` 与 `branch -d/-D`。

- [ ] **Step 5: Run tests and commit**

```bash
cargo test -p warp --lib project_organization::git_tests
git add app/src/project_organization/git.rs app/src/project_organization/git_tests.rs
git commit -m "feat: manage repository worktrees safely"
```

### Task 6: Persist workspace-scoped Tab snapshots

**Files:**
- Modify: `app/src/app_state.rs`
- Modify: `app/src/tab.rs`
- Modify: `app/src/workspace/view.rs`
- Modify: `app/src/persistence/sqlite.rs`
- Modify: `app/src/persistence/sqlite_tests.rs`
- Modify: `app/src/launch_configs/launch_config.rs`
- Modify: `app/src/launch_configs/launch_config_tests.rs`

- [ ] **Step 1: Add failing snapshot grouping tests**

```rust
#[test]
fn repository_workspace_state_round_trips() {
    let workspace_id = RepositoryWorkspaceId(uuid::Uuid::from_u128(7));
    let mut window = test_terminal_window_snapshot(false);
    window.active_repository_workspace_id = Some(workspace_id);
    window.repository_workspace_states = vec![RepositoryWorkspaceWindowStateSnapshot {
        repository_workspace_id: workspace_id,
        active_tab_index: 0,
    }];
    window.tabs[0].repository_workspace_id = Some(workspace_id);

    let state = AppState {
        windows: vec![window],
        active_window_index: Some(0),
        block_lists: Default::default(),
        running_mcp_servers: Default::default(),
    };
    let tempdir = tempfile::tempdir().unwrap();
    let mut conn = setup_database(&tempdir.path().join("warp.sqlite")).unwrap();
    save_app_state(&mut conn, &state).unwrap();
    let restored = read_sqlite_data(&mut conn, None).unwrap().app_state;

    assert_eq!(restored, state);
}
```

- [ ] **Step 2: Verify RED and extend snapshot types**

```rust
pub struct RepositoryWorkspaceWindowStateSnapshot {
    pub repository_workspace_id: RepositoryWorkspaceId,
    pub active_tab_index: usize,
}

pub struct WindowSnapshot {
    pub tabs: Vec<TabSnapshot>,
    pub active_repository_workspace_id: Option<RepositoryWorkspaceId>,
    pub repository_workspace_states: Vec<RepositoryWorkspaceWindowStateSnapshot>,
    // existing fields unchanged
}

pub struct TabSnapshot {
    pub repository_workspace_id: Option<RepositoryWorkspaceId>,
    // existing fields unchanged
}
```

在 `TabData` 和 `TransferredTab` 添加相同归属字段；LaunchConfig 转换显式把归属设为 `None`，因为配置模板不是 repository workspace 实例。

- [ ] **Step 3: Update SQLite save/restore in one transaction**

保存时扁平写入所有 TabSnapshot，并写 `tabs.repository_workspace_id`。获得新 `window_id` 后插入 `repository_workspace_window_states`。恢复时解析 UUID，非法值返回持久化错误而不是静默丢弃。

- [ ] **Step 4: Run snapshot and persistence tests**

Run:

```bash
cargo test -p warp --lib repository_workspace_state_round_trips
cargo test -p warp --lib launch_configs::launch_config_tests
cargo test -p warp --lib persistence::sqlite_tests
```

- [ ] **Step 5: Commit**

```bash
git add app/src/app_state.rs app/src/tab.rs app/src/workspace/view.rs app/src/persistence app/src/launch_configs
git commit -m "feat: persist workspace tab ownership"
```

### Task 7: Add active/inactive workspace Tab collections

**Files:**
- Create: `app/src/workspace/repository_workspace_tabs.rs`
- Create: `app/src/workspace/repository_workspace_tabs_tests.rs`
- Modify: `app/src/workspace/mod.rs`
- Modify: `app/src/workspace/view.rs`
- Modify: `app/src/workspace/cross_window_tab_drag.rs`
- Modify: `app/src/root_view.rs`

- [ ] **Step 1: Write failing collection swap tests**

```rust
#[test]
fn switching_workspaces_swaps_tabs_without_dropping_pane_groups() {
    let workspace_a = RepositoryWorkspaceId(uuid::Uuid::from_u128(1));
    let workspace_b = RepositoryWorkspaceId(uuid::Uuid::from_u128(2));
    let mut active_tabs = vec![10_u64];
    let mut active_tab_index = 0;
    let mut sets = RepositoryWorkspaceTabSets::new(Some(workspace_a));
    sets.insert_inactive(Some(workspace_b), RepositoryWorkspaceTabState::new(vec![20_u64], 0));
    sets.switch_to(Some(workspace_b), &mut active_tabs, &mut active_tab_index);
    assert_eq!(active_tabs, vec![20]);
    sets.switch_to(Some(workspace_a), &mut active_tabs, &mut active_tab_index);
    assert_eq!(active_tabs, vec![10]);
}

#[test]
fn each_window_remembers_an_active_tab_per_workspace() {
    let workspace_a = RepositoryWorkspaceId(uuid::Uuid::from_u128(1));
    let workspace_b = RepositoryWorkspaceId(uuid::Uuid::from_u128(2));
    let mut active_tabs = vec![10_u64, 11, 12];
    let mut active_tab_index = 0;
    let mut sets = RepositoryWorkspaceTabSets::new(Some(workspace_a));
    sets.insert_inactive(Some(workspace_b), RepositoryWorkspaceTabState::new(vec![20_u64], 0));
    active_tab_index = 2;
    sets.switch_to(Some(workspace_b), &mut active_tabs, &mut active_tab_index);
    sets.switch_to(Some(workspace_a), &mut active_tabs, &mut active_tab_index);
    assert_eq!(active_tab_index, 2);
}
```

- [ ] **Step 2: Verify RED and implement the focused helper**

```rust
pub struct RepositoryWorkspaceTabState<T> {
    pub tabs: Vec<T>,
    pub active_tab_index: usize,
}

pub struct RepositoryWorkspaceTabSets<T> {
    active_workspace_id: Option<RepositoryWorkspaceId>,
    inactive: HashMap<Option<RepositoryWorkspaceId>, RepositoryWorkspaceTabState<T>>,
}
```

该泛型 helper 只负责交换状态、索引 clamp、快照遍历和归属断言；生产代码使用 `RepositoryWorkspaceTabSets<TabData>`，PaneGroup 创建/关闭仍由 `Workspace` 负责。

- [ ] **Step 3: Integrate with `Workspace`**

新增 `switch_repository_workspace`、`all_repository_workspace_tabs` 和 `active_repository_workspace_id`。所有新建 TabData 使用当前 workspace id。活动集合为空时渲染项目空态，并让依赖 `active_tab_pane_group()` 的 actions 提前 no-op/disabled；不要添加虚假终端 fallback。

- [ ] **Step 4: Preserve ownership across cross-window drag**

`TransferredTab` 携带 workspace id。插入目标窗口前调用 `switch_repository_workspace`，然后按现有插入索引逻辑插入并激活。

- [ ] **Step 5: Run focused view tests**

Run:

```bash
cargo test -p warp --lib workspace::repository_workspace_tabs_tests
cargo test -p warp --lib workspace::view_test
cargo test -p warp --lib workspace::view::vertical_tabs_tests
```

- [ ] **Step 6: Commit**

```bash
git add app/src/workspace app/src/root_view.rs
git commit -m "feat: scope tabs to repository workspaces"
```

### Task 8: Implement startup migration and reconciliation

**Files:**
- Create: `app/src/project_organization/migration.rs`
- Create: `app/src/project_organization/migration_tests.rs`
- Modify: `app/src/project_organization/mod.rs`
- Modify: `app/src/lib.rs`

- [ ] **Step 1: Write failing migration tests**

```rust
#[test]
fn linked_worktree_tabs_migrate_together() {
    let identity = WorktreeIdentity {
        repository_path: PathBuf::from("/repo"),
        worktree_path: PathBuf::from("/repo-worktrees/feature-a"),
        branch: "feature/a".to_string(),
    };
    assert_eq!(classify_tab_worktree([Some(identity.clone()), Some(identity.clone())]), Some(identity));
}

#[test]
fn mixed_worktree_tab_remains_unclassified() {
    let first = WorktreeIdentity {
        repository_path: PathBuf::from("/repo"),
        worktree_path: PathBuf::from("/worktree-a"),
        branch: "feature/a".to_string(),
    };
    let second = WorktreeIdentity {
        repository_path: PathBuf::from("/repo"),
        worktree_path: PathBuf::from("/worktree-b"),
        branch: "feature/b".to_string(),
    };
    assert_eq!(classify_tab_worktree([Some(first), Some(second)]), None);
}
```

- [ ] **Step 2: Verify RED and implement deterministic classification**

对每个 TabSnapshot 收集所有 terminal cwd，解析 `--show-toplevel`、`--git-dir` 和 `--git-common-dir`。只有所有可识别 cwd 指向同一 linked worktree 时才分配；主 checkout、多 worktree、非 Git 或错误全部返回 `None`。

生产类型与纯分类函数固定为：

```rust
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorktreeIdentity {
    pub repository_path: PathBuf,
    pub worktree_path: PathBuf,
    pub branch: String,
}

pub fn classify_tab_worktree(
    identities: impl IntoIterator<Item = Option<WorktreeIdentity>>,
) -> Option<WorktreeIdentity> {
    let mut identities = identities.into_iter().flatten();
    let first = identities.next()?;
    identities.all(|identity| identity == first).then_some(first)
}
```

- [ ] **Step 3: Add idempotency and external-state reconciliation**

使用 repository path 和 `(repository_id, branch)` 唯一索引复用既有记录。启动时返回明确状态：

```rust
pub enum WorkspaceHealth {
    Ready,
    RepositoryMissing,
    WorktreeMissing,
    BranchMissing,
    WorktreeBranchMismatch { actual: String },
}
```

- [ ] **Step 4: Run tests and commit**

```bash
cargo test -p warp --lib project_organization::migration_tests
git add app/src/project_organization app/src/lib.rs
git commit -m "feat: migrate repository workspace state"
```

### Task 9: Build the repository/workspace tree

**Files:**
- Create: `app/src/project_organization/view/mod.rs`
- Create: `app/src/project_organization/view/project_tree.rs`
- Create: `app/src/project_organization/view/project_tree_tests.rs`
- Modify: `app/src/workspace/view.rs`
- Modify: `app/src/workspace/action.rs`
- Modify: `app/src/workspace/tab_settings.rs`
- Modify: `app/i18n/en/warp.ftl`
- Modify: `app/i18n/zh-CN/warp.ftl`
- Modify: `app/i18n/ja/warp.ftl`

- [ ] **Step 1: Write failing tree interaction tests**

```rust
#[test]
fn tree_renders_two_levels_and_selects_workspace() {
    let repository_id = RepositoryId(uuid::Uuid::from_u128(1));
    let workspace_id = RepositoryWorkspaceId(uuid::Uuid::from_u128(2));
    let mut state = ProjectTreeState::new(vec![RepositoryTreeNode {
        repository_id,
        display_name: "zap".to_string(),
        expanded: true,
        workspaces: vec![WorkspaceTreeNode {
            workspace_id,
            display_name: "Feature A".to_string(),
            branch: "feature/a".to_string(),
            tab_count: 2,
        }],
    }]);
    assert_eq!(state.visible_rows().len(), 2);
    state.select_workspace(workspace_id);
    assert_eq!(state.selected_workspace_id(), Some(workspace_id));
}

#[test]
fn flag_keeps_vertical_tabs_setting_but_uses_horizontal_tabbar() {
    assert_eq!(
        resolved_project_organization_tab_layout(true, true),
        TabLayout::Horizontal
    );
    assert_eq!(
        resolved_project_organization_tab_layout(false, true),
        TabLayout::Vertical
    );
}
```

- [ ] **Step 2: Verify RED and build the tree with existing components**

先实现纯状态类型 `ProjectTreeState`、`RepositoryTreeNode`、`WorkspaceTreeNode` 和 `TabLayout`，再让 View 渲染这些状态。Tree rows use existing theme accessors and button themes. Add icon-only plus/more buttons with Tooltip. Do not add a feature-specific `ActionButtonTheme`.

- [ ] **Step 3: Integrate the panel in the main window**

当 Flag 开启时，左侧可调整区域渲染 `ProjectTree`，`uses_vertical_tabs` 返回 false 但不写回 `TabSettings::use_vertical_tabs`。workspace 选择事件调用 `Workspace::switch_repository_workspace`。

- [ ] **Step 4: Add empty, loading, and health states**

实现无 repository、无 workspace、无 tabs、Git 操作中和 `WorkspaceHealth` 错误状态。所有文本走现有 i18n 机制；新增 key 同步写入 `app/i18n/en/warp.ftl`、`app/i18n/zh-CN/warp.ftl` 和 `app/i18n/ja/warp.ftl`，不得硬编码英文。

- [ ] **Step 5: Run UI tests and commit**

```bash
cargo test -p warp --lib project_organization::view::project_tree_tests
cargo test -p warp --lib workspace::view_test
git add app/src/project_organization app/src/workspace app/i18n
git commit -m "feat: add repository workspace tree"
```

### Task 10: Add repository and create workspace modals

**Files:**
- Create: `app/src/project_organization/view/add_repository_modal.rs`
- Create: `app/src/project_organization/view/add_repository_modal_tests.rs`
- Create: `app/src/project_organization/view/create_workspace_modal.rs`
- Create: `app/src/project_organization/view/create_workspace_modal_tests.rs`
- Modify: `app/src/project_organization/view/mod.rs`
- Modify: `app/src/workspace/view.rs`

- [ ] **Step 1: Write failing modal state tests**

```rust
#[test]
fn switching_creation_mode_clears_stale_branch_selection() {
    let mut state = CreateWorkspaceForm::remote("refs/remotes/origin/main", "feature/a");
    state.switch_mode(CreateWorkspaceMode::LocalBranch);
    assert_eq!(state.remote_ref, None);
    assert_eq!(state.new_branch_name, None);
}

#[test]
fn delete_branch_is_not_part_of_create_form() {
    let state = CreateWorkspaceForm::default();
    assert!(state.validate().is_err());
}
```

- [ ] **Step 2: Verify RED and implement form models first**

使用显式 enum：

```rust
pub enum AddRepositoryMode { LocalDirectory, GitUrl }
pub enum CreateWorkspaceMode { RemoteBase, LocalBranch }
```

表单校验返回结构化字段错误；视图只负责渲染和发事件。

- [ ] **Step 3: Implement async orchestration in the model**

Add Repository 调用 validate/clone 后写 model。Create Workspace 按 TECH 顺序执行 preflight → worktree → DB → 首个终端。失败时调用补偿函数，并把错误保留在 modal 中。

- [ ] **Step 4: Wire the first terminal to the worktree path**

复用 `NewTerminalOptions`/现有新 session API，显式传入 workspace path；禁止通过创建 TabConfig 再执行 `cd` 命令。

- [ ] **Step 5: Run modal and workspace tests**

```bash
cargo test -p warp --lib add_repository_modal_tests
cargo test -p warp --lib create_workspace_modal_tests
cargo test -p warp --lib workspace::view_test
```

- [ ] **Step 6: Commit**

```bash
git add app/src/project_organization app/src/workspace/view.rs
git commit -m "feat: create repositories and workspaces"
```

### Task 11: Add safe workspace and repository removal UI

**Files:**
- Create: `app/src/project_organization/view/delete_workspace_dialog.rs`
- Create: `app/src/project_organization/view/delete_workspace_dialog_tests.rs`
- Create: `app/src/project_organization/view/remove_repository_dialog.rs`
- Modify: `app/src/project_organization/view/mod.rs`
- Modify: `app/src/project_organization/model.rs`
- Modify: `app/src/workspace/view.rs`

- [ ] **Step 1: Write failing confirmation tests**

```rust
#[test]
fn delete_branch_is_checked_by_default() {
    let workspace_id = RepositoryWorkspaceId(uuid::Uuid::from_u128(1));
    assert!(DeleteWorkspaceDialogState::new(workspace_id).delete_branch);
}

#[test]
fn unmerged_branch_requires_second_confirmation_before_closing_tabs() {
    let workspace_id = RepositoryWorkspaceId(uuid::Uuid::from_u128(1));
    let mut flow = DeleteWorkspaceFlow::new(workspace_id);
    flow.apply_preflight(DeletionPreflight {
        branch: "feature/a".to_string(),
        is_merged: false,
        merge_target: "main".to_string(),
    });
    assert_eq!(flow.next_action(), DeleteWorkspaceAction::ConfirmForceBranchDelete);
    assert!(!flow.may_close_tabs());
}
```

- [ ] **Step 2: Verify RED and implement the state machine**

```rust
pub enum DeleteWorkspaceAction {
    RunPreflight,
    ConfirmForceBranchDelete,
    CloseTabsAndDelete,
    Finished,
}
```

只有进入 `CloseTabsAndDelete` 后才关闭 PaneGroup/terminal。dirty、missing 或 mismatch 错误停留在 dialog，并保持磁盘和运行时状态。

- [ ] **Step 3: Implement repository removal rules**

有 workspace 时禁用移除。Cloned repository 显示“同时删除本地目录”且默认 false；Local repository 永不显示目录删除选项。

- [ ] **Step 4: Run tests and commit**

```bash
cargo test -p warp --lib delete_workspace_dialog_tests
cargo test -p warp --lib project_organization::model_tests
git add app/src/project_organization app/src/workspace/view.rs
git commit -m "feat: remove repository workspaces safely"
```

### Task 12: Add end-to-end integration coverage

**Files:**
- Create: `app/src/integration_testing/repository_workspaces/mod.rs`
- Create: `app/src/integration_testing/repository_workspaces/step.rs`
- Create: `app/src/integration_testing/repository_workspaces/assertion.rs`
- Modify: `app/src/integration_testing/mod.rs`
- Create: `crates/integration/src/test/repository_workspaces.rs`
- Modify: `crates/integration/src/test.rs`
- Modify: `crates/integration/src/bin/integration.rs`

- [ ] **Step 1: Add a failing local repository flow**

Build steps that create a temporary bare remote and local clone, enable the Feature Flag, add repository, create remote-base workspace, open three terminal tabs, switch away/back, and assert all session IDs still exist。

```rust
pub fn test_repository_workspace_remote_flow() -> Builder {
    let mut builder = new_builder();
    builder.add_steps(vec![
        setup_repository_fixture("repository fixture"),
        enable_repository_workspaces_flag(),
        add_local_repository("repository fixture"),
        create_remote_base_workspace("origin/main", "feature/a"),
        add_workspace_terminal_tab(),
        add_workspace_terminal_tab(),
        assert_active_workspace("feature/a"),
        assert_workspace_tab_count("feature/a", 3),
        switch_to_unclassified_tabs(),
        switch_to_workspace("feature/a"),
        assert_workspace_sessions_alive("feature/a", 3),
    ]);
    builder
}
```

在 `step.rs` 定义动作并通过 Builder data 保存 fixture 路径/ID。核心 fixture step 使用真实 Git 命令和临时 HOME：

```rust
pub struct RepositoryWorkspaceFixture {
    pub tempdir: tempfile::TempDir,
    pub repository_path: PathBuf,
    pub remote_path: PathBuf,
}

impl RepositoryWorkspaceFixture {
    pub fn new() -> Self {
        let tempdir = tempfile::tempdir().unwrap();
        let repository_path = tempdir.path().join("repository");
        let remote_path = tempdir.path().join("remote.git");
        run_fixture_git(tempdir.path(), &["init", "--bare", remote_path.to_str().unwrap()]);
        run_fixture_git(tempdir.path(), &["clone", remote_path.to_str().unwrap(), repository_path.to_str().unwrap()]);
        run_fixture_git(&repository_path, &["switch", "-c", "main"]);
        std::fs::write(repository_path.join("README.md"), "fixture").unwrap();
        run_fixture_git(&repository_path, &["add", "README.md"]);
        run_fixture_git(&repository_path, &["-c", "user.name=Zap Tests", "-c", "user.email=zap@example.com", "commit", "-m", "init"]);
        run_fixture_git(&repository_path, &["push", "-u", "origin", "main"]);
        Self { tempdir, repository_path, remote_path }
    }
}

fn run_fixture_git(cwd: &Path, args: &[&str]) {
    let status = command::blocking::Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args(args)
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?} failed");
}

pub fn setup_repository_fixture(key: &'static str) -> TestStep {
    TestStep::new("Set up repository workspace fixture").with_action(move |_, _, data| {
        let fixture = RepositoryWorkspaceFixture::new();
        data.insert(key, fixture);
    })
}

pub fn enable_repository_workspaces_flag() -> TestStep {
    TestStep::new("Enable repository workspaces").with_action(|app, _, _| {
        app.update(|_| FeatureFlag::RepositoryWorkspaces.set_enabled(true));
    })
}
```

在 `assertion.rs` 实现 `assert_active_workspace`、`assert_workspace_tab_count` 和 `assert_workspace_sessions_alive`；`crates/integration/src/test.rs` 添加 `mod repository_workspaces;` 与 `pub use repository_workspaces::*;`，runner 为每个测试函数添加 `register_test!`。

- [ ] **Step 2: Run the single integration test and verify RED**

Run:

```bash
WARPUI_USE_REAL_DISPLAY_IN_INTEGRATION_TESTS=1 \
RUST_BACKTRACE=full \
WARP_SHELL_PATH=/bin/bash \
cargo run -p integration --bin integration -- test_repository_workspace_remote_flow
```

Expected: FAIL，且失败必须指向缺失的 step/action wiring，而不是 fixture 初始化。

- [ ] **Step 3: Implement test helpers and all acceptance flows**

添加以下具名 Builder，复用相同 fixture/step API：

```rust
pub fn test_repository_workspace_clone_destination() -> Builder {
    builder_with_steps(vec![setup_remote_fixture(), clone_to_edited_destination(), assert_clone_destination()])
}

pub fn test_repository_workspace_local_branch_conflict() -> Builder {
    builder_with_steps(vec![setup_occupied_branch_fixture(), attempt_local_branch_workspace(), assert_branch_occupied_error()])
}

pub fn test_repository_workspace_migration() -> Builder {
    builder_with_steps(vec![setup_linked_worktree_snapshot(), restart_app(), assert_migrated_and_unclassified_tabs()])
}

pub fn test_repository_workspace_restoration() -> Builder {
    builder_with_steps(vec![create_two_workspaces(), select_second_workspace_tab(), restart_app(), assert_workspace_selection_restored()])
}

pub fn test_repository_workspace_deletion() -> Builder {
    builder_with_steps(vec![create_dirty_and_clean_workspaces(), exercise_delete_confirmations(), assert_expected_branches_and_worktrees()])
}

pub fn test_repository_workspace_external_removal() -> Builder {
    builder_with_steps(vec![create_workspace(), remove_worktree_outside_app(), restart_app(), assert_missing_worktree_health()])
}

fn builder_with_steps(steps: Vec<TestStep>) -> Builder {
    let mut builder = new_builder();
    builder.add_steps(steps);
    builder
}
```

`builder_with_steps` 是本测试文件中的小 helper：创建 `new_builder()`、调用 `add_steps` 并返回 Builder。所有引用的 step/assertion 函数分别在 `step.rs`/`assertion.rs` 中实现，名称与上面保持一致。

- [ ] **Step 4: Run integration tests**

Run the new test module with `--no-fail-fast`, then run the nearest existing tab/workspace suites to detect regressions.

- [ ] **Step 5: Commit**

```bash
git add app/src/integration_testing crates/integration
git commit -m "test: cover repository workspace flows"
```

### Task 13: Final cleanup, verification, and review

**Files:**
- Modify only files required by compiler/test feedback
- Update: `specs/repository-workspaces/PRODUCT.md`
- Update: `specs/repository-workspaces/TECH.md`

- [ ] **Step 1: Keep specs synchronized**

Compare shipped behavior and module boundaries with both specs. Update them only for actual implementation decisions; do not leave stale paths, commands, or behavior.

- [ ] **Step 2: Confirm old and new entrypoints are correctly gated**

With Flag off, old worktree/TabConfig behavior remains available. With Flag on, tree-driven repository/workspace entrypoints are visible and Vertical Tabs is render-disabled without changing the stored setting.

- [ ] **Step 3: Run formatting and focused tests**

```bash
cargo fmt --all -- --check
cargo test -p warp_features repository_workspaces_is_dogfood_only
cargo test -p warp --lib project_organization
cargo test -p warp --lib workspace::repository_workspace_tabs_tests
cargo test -p warp --lib persistence::sqlite_tests
```

- [ ] **Step 4: Run full verification**

```bash
cargo nextest run --no-fail-fast --workspace --exclude command-signatures-v2
cargo check
```

Expected: exit 0。若存在已知无关失败，保存完整输出并运行所有受影响的针对性套件；不得把部分验证描述为全量通过。

- [ ] **Step 5: Review the diff**

使用 `superpowers:requesting-code-review` 执行规格一致性和代码质量审查。若收到 review 反馈，先完整呈现给用户并等待确认要修复的项目，再按 `superpowers:receiving-code-review` 处理。

- [ ] **Step 6: Commit final fixes**

```bash
git add specs/repository-workspaces app crates
git commit -m "feat: organize terminals by repository workspace"
```

- [ ] **Step 7: Complete branch handoff**

使用 `superpowers:finishing-a-development-branch` 汇总验证证据，并提供合并、PR 或保留分支选项。

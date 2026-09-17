# Create Workspace Branch Picker Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 让创建 workspace 弹窗安全地选择 remote 分支，并由选择结果生成受约束显示的默认字段。

**Architecture:** `CreateWorkspaceModal` 把 `BranchRef::Remote` 映射为保留完整 refname 的结构化选项，并用已有 `FilterableDropdown` 承载 remote/local 选择。modal 从当前选择的 branch name 同步派生各默认字段；`Workspace` 负责 fetch、失败后的本地 refs 回退和 Retry 调度。编辑器及错误文本被限制在 modal 内容宽度内，选择器只限制顶栏及菜单宽度以避免裁剪其弹出菜单。

**Tech Stack:** Rust 2021、WarpUI `FilterableDropdown`、`EditorView`、`ActionButton`、`BranchRef`、`cargo test`、Cargo workspace。

---

## 文件结构

- 修改：`app/src/project_organization/view/create_workspace_modal.rs`
  - 定义结构化 remote 选项、默认值派生、远端加载状态和 picker action。
  - 用不可编辑的 `FilterableDropdown` 替换 branch text editor。
  - 约束长编辑器内容与错误文本，保留 picker 下拉菜单的可见性。
- 修改：`app/src/project_organization/view/create_workspace_modal_tests.rs`
  - 覆盖标签消歧、完整 ref 保留、默认值、覆盖语义和 Retry event 映射。
- 修改：`app/src/workspace/view.rs`
  - 统一首次打开和 Retry 的 remote ref 获取；fetch 失败时读取本地 refs，保持 local 模式可用。

### Task 1: 建立结构化 remote 分支与默认路径

**Files:**
- Modify: `app/src/project_organization/view/create_workspace_modal_tests.rs:1-110`
- Modify: `app/src/project_organization/view/create_workspace_modal.rs:1-150`

- [ ] **Step 1: 编写 remote 标签与默认路径的失败测试**

替换现有 `branch_ref_options_preserve_remote_refnames_and_local_branch_names`，并增加下列测试。测试 imports 加入 `PathBuf`、`RemoteBranchOption` 与 `default_worktree_path`。

```rust
#[test]
fn remote_branch_options_hide_ref_prefix_and_disambiguate_duplicate_names() {
    let (remote_options, local_branches) = branch_ref_options([
        BranchRef::Remote {
            remote: "origin".to_string(),
            name: "main".to_string(),
            full_ref: "refs/remotes/origin/main".to_string(),
        },
        BranchRef::Remote {
            remote: "upstream".to_string(),
            name: "main".to_string(),
            full_ref: "refs/remotes/upstream/main".to_string(),
        },
        BranchRef::Remote {
            remote: "origin".to_string(),
            name: "feature/tree".to_string(),
            full_ref: "refs/remotes/origin/feature/tree".to_string(),
        },
        BranchRef::Local {
            name: "feature/local".to_string(),
            full_ref: "refs/heads/feature/local".to_string(),
        },
    ]);

    assert_eq!(
        remote_options,
        vec![
            RemoteBranchOption::new(
                "refs/remotes/origin/feature/tree",
                "origin",
                "feature/tree",
                "feature/tree",
            ),
            RemoteBranchOption::new(
                "refs/remotes/origin/main",
                "origin",
                "main",
                "origin/main",
            ),
            RemoteBranchOption::new(
                "refs/remotes/upstream/main",
                "upstream",
                "main",
                "upstream/main",
            ),
        ]
    );
    assert_eq!(local_branches, vec!["feature/local"]);
}

#[test]
fn default_worktree_path_uses_repository_and_branch_names() {
    assert_eq!(
        default_worktree_path(
            PathBuf::from("/Users/example"),
            "dip-agent",
            "feature/project-tree",
        ),
        PathBuf::from("/Users/example/.warp/worktrees/dip-agent/feature-project-tree"),
    );
}
```

- [ ] **Step 2: 运行测试并验证 RED**

Run:

```bash
cargo test -p warp remote_branch_options_hide_ref_prefix_and_disambiguate_duplicate_names --lib -- --nocapture
cargo test -p warp default_worktree_path_uses_repository_and_branch_names --lib -- --nocapture
```

Expected: 两项均因 `RemoteBranchOption` 或 `default_worktree_path` 尚不存在而失败。

- [ ] **Step 3: 实现 remote 选项与路径函数**

在 `create_workspace_modal.rs` 增加：

```rust
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RemoteBranchOption {
    full_ref: String,
    remote: String,
    branch_name: String,
    display_label: String,
}

impl RemoteBranchOption {
    fn new(
        full_ref: impl Into<String>,
        remote: impl Into<String>,
        branch_name: impl Into<String>,
        display_label: impl Into<String>,
    ) -> Self {
        Self {
            full_ref: full_ref.into(),
            remote: remote.into(),
            branch_name: branch_name.into(),
            display_label: display_label.into(),
        }
    }
}

pub fn default_worktree_path(home: PathBuf, repository_name: &str, branch_name: &str) -> PathBuf {
    home.join(".warp")
        .join("worktrees")
        .join(workspace_dir_name(repository_name, ""))
        .join(workspace_dir_name(branch_name, ""))
}
```

把 `branch_ref_options` 的返回类型改为 `(Vec<RemoteBranchOption>, Vec<String>)`。先收集 remote 的 `full_ref`、`remote` 与 `name`，以 `branch_name` 计数；计数为 1 时 `display_label` 是裸分支名，否则是 `format!("{remote}/{branch_name}")`。按 `(display_label, full_ref)` 排序，且绝不改写 `full_ref`。导入 `workspace_dir_name`：

```rust
use crate::project_organization::git::{workspace_dir_name, BranchRef};
```

- [ ] **Step 4: 运行测试并验证 GREEN**

Run:

```bash
cargo test -p warp remote_branch_options_hide_ref_prefix_and_disambiguate_duplicate_names --lib -- --nocapture
cargo test -p warp default_worktree_path_uses_repository_and_branch_names --lib -- --nocapture
```

Expected: 两项通过，普通 remote ref 显示裸 branch，同名项显示 remote 前缀，完整 ref 和路径派生确定。

- [ ] **Step 5: 检查任务范围的 diff**

```bash
git diff --check -- app/src/project_organization/view/create_workspace_modal.rs app/src/project_organization/view/create_workspace_modal_tests.rs
git diff -- app/src/project_organization/view/create_workspace_modal.rs app/src/project_organization/view/create_workspace_modal_tests.rs
```

Expected: 新增内容仅是结构化 remote 选项与确定性路径派生。由于这些文件是工作区中既有的未跟踪功能实现，不在本任务中提交整份文件。

### Task 2: 用 picker 选择分支并同步派生字段

**Files:**
- Modify: `app/src/project_organization/view/create_workspace_modal_tests.rs:1-180`
- Modify: `app/src/project_organization/view/create_workspace_modal.rs:145-510`

- [ ] **Step 1: 编写选择覆盖语义的失败测试**

在测试文件加入：

```rust
#[test]
fn selecting_remote_branch_overwrites_all_derived_workspace_fields() {
    let mut defaults = CreateWorkspaceDefaults::new(
        PathBuf::from("/Users/example"),
        "dip-agent".to_string(),
    );
    defaults.apply_branch("feature/one");
    defaults.new_branch = "custom".to_string();
    defaults.workspace_name = "custom workspace".to_string();
    defaults.worktree_path = PathBuf::from("/tmp/custom");

    defaults.apply_branch("feature/two");

    assert_eq!(defaults.new_branch, "feature/two");
    assert_eq!(defaults.workspace_name, "feature/two");
    assert_eq!(
        defaults.worktree_path,
        PathBuf::from("/Users/example/.warp/worktrees/dip-agent/feature-two"),
    );
}
```

测试 imports 加入 `CreateWorkspaceDefaults`。

- [ ] **Step 2: 运行测试并验证 RED**

Run:

```bash
cargo test -p warp selecting_remote_branch_overwrites_all_derived_workspace_fields --lib -- --nocapture
```

Expected: 因 `CreateWorkspaceDefaults` 不存在而失败。

- [ ] **Step 3: 加入纯默认值状态与 picker action**

在 modal 文件增加下列值对象，并使用它作为唯一的默认值派生位置：

```rust
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreateWorkspaceDefaults {
    home: PathBuf,
    repository_name: String,
    new_branch: String,
    workspace_name: String,
    worktree_path: PathBuf,
}

impl CreateWorkspaceDefaults {
    pub fn new(home: PathBuf, repository_name: String) -> Self {
        Self {
            home,
            repository_name,
            new_branch: String::new(),
            workspace_name: String::new(),
            worktree_path: PathBuf::new(),
        }
    }

    pub fn apply_branch(&mut self, branch_name: &str) {
        self.new_branch = branch_name.to_string();
        self.workspace_name = branch_name.to_string();
        self.worktree_path = default_worktree_path(
            self.home.clone(),
            &self.repository_name,
            branch_name,
        );
    }
}
```

把 `branch_editor`、`remote_refs` 替换为：

```rust
remote_branch_picker: ViewHandle<FilterableDropdown<CreateWorkspaceModalAction>>,
local_branch_picker: ViewHandle<FilterableDropdown<CreateWorkspaceModalAction>>,
remote_branch_options: Vec<RemoteBranchOption>,
local_branches: Vec<String>,
selected_remote_branch: Option<RemoteBranchOption>,
selected_local_branch: Option<String>,
remote_fetch_error: Option<String>,
defaults: Option<CreateWorkspaceDefaults>,
```

给 action 加 `NoOp`、`SelectRemoteBranch(RemoteBranchOption)`、`SelectLocalBranch(String)` 和 `RetryBranchRefs`，并为 action 派生 `Clone, Debug, Eq, PartialEq`。在 `new` 中创建两个 `FilterableDropdown`，均调用：

```rust
dropdown.set_top_bar_max_width(480.);
dropdown.set_menu_width(480., ctx);
dropdown.set_disabled(ctx);
```

`configure` 接收 `home: PathBuf`、`repository_name: String`，创建 `CreateWorkspaceDefaults` 并清空两个选项/选择。remote picker 设置一个禁用的 `Fetching remote branches...` 占位项（以 `NoOp` action 承载）；local picker 清空并禁用。不要再预生成带 workspace UUID 的 worktree path。

`set_branch_refs` 构造两组 `DropdownItem`，成功时启用两个 picker。保存 remote/local 的首项为已选值，把首个 remote 的 `full_ref` 写入 form，并且仅在当前是 remote mode 时调用 `apply_remote_branch`。`apply_remote_branch` 与 `apply_local_branch` 必须：

1. 设置 form 的实际 remote ref 或 local branch。
2. 调用 `defaults.apply_branch`。
3. 把 `workspace_name` 和 `worktree_path` 写入 editor；remote 分支还把 `new_branch` 写入 editor。

所以任意新选择都会覆盖用户此前修改。`set_mode` 改为应用存储的当前 remote/local 选择，而不从显示文字解析 Git ref。`try_submit` 直接使用 `form` 中的结构化选择，不再从可编辑 branch editor 读取 ref。

- [ ] **Step 4: 受约束地渲染 picker、编辑器和错误文本**

导入：

```rust
use crate::view_components::{DropdownItem, FilterableDropdown};
use warpui::elements::{Clipped, ConstrainedBox, Shrinkable};
```

remote/local 分支 section 使用对应的 `ChildView<FilterableDropdown<...>>`，不放进 `Clipped`，以免裁剪其定位的下拉菜单；两个 picker 已由 `set_top_bar_max_width(480.)` 和 `set_menu_width(480., ctx)` 约束。

对 `new_branch_editor`、`display_name_editor`、`worktree_path_editor` 和错误 `Text` 使用：

```rust
Shrinkable::new(
    1.0,
    ConstrainedBox::new(Clipped::new(child).finish())
        .with_max_width(480.)
        .finish(),
)
.finish()
```

remote 分支错误显示在分支 section 下方。远端模式在 picker 尚未成功选择分支时，`try_submit` 显示 `"Select a remote branch before creating a workspace."` 并不提交。

- [ ] **Step 5: 运行测试并验证 GREEN**

Run:

```bash
cargo test -p warp selecting_remote_branch_overwrites_all_derived_workspace_fields --lib -- --nocapture
cargo test -p warp create_workspace_modal --lib -- --nocapture
```

Expected: 选择 remote 分支覆盖三个派生字段，完整 remote ref 仍传入创建请求，local 分支不能接受 remote ref。

- [ ] **Step 6: 检查 picker 和布局 diff**

```bash
git diff --check -- app/src/project_organization/view/create_workspace_modal.rs app/src/project_organization/view/create_workspace_modal_tests.rs
```

Expected: 新增内容只替换 branch text editor、同步默认值并约束长内容，不提交整份既有未跟踪文件。

### Task 3: 协调 fetch、失败回退与 Retry

**Files:**
- Modify: `app/src/project_organization/view/create_workspace_modal_tests.rs:1-230`
- Modify: `app/src/project_organization/view/create_workspace_modal.rs:145-520`
- Modify: `app/src/workspace/view.rs:75-90,5630-5710`

- [ ] **Step 1: 为 Retry repository 映射编写失败测试**

为 production helper 而非 test-only constructor 编写测试：

```rust
#[test]
fn retry_event_targets_the_configured_repository() {
    let target = CreateWorkspaceTarget {
        repository_id: RepositoryId(uuid::Uuid::from_u128(1)),
        workspace_id: RepositoryWorkspaceId(uuid::Uuid::from_u128(2)),
    };

    assert_eq!(
        target.retry_branch_refs_event(),
        CreateWorkspaceModalEvent::RetryBranchRefs {
            repository_id: RepositoryId(uuid::Uuid::from_u128(1)),
            workspace_id: RepositoryWorkspaceId(uuid::Uuid::from_u128(2)),
        },
    );
}
```

测试 imports 加入 `CreateWorkspaceModalEvent` 与 `CreateWorkspaceTarget`；event 需派生 `Eq, PartialEq`。

- [ ] **Step 2: 运行测试并验证 RED**

Run:

```bash
cargo test -p warp retry_event_targets_the_configured_repository --lib -- --nocapture
```

Expected: 因 `retry_branch_refs_event` 与 event variant 尚不存在而失败。

- [ ] **Step 3: 实现 modal 的加载状态与 Retry event**

在 `CreateWorkspaceModalEvent` 加入：

```rust
RetryBranchRefs {
    repository_id: RepositoryId,
    workspace_id: RepositoryWorkspaceId,
},
```

为 `CreateWorkspaceTarget` 添加生产 helper：

```rust
fn retry_branch_refs_event(self) -> CreateWorkspaceModalEvent {
    CreateWorkspaceModalEvent::RetryBranchRefs {
        repository_id: self.repository_id,
        workspace_id: self.workspace_id,
    }
}
```

`handle_action` 在 `RetryBranchRefs` 分支中仅当 `target` 存在时发出该 helper 的 event。`begin_branch_fetch` 清除 remote error、禁用 remote picker 并显示 loading 占位项；若 local picker 已有 fallback 结果则保持它可用。`set_branch_fetch_error` 保存错误并禁用 remote picker；`set_local_branch_refs` 只提取/填充 local 分支并启用 local picker。增加采用现有 `SecondaryTheme` 的 `retry_remote_button`，其 click 派发 `RetryBranchRefs` action。

`try_submit` 在 remote mode 且 `remote_fetch_error.is_some()` 时拒绝提交并显示该错误，local mode 不受该状态影响。

- [ ] **Step 4: 提取 Workspace fetch 协调逻辑并添加 local 回退**

在 `workspace/view.rs` imports 中加入 `list_branch_refs_async`。新增：

```rust
fn fetch_create_workspace_branch_refs(
    &mut self,
    repository_id: RepositoryId,
    workspace_id: RepositoryWorkspaceId,
    repository_path: PathBuf,
    ctx: &mut ViewContext<Self>,
) {
    self.create_workspace_modal.view.update(ctx, |modal, ctx| {
        modal.body().update(ctx, |body, ctx| body.begin_branch_fetch(ctx));
    });

    ctx.spawn(fetch_and_list_refs_async(repository_path.clone()), move |workspace, result, ctx| {
        match result {
            Ok(refs) => workspace.create_workspace_modal.view.update(ctx, |modal, ctx| {
                modal.body().update(ctx, |body, ctx| {
                    if body.matches_target(repository_id, workspace_id) {
                        body.set_branch_refs(refs, ctx);
                    }
                });
            }),
            Err(fetch_error) => {
                let error_message = format!("Failed to fetch repository refs: {fetch_error}");
                ctx.spawn(list_branch_refs_async(repository_path), move |workspace, local_result, ctx| {
                    workspace.create_workspace_modal.view.update(ctx, |modal, ctx| {
                        modal.body().update(ctx, |body, ctx| {
                            if !body.matches_target(repository_id, workspace_id) {
                                return;
                            }
                            if let Ok(refs) = local_result {
                                body.set_local_branch_refs(refs, ctx);
                            }
                            body.set_branch_fetch_error(error_message, ctx);
                        });
                    });
                });
            }
        }
    });
}
```

`open_create_workspace_modal` 使用 `dirs::home_dir().expect("home directory should be available")`、`repository.display_name.clone()` 调用新的 `configure`，打开 modal 后调用上述 helper。不要再生成随机 UUID path。

在 `handle_create_workspace_modal_body_event` 处理：

```rust
CreateWorkspaceModalEvent::RetryBranchRefs {
    repository_id,
    workspace_id,
} => {
    let Some(repository) = ProjectOrganizationModel::handle(ctx)
        .as_ref(ctx)
        .repository(*repository_id)
        .cloned()
    else {
        return;
    };
    self.fetch_create_workspace_branch_refs(*repository_id, *workspace_id, repository.path, ctx);
}
```

`matches_target(repository_id, workspace_id)` 同时核对当前 target 的两项 ID；这个 guard 必须覆盖首次 fetch、local fallback 和 Retry，避免关闭、重新配置或同一 repository 的新 workspace 打开后被慢结果覆盖。

- [ ] **Step 5: 运行测试并验证 GREEN**

Run:

```bash
cargo test -p warp retry_event_targets_the_configured_repository --lib -- --nocapture
cargo test -p warp create_workspace_modal --lib -- --nocapture
cargo check -p warp
```

Expected: Retry event 仅对应当前 repository，fetch 失败时 remote 禁用且 local 可从已有 refs 选择，crate 编译成功。

- [ ] **Step 6: 检查 fetch 协调 diff**

```bash
git diff --check -- app/src/project_organization/view/create_workspace_modal.rs app/src/project_organization/view/create_workspace_modal_tests.rs app/src/workspace/view.rs
```

Expected: 新增内容只包含 fetch 协调、local refs 回退和 Retry；不提交含用户既有改动的文件。

### Task 4: 复核与人工验证构建

**Files:**
- Modify: `app/src/project_organization/view/create_workspace_modal.rs`
- Modify: `app/src/project_organization/view/create_workspace_modal_tests.rs`
- Modify: `app/src/workspace/view.rs`

- [ ] **Step 1: 运行所有相关测试和格式检查**

Run:

```bash
cargo test -p warp create_workspace_modal --lib -- --nocapture
cargo test -p warp remote_branch_options --lib -- --nocapture
cargo fmt --check
cargo check -p warp
```

Expected: 全部 exit code 为 0。当前环境没有 `cargo-nextest` 时，这些 `cargo test` 是针对性回归验证。

- [ ] **Step 2: 复查精确 diff**

Run:

```bash
git diff --check -- app/src/project_organization/view/create_workspace_modal.rs app/src/project_organization/view/create_workspace_modal_tests.rs app/src/workspace/view.rs
git diff -- app/src/project_organization/view/create_workspace_modal.rs app/src/project_organization/view/create_workspace_modal_tests.rs app/src/workspace/view.rs
```

确认改动仅包括 picker、默认字段同步、fetch 回退/重试和长内容约束，未混入用户已有的 repository workspace 改动。

- [ ] **Step 3: 生成 macOS 人工验证 bundle**

Run:

```bash
./script/run --dont-open
codesign --verify --deep --strict --verbose=2 target/debug/bundle/osx/Zap.app
```

Expected: `target/debug/bundle/osx/Zap.app` 存在且签名校验通过。手工验证：打开包含长 remote 分支和长路径的 repository；确认普通 picker 项显示裸 branch，同名项显示 remote 前缀；选择项会同步三个默认字段；fetch 失败后 Retry 可用且 local mode 可继续；所有长内容均保持在 modal 宽度内。

- [ ] **Step 4: 给出建议 commit message**

在交付时提供：

```text
feat: improve workspace branch selection
```

不在本任务中创建 commit，以避免把用户已有的未跟踪 repository workspace 实现混入提交。

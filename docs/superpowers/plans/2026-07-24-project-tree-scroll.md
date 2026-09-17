# Project Tree Scrolling Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 让 `ProjectTreePanel` 的 repository/workspace 树在内容超出可视高度时支持垂直滚轮滚动，同时固定标题和底部区域。

**Architecture:** 在现有 `ProjectTreePanel` 内持有 `ClippedScrollStateHandle`，使用 WarpUI 的 `ClippedScrollable::vertical` 只包裹中间树内容。顶部标题和底部未分类 workspace 继续作为外层 `Flex::column` 的独立子元素，不改变项目组织模型和树状态逻辑。

**Tech Stack:** Rust, WarpUI `ClippedScrollable`, `ProjectTreePanel` view tests, Cargo.

---

### Task 1: 添加可复现滚动失败的视图测试

**Files:**
- Modify: `app/src/project_organization/view/project_tree_tests.rs`

- [ ] **Step 1: Write the failing test**

在现有测试文件中添加以下测试。它创建 12 个 repository，每个 repository 包含一个 workspace，使树内容明确超过 `320x240` 的窗口高度；测试记录最后一个 repository 的“创建 workspace”按钮位置，派发垂直滚轮事件并重建场景，要求该按钮向上移动。

```rust
#[test]
fn project_tree_scrolls_when_workspace_list_overflows() {
    App::test((), |mut app| async move {
        app.add_singleton_model(|_| Appearance::mock());
        let tempdir = tempfile::tempdir().expect("temporary directory should be created");
        let timestamp = chrono::DateTime::from_timestamp(0, 0)
            .expect("timestamp should be valid")
            .naive_utc();
        let mut repositories = Vec::new();
        let mut workspaces = Vec::new();
        let mut last_repository_id = None;

        for index in 0..12 {
            let repository_path = tempdir.path().join(format!("repository-{index}"));
            let worktree_path = tempdir.path().join(format!("worktree-{index}"));
            std::fs::create_dir(&repository_path)
                .expect("repository directory should be created");
            std::fs::create_dir(&worktree_path).expect("worktree directory should be created");

            let repository_id = RepositoryId(uuid::Uuid::from_u128(index + 1));
            let workspace_id = RepositoryWorkspaceId(uuid::Uuid::from_u128(index + 100));
            last_repository_id = Some(repository_id);
            repositories.push(PersistedRepository {
                id: repository_id.to_string(),
                display_name: format!("repository-{index:02}"),
                path: repository_path.to_string_lossy().to_string(),
                remote_url: None,
                source: "local".to_string(),
                created_at: timestamp,
                last_opened_at: timestamp,
            });
            workspaces.push(PersistedRepositoryWorkspace {
                id: workspace_id.to_string(),
                repository_id: repository_id.to_string(),
                display_name: format!("workspace-{index:02}"),
                branch: format!("feature/workspace-{index:02}"),
                worktree_path: worktree_path.to_string_lossy().to_string(),
                created_at: timestamp,
                last_opened_at: timestamp,
            });
        }

        app.add_singleton_model(|ctx| {
            ProjectOrganizationModel::try_new(
                repositories,
                workspaces,
                RepositoryPersistence::new(None),
                ctx,
            )
            .expect("project organization model should initialize")
        });

        let (window_id, host) =
            app.add_window(WindowStyle::NotStealFocus, ProjectTreeTestHost::new);
        let project_tree = host.read(&app, |host, _| host.project_tree.clone());
        let root_view_id = app
            .root_view_id(window_id)
            .expect("window should have a root view");
        let last_repository_id = last_repository_id.expect("at least one repository should exist");
        let last_button_position_id = repository_add_workspace_position_id(last_repository_id);
        let presenter = Rc::new(RefCell::new(Presenter::new(window_id)));

        let initial_y = app.update({
            let presenter = presenter.clone();
            move |ctx| {
                presenter.borrow_mut().invalidate(
                    WindowInvalidation {
                        updated: [root_view_id, project_tree.id()].into_iter().collect(),
                        ..Default::default()
                    },
                    ctx,
                );
                presenter
                    .borrow_mut()
                    .build_scene(vec2f(320., 240.), 1., None, ctx);
                presenter
                    .borrow()
                    .position_cache()
                    .get_position(&last_button_position_id)
                    .expect("last repository button should have a saved position")
                    .origin()
                    .y()
            }
        });

        app.update({
            let presenter = presenter.clone();
            move |ctx| {
                ctx.simulate_window_event(
                    Event::ScrollWheel {
                        position: vec2f(160., 120.),
                        delta: vec2f(0., -50.),
                        precise: true,
                        modifiers: Default::default(),
                    },
                    window_id,
                    presenter.clone(),
                );
                presenter.borrow_mut().invalidate(
                    WindowInvalidation {
                        updated: [root_view_id, project_tree.id()].into_iter().collect(),
                        ..Default::default()
                    },
                    ctx,
                );
                presenter
                    .borrow_mut()
                    .build_scene(vec2f(320., 240.), 1., None, ctx);
                let scrolled_y = presenter
                    .borrow()
                    .position_cache()
                    .get_position(&last_button_position_id)
                    .expect("last repository button should have a saved position")
                    .origin()
                    .y();
                assert!(
                    scrolled_y < initial_y,
                    "scrolling the workspace list should move its rows upward"
                );
            }
        });
    });
}
```

- [ ] **Step 2: Run the test to verify it fails for the missing behavior**

Run:

```bash
cargo test -p warp project_tree_scrolls_when_workspace_list_overflows --lib
```

Expected: the test compiles and fails at the `scrolled_y < initial_y` assertion, because `ProjectTreePanel` currently has no scrollable element to consume the wheel event.

- [ ] **Step 3: Commit the failing test**

```bash
git add app/src/project_organization/view/project_tree_tests.rs
git commit -m "test: reproduce project tree scroll overflow"
```

### Task 2: 添加垂直滚动容器

**Files:**
- Modify: `app/src/project_organization/view/project_tree.rs:7-20` (WarpUI imports)
- Modify: `app/src/project_organization/view/project_tree.rs:344-357` (`ProjectTreePanel` state)
- Modify: `app/src/project_organization/view/project_tree.rs:358-390` (`ProjectTreePanel::new`)
- Modify: `app/src/project_organization/view/project_tree.rs:995-1070` (`ProjectTreePanel::render`)

- [ ] **Step 1: Add the scroll state and existing WarpUI imports**

Extend the `warpui::elements` import with `ClippedScrollStateHandle`, `ClippedScrollable`, `Fill`, and `ScrollbarWidth`, then add this field to `ProjectTreePanel`:

```rust
clipped_scroll_state: ClippedScrollStateHandle,
```

Initialize it in `ProjectTreePanel::new` alongside the other view state:

```rust
clipped_scroll_state: Default::default(),
```

- [ ] **Step 2: Wrap only the tree body with the vertical scroller**

After the existing `body` conditional is built, create the scrollable body using the current theme and WarpUI defaults:

```rust
let scrollable_body = ClippedScrollable::vertical(
    self.clipped_scroll_state.clone(),
    body,
    ScrollbarWidth::Auto,
    theme.disabled_text_color(theme.background()).into(),
    theme.main_text_color(theme.background()).into(),
    Fill::None,
)
.finish();
```

Replace the outer `Shrinkable::new(1.0, body)` child with:

```rust
.with_child(Shrinkable::new(1.0, scrollable_body).finish())
```

Do not move the header or `render_unclassified_row` child into this wrapper; they must remain fixed.

- [ ] **Step 3: Run the focused test to verify it passes**

Run:

```bash
cargo test -p warp project_tree_scrolls_when_workspace_list_overflows --lib
```

Expected: PASS, with the last repository button position moving upward after the wheel event.

- [ ] **Step 4: Commit the minimal implementation**

```bash
git add app/src/project_organization/view/project_tree.rs
git commit -m "fix: make project tree vertically scrollable"
```

### Task 3: 完成回归验证

**Files:**
- Verify: `app/src/project_organization/view/project_tree.rs`
- Verify: `app/src/project_organization/view/project_tree_tests.rs`

- [ ] **Step 1: Run all project tree view tests**

Run:

```bash
cargo test -p warp project_organization::view::project_tree::tests --lib
```

Expected: all tests in the project tree test module pass.

- [ ] **Step 2: Check the workspace build**

Run:

```bash
cargo check
```

Expected: exit code 0 with no compile errors.

- [ ] **Step 3: Check formatting and the final diff**

Run:

```bash
cargo fmt --all -- --check
git diff --check
git status --short
git log -4 --oneline
```

Expected: formatting and diff checks pass; only the design/plan commits and the focused implementation commits are present, with no unrelated files modified.

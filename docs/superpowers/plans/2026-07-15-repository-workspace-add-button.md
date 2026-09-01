# Repository Workspace Add Button Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 让 repository 行内的创建 workspace 按钮在重绘后仍保留完整的点击与 tooltip 状态。

**Architecture:** `ProjectTreePanel` 在 `refresh_tree` 结束时，以当前树中的 repository/workspace ID 同步四组 `MouseStateHandle` 缓存。渲染路径只读取已初始化的稳定句柄，避免在渲染中创建状态。回归测试在 `LeftMouseDown` 与 `LeftMouseUp` 之间强制重建 scene，覆盖真实 UI 重绘时序。

**Tech Stack:** Rust 2021、WarpUI `Hoverable`/`MouseStateHandle`、`cargo test`、Cargo workspace。

---

## 文件结构

- 修改：`app/src/project_organization/view/project_tree.rs`
  - 在树状态刷新时同步动态行的鼠标状态句柄。
  - 渲染 repository/workspace 行时只读取缓存句柄。
- 修改：`app/src/project_organization/view/project_tree_tests.rs`
  - 在现有行内“+”UI 事件测试中插入一次 scene 重建，复现重绘后的鼠标释放。

### Task 1: 持久化动态行交互状态

**Files:**
- Modify: `app/src/project_organization/view/project_tree_tests.rs:205-265`
- Modify: `app/src/project_organization/view/project_tree.rs:1-6, 216-430, 472-510`

- [ ] **Step 1: 修改失败回归测试，模拟按下后的重绘**

在 `create_workspace_button_does_not_toggle_its_repository` 中，保留现有 `LeftMouseDown`，但在它和 `LeftMouseUp` 之间插入下列 scene 重建代码：

```rust
            presenter.borrow_mut().invalidate(
                WindowInvalidation {
                    updated: [root_view_id, project_tree.id()].into_iter().collect(),
                    ..Default::default()
                },
                ctx,
            );
            presenter
                .borrow_mut()
                .build_scene(vec2f(320., 160.), 1., None, ctx);
```

`presenter` 必须保持为 `Rc<RefCell<Presenter>>`，因此把现有的 `let presenter = Rc::new(RefCell::new(presenter));` 放在 `LeftMouseDown` 之前。`LeftMouseUp` 继续使用同一个 `presenter.clone()`。

- [ ] **Step 2: 运行测试并验证 RED**

Run:

```bash
cargo test -p warp create_workspace_button_does_not_toggle_its_repository --lib -- --nocapture
```

Expected: FAIL，`events` 为空而断言要求一个 `ProjectTreeEvent::CreateWorkspaceRequested`。当前 `unwrap_or_default()` 会在 scene 重建时替换鼠标按下状态的句柄。

- [ ] **Step 3: 在树刷新阶段同步句柄缓存**

在 `project_tree.rs` 顶部改为导入 `HashSet` 和 `Hash`：

```rust
use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};
```

在 `repository_add_workspace_position_id` 后添加私有 helper：

```rust
fn synchronize_mouse_states<Id>(
    mouse_states: &mut HashMap<Id, MouseStateHandle>,
    ids: &HashSet<Id>,
) where
    Id: Copy + Eq + Hash,
{
    mouse_states.retain(|id, _| ids.contains(id));
    for id in ids {
        mouse_states.entry(*id).or_default();
    }
}
```

在 `refresh_tree` 中 `self.state` 重建并恢复选择状态之后、`ctx.notify()` 之前，收集当前 ID 并同步全部四组缓存：

```rust
        let repository_ids = self
            .state
            .repositories()
            .iter()
            .map(|repository| repository.repository_id)
            .collect::<HashSet<_>>();
        let workspace_ids = self
            .state
            .repositories()
            .iter()
            .flat_map(|repository| repository.workspaces.iter())
            .map(|workspace| workspace.workspace_id)
            .collect::<HashSet<_>>();

        synchronize_mouse_states(&mut self.repository_mouse_states, &repository_ids);
        synchronize_mouse_states(
            &mut self.repository_add_workspace_mouse_states,
            &repository_ids,
        );
        synchronize_mouse_states(&mut self.workspace_mouse_states, &workspace_ids);
        synchronize_mouse_states(&mut self.workspace_delete_mouse_states, &workspace_ids);
```

把四个渲染位置的 `get(...).cloned().unwrap_or_default()` 改为 `expect(...).clone()`，分别使用下列错误信息：

```rust
"repository add-workspace mouse state should be initialized during tree refresh"
"repository row mouse state should be initialized during tree refresh"
"workspace delete mouse state should be initialized during tree refresh"
"workspace row mouse state should be initialized during tree refresh"
```

这样状态缺失会立即暴露，而不会在 render 路径静默创建一个无法跨重绘保存的 fallback 句柄。

- [ ] **Step 4: 运行针对性测试并验证 GREEN**

Run:

```bash
cargo test -p warp create_workspace_button_does_not_toggle_its_repository --lib -- --nocapture
```

Expected: PASS，日志显示 `ProjectTreeAction::CreateWorkspace` 被派发，测试断言收到唯一的 `CreateWorkspaceRequested` 事件，repository 保持展开。

- [ ] **Step 5: 运行 crate 级编译验证**

Run:

```bash
cargo check -p warp
```

Expected: exit code 0。现有未修改模块的 warning 可记录，但本次改动不得引入编译错误。

- [ ] **Step 6: 复查 diff 并提交**

Run:

```bash
git diff --check -- app/src/project_organization/view/project_tree.rs app/src/project_organization/view/project_tree_tests.rs
git diff -- app/src/project_organization/view/project_tree.rs app/src/project_organization/view/project_tree_tests.rs
```

确认只包含稳定鼠标状态缓存和回归测试后，提交本次代码：

```bash
git add app/src/project_organization/view/project_tree.rs app/src/project_organization/view/project_tree_tests.rs
git commit -m "fix: preserve repository workspace row interactions"
```

# Workspace 页签进树 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 把 repository workspace 的页签渲染成左侧树的子节点，侧栏打开时用 Git 信息栏替换顶部页签列表。

**Architecture:** 页签视图模型由 `Workspace::sync_project_tree` 计算，`ProjectTreePanel` 只渲染。父节点折叠时用子节点 activity 的 OR 画绿点，不再画聚合 agent 头像。信息栏谓词建立在现有通顶 chrome 之上。页签归属仍由 `RepositoryWorkspaceTabSets` 持有。

**Tech Stack:** Rust, WarpUI, 现有 `IconWithStatus` / `BreathingRing` / `get_repo_git_summary`

---

### Task 1: 页签节点类型与父节点活动槽

**Files:**
- Create: `app/src/project_organization/project_tree_tab.rs`
- Create: `app/src/project_organization/project_tree_tab_tests.rs`
- Modify: `app/src/project_organization/mod.rs`

**Steps:** 新增 `ProjectTreeTabId`、`TabNodeActivity`、`ProjectTreeTabNode`、`tab_node_activity`、`workspace_parent_activity_slot`。单测：agent 优先于绿点、父节点展开时空槽、折叠且有活动时绿点、父节点永不返回 Agent。

### Task 2: TabSets 按 workspace 列出全部页签

**Files:**
- Modify: `app/src/workspace/repository_workspace_tabs.rs`
- Modify: `app/src/workspace/repository_workspace_tabs_tests.rs`

**Steps:** 新增 `map_tabs`，对每个 repository workspace（含 inactive）按顺序映射全部页签。未归类不计。单测覆盖 active + inactive 顺序和 is_active 只标当前活动 workspace 的活动下标。

### Task 3: ProjectTreeState 三层可见行

**Files:**
- Modify: `app/src/project_organization/view/project_tree.rs`
- Modify: `app/src/project_organization/view/project_tree_tests.rs`

**Steps:** `WorkspaceTreeNode` 增加 `expanded` 与 `tabs`。`ProjectTreeRow` 增加 `Tab { workspace_id, tab_id }`。展开 workspace 时 `visible_rows` 含页签。`select_workspace` 自动展开。`toggle_workspace_expanded`。默认 expanded=true。单测三层可见行、折叠隐藏子节点、选中即展开。

### Task 4: sync_project_tree 下发 per-tab 节点

**Files:**
- Modify: `app/src/workspace/view.rs`
- Modify: `app/src/workspace/view/left_panel.rs`
- Modify: `app/src/project_organization/view/project_tree.rs`

**Steps:** Workspace 用 pane_group id、`display_title`、`tab_agent_activity`、long-running 组装 `ProjectTreeTabNode`。LeftPanel / ProjectTreePanel 增加 setter。呼吸环 handle 改按 tab id 缓存。父节点 `set_agent_activities` 不再用于头像。

### Task 5: 渲染页签子节点并改 workspace 行

**Files:**
- Modify: `app/src/project_organization/view/project_tree.rs`
- Modify: `app/src/project_organization/view/project_tree_tests.rs`

**Steps:** 缩进渲染 tab 行：活动槽、标题、hover 关闭。高亮画在 `is_active` 子节点上；展开的父节点不高亮。折叠且为当前 workspace 时父节点保留 accent 作为回退。父节点加 chevron（toggle，阻断冒泡）和 `+`。折叠父节点只画绿点。更新旧的 workspace-agent-on-parent 单测。

### Task 6: 树事件接到 Workspace

**Files:**
- Modify: `app/src/project_organization/view/project_tree.rs`
- Modify: `app/src/workspace/view.rs`

**Steps:** 新增 `TabSelected`、`TabCloseRequested`、`NewTabRequested`、`TabsReordered`。选中：必要时 switch workspace，再按 pane_group id activate。关闭：切到所属 workspace 后 `close_tab`；repository workspace 允许关掉最后一页变成空 workspace，不关窗。新建：先 switch 再 `AddDefaultTab`。同 workspace 重排更新 tabs 向量。

### Task 7: 信息栏替换顶栏页签列表

**Files:**
- Modify: `app/src/workspace/view/full_height_left_panel_chrome.rs`
- Modify: `app/src/workspace/view/full_height_left_panel_chrome_tests.rs`
- Modify: `app/src/workspace/view.rs`
- Modify: `app/src/project_organization/git.rs`

**Steps:** `use_workspace_info_bar = full_height_chrome && active_workspace_id.is_some()`。成立时 `render_tab_bar_contents` 不画页签和 `+`，中间画 `branch · from <upstream> · +n −n`。无 upstream 省略 from；+/- 均为 0 不画数字；Git 失败不造数字。复用 `get_repo_git_summary`，新增公开的 upstream 短名查询。只刷新当前 workspace。

### Task 8: 同 workspace 树内拖拽排序

**Files:**
- Modify: `app/src/project_organization/view/project_tree.rs`

**Steps:** 页签行使用现有 `Draggable`，drop 限制在同一 workspace 的兄弟节点。派发 `TabsReordered`。不接 `TAB_BAR_POSITION_ID` 拆窗路径。

### Task 9: 验证

运行针对性单测和 `cargo check -p warp`。

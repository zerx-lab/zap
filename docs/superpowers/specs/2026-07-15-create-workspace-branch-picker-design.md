# Create Workspace Branch Picker Design

## 背景

当前创建 workspace 弹窗将 `refs/remotes/origin/main` 等完整 remote ref 作为可编辑文本展示。长 remote ref 和长 worktree path 会撑破弹窗左侧边界；用户也容易手动输入无效 ref。弹窗打开时已经异步获取 repository refs，但结果仅回填到文本编辑器。

当前默认 worktree path 在分支选择前使用随机 workspace ID 生成，`New local branch` 和 `Workspace name` 没有初始值。

## 目标

- remote 分支通过可搜索的下拉选择，不允许手动编辑完整 remote ref。
- remote 分支默认只显示 branch name；只有多个 remote 含相同 branch name 时显示 `remote/branch` 消歧。
- 所选项在内部保留完整 refname，创建 Git worktree 时继续传递完整 `refs/remotes/<remote>/<branch>`。
- 成功加载 remote refs 后自动选择首项。
- 每次选择 remote 分支时，强制同步覆盖：
  - `New local branch`
  - `Workspace name`
  - `Worktree path`
- 三项默认值分别为 branch name、branch name、`{home}/.warp/worktrees/{repository name}/{branch name}`。
- 切换或选择 local branch 时，也以所选 local branch 同步 workspace name 和 worktree path。
- remote 获取失败时，local branch 模式保持可用；remote 模式显示错误、禁止创建，并提供 Retry。
- 长 branch name、worktree path 和错误文本必须受 modal 可用宽度约束，不得横向溢出或把内容推到左边界之外。

## 非目标

- 不修改 `fetch_and_list_refs_async`、Git ref 校验、worktree 创建或 SQLite 持久化协议。
- 不新增全局通用 branch picker；当前仅复用既有 `FilterableDropdown`。
- 不保留用户在切换 remote 分支前手动修改的三个派生字段，选择新分支总是覆盖它们。
- 不改变 remote 分支排序策略以外的 Git 行为。

## 方案

### 结构化 remote 选项

`CreateWorkspaceModal` 将 remote refs 转换为内部选项，包含：

- `full_ref`：提交给 `CreateWorkspaceSource::RemoteBranch` 的完整 ref。
- `remote`：remote 名称。
- `branch_name`：去除 `refs/remotes/<remote>/` 后的分支名。
- `display_label`：正常为 `branch_name`；当同一个 `branch_name` 出现在多个 remote 时为 `remote/branch_name`。

下拉 action 携带完整选项或稳定索引，显示文字不参与 Git ref 解析。分支名重复检测以 `branch_name` 分组，而非格式化字符串分组。

下拉使用现有 `FilterableDropdown`：加载中显示不可选的 loading 占位项；成功后填充可搜索选项并自动选中首项；remote 获取失败时禁用 remote 创建并显示 Retry 按钮。

### 默认字段同步

弹窗配置时接收 repository display name 与 worktree 根目录所需信息，不再预先使用随机 workspace ID 伪造最终路径。

remote/local 分支选中后调用同一派生逻辑：

1. 读取选中的 branch name。
2. 将 branch name 写入 remote 模式的 `New local branch`；local 模式的实际分支由所选 local branch 保持。
3. 将 `Workspace name` 设为 branch name。
4. 以 `workspace_dir_name` 生成 `{home}/.warp/worktrees/{repository name}/{branch name}` 并写入 `Worktree path`。

每次选择都覆盖用户此前对这三个编辑器的修改。提交时 remote 来源取结构化选项的 `full_ref`，local 来源取选择的 local branch。

### 加载与重试

异步 fetch 仍由 `Workspace` 协调。`CreateWorkspaceModal` 新增重试事件，`Workspace` 使用当前 repository path 再次调用 `fetch_and_list_refs_async`。modal 持有独立的 remote 加载状态与错误文本；不把 fetch 错误混入提交校验错误。

加载失败时：

- remote picker 不能打开或提交。
- Create 按钮在 remote 模式禁用或提交时返回清晰错误。
- 切换到 local branch 模式后，可继续选择并创建。
- Retry 重新进入 loading 状态并在成功后自动选择首个 remote 分支。

### 布局约束

所有 editor section 以 `Shrinkable` + `ConstrainedBox` + `Clipped` 包装 `ChildView<EditorView>`，让单行内容在固定 modal 可用宽度内裁剪/水平滚动。remote picker 的顶栏和菜单使用与 modal 内容一致的最大宽度。错误文本也置于受约束容器中，避免错误消息造成横向布局溢出。

## 测试策略

- remote option 映射：完整 ref 保持不变，普通项仅显示 branch name，同名跨 remote 项显示 `remote/branch`。
- 分支选择：remote 选择同步 new local branch、workspace name 和 worktree path；后续选择覆盖手动编辑值。local 选择同步 workspace name/path。
- 路径派生：使用注入或显式传入的 home/repository/branch，断言无随机 workspace ID。
- 加载状态：remote 加载失败时 remote 不能提交且发出重试事件；成功时自动选中首项。
- UI 布局：长内容通过受约束 editor/picker 路径渲染，测试固定宽度的 scene 不产生超出 modal 的输入宽度。

## 风险与兼容

- 多 remote 同名分支不再依赖展示文本解析，避免选错 ref。
- 非规范 remote ref 继续由现有 Git 层校验；UI 不绕过校验。
- 空 remote ref 列表保留 remote 模式的不可提交状态，用户仍可使用 local branch 模式或关闭弹窗。

# 接入已有 Worktree 设计

## 背景

当前创建 workspace 弹窗只能基于 remote 分支或本地分支创建新的 Git worktree。用户已经在本地创建的 linked worktree 不能直接出现在 repository workspace 列表中，必须重复创建或手动迁移。

## 目标

- 在创建 workspace 弹窗中新增“Use existing worktree”模式。
- 仅列出当前 repository 通过 `git worktree list` 注册的 linked worktree。
- 排除 repository 主目录和 detached HEAD worktree。
- 选择 worktree 后，使用其 canonical path 作为只读 Worktree path。
- 从 `refs/heads/<branch>` 解析 branch short name，作为可编辑 Workspace name 的默认值。
- 创建时不执行 `git worktree add`，只校验选中的 worktree 仍归属于当前 repository 后持久化 workspace。

## 非目标

- 不支持文件选择器或任意目录。
- 不支持接入 detached HEAD worktree。
- 不允许在“Use existing worktree”模式修改已选 worktree path。
- 不修改 remote 分支和本地分支创建新 worktree 的现有语义。

## 交互

创建弹窗保留现有“From remote branch”和“Use local branch”模式，并新增第三个 `SecondaryTheme` 按钮“Use existing worktree”。

进入或打开弹窗时，`Workspace` 异步调用现有 `list_worktrees_async(repository.path)`。modal 将结果转换为 `ExistingWorktreeOption`：

- `path`：已 canonicalize 的 linked worktree path。
- `branch_name`：去除 `refs/heads/` 前缀后的本地 branch name。
- `display_label`：branch name。

转换时剔除与 repository root 相同的主 worktree、`branch` 缺失的 detached worktree、以及不符合 `refs/heads/` 格式的 worktree。下拉加载时禁用；成功后按 `(branch_name, path)` 排序并启用；失败时该模式禁用并显示可重试错误。选择第一项不自动创建，但在用户切换到 existing mode 时自动选中第一项并回填字段。

用户选择一个候选项后：

- `Workspace name` 设置为 `branch_name`，用户仍可编辑。
- Worktree path 显示 canonical path，不能编辑。
- Create 按钮只在成功选择一个候选项后可用。

长 branch/path/error 文本继续使用现有 modal 的 480px 约束；picker 保持不被 `Clipped` 包围，避免截断 overlay。

## 数据与 Git 边界

`CreateWorkspaceSource` 新增：

```rust
ExistingWorktree {
    local_branch: String,
}
```

`CreateWorkspaceRequest.worktree_path` 保存选择的 canonical path。提交前，`Workspace` 调用新增的只读异步验证函数，重新执行 `list_worktrees`，要求同一 canonical path 仍存在且仍检出 `refs/heads/<local_branch>`。验证失败不持久化，向用户显示操作错误。

验证通过后，复用现有持久化、tab 切换与新终端页签创建流程；该 source 不调用 `create_from_remote_async` 或 `create_from_local_async`，也不执行任何 Git mutation。

首次加载、Retry 和所有 list worktree 回调与现有 remote refs 一样携带 `repository_id` 和 `workspace_id`，仅当 modal target 同时匹配时更新 UI，避免关闭或重新打开后写入陈旧结果。

## 错误处理

- worktree 列表加载失败：existing mode 禁用，错误位于 existing section，提供 Retry；remote/local 两种模式不受影响。
- 选择在提交前失效：验证失败，保持 modal 打开且不创建数据库记录或 terminal tab。
- 空列表：existing mode 无候选且不可提交，其他模式保持可用。

## 测试策略

- option 映射：排除主 worktree、detached/异常 branch，保留 canonical linked path，按 branch/path 稳定排序。
- 默认值：选择已有 worktree 时 Workspace name 使用 branch，path 不可被 form 改写。
- request：已有 worktree source 携带 local branch 和已选 path。
- Git 验证：有效注册 path/branch 通过；路径不存在、branch 已变更、主 worktree 和 detached worktree 被拒绝。
- Workspace 异步协调：陈旧 repository/workspace ID 的成功、错误与 Retry 结果均被忽略。
- 回归：remote/local 创建路径和已有 picker 行为继续通过。

## 验收标准

1. 用户能从下拉列表选择当前 repository 的 linked worktree。
2. 主目录与 detached worktree 不出现。
3. 选择后 path 固定为已注册路径，Workspace name 默认是 branch name。
4. 创建不会调用 Git worktree 创建命令，验证后仅注册 workspace 并打开 terminal。
5. 列表错误不会阻断 remote/local 模式，Retry 可重新加载。
6. 所有长内容在 modal 内保持受约束显示。

# Repository Local Workspace 设计

## 背景

Project Organization 当前可以保存 repository 和基于 Git worktree 的 workspace，但 repository 的主 worktree 没有统一进入 workspace 模型。用户新增 repository 后需要额外完成一次 workspace 创建；已有 repository 也缺少将主 worktree加入 workspace 的入口。

本功能为每个 repository 提供一个名为 `local` 的 workspace，并复用现有 workspace 创建弹窗完成已有 repository 的添加操作。

## 目标

1. 新增 repository 时自动创建名为 `local` 的 workspace。
2. 已有 repository 可以通过现有“创建 workspace”弹窗，将主 worktree 选择为 workspace。
3. `local` workspace 的显示名固定为 `local`，branch 字段记录主 worktree 当前实际分支，例如 `main` 或 `develop`。
4. 创建或添加成功后立即切换到该 workspace，并打开以 repository 根目录为初始目录的终端页签。
5. 重复操作不会创建重复 workspace，并返回明确错误。
6. 主 worktree 处于 detached HEAD 时拒绝操作，不创建 workspace，并显示明确错误。

## 非目标

- 不在项目树中增加独立的“添加 local workspace”按钮。
- 不改变现有普通 worktree workspace 的创建流程或命名规则。
- 不为 detached HEAD 自动生成临时分支。
- 不改变 Git repository 的分支、worktree 或文件内容。

## 用户界面

### 新增 repository

新增 repository 的成功流程直接创建 repository 和 `local` workspace。成功后当前 repository 被选中，应用切换到 `local` workspace，并打开 repository 根目录终端。

如果主 worktree 无法解析到当前分支（detached HEAD），新增操作失败，repository 和 workspace 都不应被持久化。

### 已有 repository

已有 repository 继续使用当前的 workspace 创建入口和弹窗。弹窗的“Use existing worktree”页需要把主 worktree作为一个可选项展示：

- 列表项使用主 worktree 的实际 branch 名，并标识其为主 worktree，例如 `main (local)`。
- 选择主 worktree 后，workspace name 默认填充为 `local`。
- 选择主 worktree 后，提交请求使用 repository 根目录作为 `worktree_path`，而不是执行 `git worktree add`。
- branch 字段使用选择时读取到的实际 branch 名。
- 主 worktree 为 detached HEAD 时不作为可提交的有效选项；弹窗在 existing worktree 区域显示明确的错误提示，说明主 worktree 当前没有分支，不能创建 `local` workspace，不能静默地把它当作普通 worktree。

普通已有 worktree 的展示、选择、名称编辑和创建行为保持不变。

## 领域模型与 Git 行为

`ValidatedRepository` 增加主 worktree 当前 branch 信息。repository 校验阶段读取主 worktree状态；无法读取 branch 或检测到 detached HEAD 时返回领域错误。

`ExistingWorktreeOption` 增加能区分主 worktree 的信息。`existing_worktree_options` 不再无条件排除 repository 根目录，但仍排除 prunable 或无法验证的 worktree。主 worktree使用 repository 根目录作为 path，并保留真实 branch 名；detached 主 worktree由弹窗的 existing worktree 状态错误单独呈现，不能提交。

创建普通 worktree workspace 时继续执行 `git worktree add`。创建主 worktree workspace 时不执行 `git worktree add`，只验证它仍然是当前 repository 的主 worktree，然后将其路径保存为 workspace path。

## 数据流与持久化

### 新增 repository

1. 校验 repository 根目录、Git 状态、remote/default branch 和主 worktree当前 branch。
2. 构造 repository 记录及 `local` workspace 记录。
3. 在同一个 SQLite transaction 中持久化两条记录；任一条失败都回滚整个操作。
4. 提交内存状态变更，并发布 repository/workspace 添加事件。
5. UI 使用返回的 workspace ID 完成切换和初始终端创建。

`local` workspace 的 path 为 repository 根目录，branch 为主 worktree当前 branch，workspace name 为 `local`。

### 已有 repository

1. 弹窗加载 repository 的 existing worktree options，其中包含主 worktree。
2. 用户选择主 worktree并提交。
3. model 校验 repository、path 和 branch 的唯一性。
4. 对主 worktree跳过 Git worktree 创建，直接持久化 workspace。
5. 复用现有 workspace 创建成功后的切换和初始终端逻辑。

workspace 唯一性沿用现有模型约束：同一 repository 下不能重复保存相同 worktree path 或相同 branch。若主 worktree已经对应 workspace，操作失败并保持原有数据不变。

## 错误处理

错误必须向用户暴露真实原因，不增加静默 fallback：

- 主 worktree detached HEAD：提示主 worktree 当前没有分支，无法创建 `local` workspace。
- 主 worktree不存在、不可验证或 Git 状态无效：返回现有 repository/worktree 校验错误。
- `local` workspace已存在或 path/branch冲突：提示 workspace 已存在或冲突，不执行重复持久化。
- 新增 repository 的 repository/workspace 原子保存失败：回滚数据库事务，并向用户显示保存失败原因。

## 验证范围

需要增加或更新以下测试：

- Git：主 worktree被列入 existing worktree options；显示真实当前 branch；detached 主 worktree返回明确错误。
- workspace modal：主 worktree可见；选择后默认名称为 `local`；请求使用 repository 根目录 path。
- model/persistence：新增 repository 自动生成 `local` workspace；repository 与 workspace 原子保存；重复 local workspace被拒绝且不产生部分数据。
- workspace UI：成功后切换到返回的 workspace，并在 repository 根目录打开初始终端；失败时不切换、不创建终端。

## 验收标准

1. 新增一个正常 checkout 的 repository 后，列表中同时出现 repository 和 `local` workspace，当前页面切换到该 workspace，并出现 repository 根目录终端。
2. 对已有 repository 打开创建 workspace 弹窗，在“Use existing worktree”页能看到并选择主 worktree；提交后得到 `local` workspace，且不生成额外 Git worktree。
3. 再次执行相同操作不会创建第二个 `local` workspace。
4. 对 detached HEAD 的主 worktree执行新增或添加操作时，用户看到明确错误，数据库和 UI 状态不发生部分变更。
5. 现有普通 worktree workspace 创建流程与测试保持通过。

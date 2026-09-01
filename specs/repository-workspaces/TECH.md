# Repository Workspaces 技术规格

## Context

本规格实现 [PRODUCT.md](./PRODUCT.md) 中的 repository → workspace 项目组织行为。

当前代码已有部分可复用能力，但缺少稳定领域模型：

- `app/src/projects.rs:21` 的 `ProjectManagementModel` 只按路径保存最近项目，无法表达 repository 元数据、workspace 或页签归属。
- `app/src/tab_configs/new_worktree_modal.rs:93` 提供一次性 worktree 创建表单；`app/src/tab_configs/tab_config.rs:417` 通过 TabConfig 生成 `git worktree` shell 命令，无法管理实例生命周期或多个页签。
- `app/src/tab_configs/repo_picker.rs:60` 在旧 `PersistedWorkspace` 下线后没有真实 repository 数据源。
- `app/src/workspace/view.rs:855` 的窗口根直接持有 `tabs: Vec<TabData>` 和单个 `active_tab_index`，多数 Tab/PaneGroup API 假设该向量就是当前可见集合。
- `app/src/workspace/view.rs:9665` 从当前可见 tabs 生成 `WindowSnapshot`；保存逻辑在 `app/src/persistence/sqlite.rs:1090` 后重建 windows/tabs 行，因此不能用长期稳定的外部记录引用临时 tab 主键。
- `crates/persistence/src/model.rs:312` 与 `crates/persistence/src/schema.rs:398` 的 tabs 目前只保存窗口、标题和颜色。
- Feature Flag 定义及通道列表位于 `crates/warp_features/src/lib.rs:9` 与 `crates/warp_features/src/lib.rs:710`。
- 所有新 Git 子进程必须使用 `crates/command/src/lib.rs:1` 提供的跨平台命令封装。

`app/src/workspaces/` 表示云端团队 workspace，与本功能不是同一领域。新内部类型使用 `RepositoryWorkspace`，避免与现有 `workspaces::Workspace` 和窗口根 `workspace::Workspace` 混淆。

## Proposed changes

### 1. Feature Flag 与模块边界

在 `crates/warp_features/src/lib.rs` 新增 `FeatureFlag::RepositoryWorkspaces`，并加入 `DOGFOOD_FLAGS`，不加入 Preview 或 Release 列表。

新增 `app/src/project_organization/`，按职责拆分：

- `domain.rs`: `RepositoryId`、`RepositoryWorkspaceId`、`Repository`、`RepositoryWorkspace` 和结构化错误。
- `model.rs`: repository/workspace 集合、活动操作状态、CRUD 事件和持久化事件桥接。
- `git.rs`: repository 校验、clone、fetch、ref 解析、worktree 创建、状态检查和删除。
- `migration.rs`: 旧 projects 与现有 Tab 快照的首次迁移和启动一致性检查。
- `workspace_agent_activity.rs`: 页签级活动类型、CLI/Oz 身份与绿点互斥决胜；折叠 workspace 父节点只用子节点 activity 的 OR 画通用绿点，不再渲染聚合头像。
- `view/project_tree.rs`: 左侧三层树（repository → workspace → 页签）和空态/错误态。
- `view/workspace_info_bar.rs`（或 `Workspace` 顶栏槽内的等价渲染）：侧栏打开且当前是 repository workspace 时替换 TabBar 中间的页签列表。

`Workspace::sync_project_tree` 下发：`tab_counts`、`active_workspace_id`、每个 workspace 的 `Vec<ProjectTreeTabNode>`（稳定 `PaneGroup` id、标题、per-tab activity、`is_active`）。`ProjectTreePanel` 不遍历 terminal / conversation。页签节点活动槽复用 `tab_agent_activity` 与现有 long-running 判定；身份跟该页签的 agent 走，不跟底层模型。折叠父节点有任一子节点活动时画绿点。详情见 `docs/plans/2026-08-31-workspace-tabs-in-tree-design.md`。

- `view/add_repository_modal.rs`: 本地目录与 Git URL 两种添加流程。
- `view/create_workspace_modal.rs`: 远端基线新建分支与本地分支关联流程。
- `view/delete_workspace_dialog.rs`: 安全删除、复选框和二次确认。

领域模型只依赖持久化事件和 Git 服务，不持有 TerminalModel。UI 通过现有 Workspace/PaneGroup API 创建或关闭页签。

### 2. 持久化模型与迁移

在 `crates/persistence/migrations/` 新增 migration，并由 Diesel 重新生成 `crates/persistence/src/schema.rs`。不得手改生成后的 schema 作为迁移替代。

新增表：

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
```

扩展现有表：

- `tabs.repository_workspace_id TEXT NULL REFERENCES repository_workspaces(id) ON DELETE SET NULL`
- `windows.active_repository_workspace_id TEXT NULL REFERENCES repository_workspaces(id) ON DELETE SET NULL`

`windows.active_tab_index` 继续保存当前活动集合的索引，并兼容未归类集合。`repository_workspace_window_states` 保存同一窗口中其他真实 workspace 的活动索引；未归类集合不创建关联行。

迁移分两阶段执行，保证每个提交都能独立构建和回滚。第一条 migration 创建新表并把旧 `projects` 路径复制到 `repositories`，暂时保留旧表；默认显示名在 Rust 首次加载时规范化为目录名，`source = 'local'`。代码消费者全部切换到新模型后，第二条 migration 删除旧 `projects` 表；该 migration 的 down 方向从 repositories 回填旧路径与时间戳。

### 3. Repository 与 Git 服务

Git 服务使用 `command::r#async::Command` 或 `command::blocking::Command`，所有参数单独传递。禁止通过 shell 拼接路径、URL、分支名或 ref。

repository 校验流程：

1. 规范化并 canonicalize 路径。
2. 使用 `git -C <path> rev-parse --show-toplevel` 验证工作树根。
3. 比较绝对 `--git-dir` 与 `--git-common-dir`，拒绝 linked worktree；保留现有 repository metadata 能力作为 UI 状态来源。
4. 读取 remote URL 和默认分支；失败返回结构化错误，不降级为普通目录。

Git URL clone 默认路径由 URL 的 repository 名生成，并在冲突时要求用户修改。clone 失败只删除本次调用创建的目录；调用前已存在的路径永不自动清理。

默认 worktree 目录名由新分支名生成安全 slug：将 `/`、`\\`、空白和非文件名安全字符折叠为 `-`，去除首尾分隔符，并追加 workspace UUID 的前 8 位。最终路径为 `~/.warp/worktrees/<repository-display-slug>/<branch-slug>-<short-id>/`，因此分支名不会创建意外的嵌套目录，重名 slug 也不会冲突。

分支列表使用完整 refname 分类：`refs/heads/` 与 `refs/remotes/<remote>/`。不得用字符串删除 `origin/` 前缀推断 ref 类型。创建弹窗打开时执行一次异步 `fetch --prune --quiet --no-tags`，然后用 `for-each-ref` 构造可搜索列表。

远端模式：

1. 验证选中的完整 remote ref 存在。
2. 验证新本地分支名合法且 `refs/heads/<name>` 不存在。
3. 原子创建最终目标目录并保留其 canonical path，创建失败时报告占用或 I/O 错误。
4. 执行唯一一条 `git worktree add --no-track -b <new-branch> <path> <remote-ref>` 命令；成功后验证直接 OID 和新分支不带 upstream。

本地模式：

1. 验证 `refs/heads/<branch>` 存在。
2. 解析 `git worktree list --porcelain`，若分支已检出则返回占用路径。
3. 与远端模式相同地原子创建最终目标目录，再执行 `git worktree add <path> refs/heads/<branch>`。

远端与本地创建都只会清理本次原子 claim 的、未注册且为空的目录。远端 `worktree add -b` 失败时绝不自动删除分支，因为无法安全断定该分支由本次调用创建。

### 4. 创建与补偿事务

`ProjectOrganizationModel` 为每个 repository 维护至多一个冲突操作状态，阻止并发 clone/fetch/create/delete。

创建顺序：

1. 完成 repository、ref、分支名、路径和重复 workspace 的全部只读校验。
2. 创建 worktree。
3. 在 SQLite transaction 中写入 `repository_workspaces`。
4. 将新 workspace 设为窗口活动 workspace，并通过 Workspace API 创建首个终端页签。

若步骤 3 或 4 失败，补偿顺序为删除新记录、移除本次创建的 worktree、删除本次创建的新分支。补偿失败时保留结构化错误并在启动一致性检查中显示残留状态；不得把失败的 workspace 标记为 ready。

### 5. Workspace 页签集合

保持 `app/src/workspace/view.rs` 中现有 `tabs` 和 `active_tab_index` 表示当前活动集合，新增：

```rust
struct RepositoryWorkspaceTabState {
    tabs: Vec<TabData>,
    active_tab_index: usize,
}

inactive_repository_workspace_tabs:
    HashMap<Option<RepositoryWorkspaceId>, RepositoryWorkspaceTabState>
active_repository_workspace_id: Option<RepositoryWorkspaceId>
```

`None` 表示未归类页签。切换时把当前 `tabs`/`active_tab_index` 与目标状态整体交换，因此现有基于 tab index 的渲染、PaneGroup、焦点和会话 API继续只处理活动集合。

新增统一遍历器供以下跨集合行为使用：

- WindowSnapshot 保存。
- 应用退出和窗口关闭时的终端清理。
- 跨窗口拖拽、全局会话定位和需要覆盖后台页签的管理操作。

新建页签读取 `active_repository_workspace_id`，为 TabData/TabSnapshot 写入归属，并将终端启动目录设为 workspace worktree。跨窗口拖拽在 `TransferredTab` 中携带 workspace id，目标窗口激活相同 workspace 后插入。

`WindowSnapshot` 扩展为包含各 workspace 的有序 TabSnapshot 集合和活动索引。SQLite 保存时扁平写入 tabs 行，并直接写 `repository_workspace_id`；恢复时按该字段重新分组。不得建立引用重建 tab id 的长期绑定表。

### 6a. 窗口 chrome 装配

`FeatureFlag::RepositoryWorkspaces` 开启且 `Workspace::is_left_panel_open` 为真、且不是简化 WASM 标题栏 / vertical tabs / mobile overlay 时，`Workspace::render` 使用：

```
row
  ├── ToolsPanel（通顶，SavePosition `LEFT_PANEL_POSITION_ID`）
  └── column
        ├── TabBar（`TAB_BAR_POSITION_ID`）
        └── 其余 panels（header items 已去掉 ToolsPanel）
```

判断与 padding 公式在 `app/src/workspace/view/full_height_left_panel_chrome.rs`。macOS 红绿灯避让从 TabBar 改到 `LeftPanelView.titlebar_leading_inset`；Windows/Linux 右侧红绿灯仍由顶栏右侧 padding 承担。`titlebar_height` 仍是整窗顶带 `TOTAL_TAB_BAR_HEIGHT`。

当通顶 chrome 成立且 `active_repository_workspace_id` 为 `Some` 时，内容列顶部中间换成 workspace 信息栏，页签列表不画在顶栏；窗控仍在右侧。侧栏收起、当前为未归类、或 Flag 关闭时恢复 TabBar。

侧栏收起或 Flag 关闭时恢复 `column(TabBar, panels)`。

### 6b. 树内页签

页签子节点的视图模型由 `Workspace` 计算，不进入 `ProjectTreePanel` 的 terminal 遍历：

```rust
struct ProjectTreeTabNode {
    tab_id: PaneGroupId, // 或现有稳定 PaneGroup identity
    title: String,
    activity: TabNodeActivity, // agent InProgress/Blocked | shell 长任务 | 类型图标
    is_active: bool,
}
```

`ProjectTreeEvent` 增加 `TabSelected`、`TabCloseRequested`、`NewTabRequested { workspace_id }`、`TabsReordered { workspace_id, from, to }`。`Workspace` 把这些事件映射到现有激活/关闭/新建/重排 API；`NewTabRequested` 在目标不是当前 workspace 时先做集合交换。

树行交互状态（hover、关闭按钮、拖拽）按页签稳定 id 缓存 `MouseStateHandle` / `DraggableState`，在 `refresh_tree` 后同步，避免重绘丢失点击。同一 workspace 内拖拽只改该 workspace 的 tabs 顺序；不得走跨窗口 `TAB_BAR_POSITION_ID` 拆窗路径。

渲染复用 `render_icon_with_status` 与 workspace 行同一 16px 活动槽宽。选中高亮画在 `is_active` 的子节点上。父节点 `+` 与现有 repository 行 `+` 同一套 `ActionButton` / tooltip 模式。

标题由 `Workspace::resolve_workspace_tab_label` 计算，纯规则在 `project_tree_tab.rs` 的 `resolve_terminal_tab_label` / `assign_idle_terminal_numbers`。每个 workspace 内空闲 terminal 从 1 编号。树和 TabBar 共用该结果；窗口标题仍走 `display_title`（cwd / 自定义名）。`PaneTitleUpdated` 和 `TerminalViewStateChanged` 都会 `sync_project_tree`，命令结束后树标题会更新。

### 6c. Workspace 信息栏

纯展示。数据：

- 分支：当前 `RepositoryWorkspace.branch`；与 Git HEAD 不一致时走现有树行错误态，不在信息栏另造分支。
- upstream：`branch_upstream`；空或 gone 则省略 `from`。
- `+n −n`：`get_repo_git_summary`（`git diff --shortstat HEAD` + untracked 文本行）。都为 0 不渲染数字。

只对当前可见 workspace debounce 刷新，失败时保留上一次成功值或留空，禁止显示 0 或猜测值冒充成功。Git 子进程走 `crates/command`。信息不栏没有 `+`、不列出页签。

### 6. 左侧树与弹窗

项目组织面板复用现有可调整宽度的左侧区域。Flag 启用时不渲染 Vertical Tabs，但不修改 `TabSettings::use_vertical_tabs` 的持久值；Flag 关闭后用户原设置恢复生效。

UI 遵循 `warp-ui-guidelines`：按钮使用现有 ActionButton/Button 主题，颜色来自 Appearance/Theme，图标按钮具有 Tooltip，不新增功能专用按钮主题。

树行显示：

- repository: 展开状态、显示名称、进行中/错误状态、添加 workspace 和更多菜单。
- workspace: 显示名称、页签数量、折叠时的通用绿点（有子页签活动时）、新建页签 `+`、hover 删除。展开后不在父节点画 agent 头像。
- tab: 活动槽、标题、hover 关闭；当前活动页签高亮。
- 底部固定“未归类页签”入口和数量。第一期不展开子页签。

创建弹窗使用 segmented control 切换两种模式。远端模式展示 remote branch、新分支名、自动生成、workspace 名称和 worktree 路径；本地模式展示 local branch、workspace 名称和路径。切换模式时清除不属于目标模式的选择，避免提交陈旧 ref。

### 7. 删除与外部状态变化

删除前置检查只生成给 UI 的建议快照；真正的删除校验在引用锁内执行：

1. workspace 记录、worktree 路径和目标分支仍相互一致。
2. `git status --porcelain` 为空；未跟踪文件同样阻止删除。
3. 若选择删除分支，判断该分支是否合并到已配置且仍然存在的 upstream；没有 upstream，或 remote-tracking ref 已消失（`upstream is gone`）时，判断是否合并到 repository 默认分支。

删除分支时，服务通过 prepared `git update-ref --stdin` transaction 锁定待删 branch 和非强制删除的 merge target。transaction 只队列 merge target 的 `verify` 与 branch 的 `delete <expected-oid>`，不会对同一 branch 同时队列 `verify` 和 `delete`。持锁后重新检查 worktree 注册路径、branch、dirty 状态、目标选择、目标 OID 与合并关系；强制删除不读取或锁定远端 merge target。

未合并状态在关闭终端或移除 worktree前触发二次确认。确认完成后依次关闭所属页签、移除 worktree、提交已 prepare 的 branch 删除 transaction，最后删除数据库记录。worktree remove 失败必须 abort transaction；若 worktree 已移除但 transaction commit 失败，服务返回带路径、branch、OID 和辅助 inspection 的明确部分状态，而不是推断或补偿删除 branch。

启动一致性检查验证 repository 路径、worktree 路径和 branch/ref。失效记录保留在模型中并带错误状态，UI 提供重新定位或移除记录；不自动选择其他路径或分支。

### 8. 旧数据迁移

首次加载新模型时：

1. 迁移旧 projects 为 repositories。
2. 遍历恢复后的 TabSnapshot，收集其中所有持久化 terminal cwd。
3. 通过 Git common directory 聚合到 repository；只有 external git directory 指向 linked worktree 的页签才自动创建/关联 `RepositoryWorkspace`。
4. 当一个页签中的所有可识别 Git cwd 都指向同一 linked worktree 时，该页签归入对应 workspace；同一 worktree 的页签共享一个 workspace，显示名默认取分支名。
5. 页签同时指向多个 repository/worktree、位于主工作目录、非 Git、缺失路径或解析冲突时，归入未分类集合。

迁移不执行 checkout、worktree add、目录移动或终端重启，并使用幂等唯一约束保证重复启动不会重复创建记录。

## Testing and validation

### Unit tests

- `app/src/project_organization/git_tests.rs`: 覆盖 PRODUCT 行为 4-7、11-18、29-35，包括完整 ref 分类、remote 基线新分支、本地分支已检出、脏 worktree、未合并分支、路径与 shell 特殊字符。
- `app/src/project_organization/model_tests.rs`: 覆盖行为 3、8-10、14、16、36、38，验证 CRUD、唯一约束映射和并发操作门禁。
- `app/src/workspace/repository_workspace_tabs_tests.rs`: 覆盖行为 19-25、42，验证集合交换、活动索引、后台实体保留、新页签归属、同 workspace 重排和跨窗口拖拽。
- `app/src/project_organization/view/project_tree.rs` 及相关 `*_tests.rs`: 覆盖行为 1、20、22、39-43 的树结构：子节点 per-tab 状态、折叠父节点无头像、点父节点恢复上次页签、hover 关闭、workspace 行 `+` 先切换再新建。
- `app/src/workspace/view/full_height_left_panel_chrome_tests.rs` 与信息栏谓词测试: 覆盖行为 1、44-45，验证通顶 chrome + 真实 workspace 才显示信息栏；侧栏收起 / 未归类 / flag 关恢复 TabBar；无 upstream 省略 `from`；`+/-` 为 0 或 Git 失败不造数字。
- `app/src/project_organization/migration_tests.rs`: 覆盖行为 26-28、34，使用临时 repository/worktree 验证幂等迁移和不确定状态进入未分类。
- persistence round-trip tests: 覆盖行为 24、27、38，验证多窗口、多 workspace、未分类 tabs 和 down/up migration。
- modal/view tests: 覆盖行为 1-2、10-18、29-37，验证模式切换、焦点、禁用状态、默认复选框和二次确认。

所有逻辑改动遵循 TDD：先添加失败测试并确认失败原因，再实现最小代码使其通过。

### Integration tests

使用 `crates/integration` Builder/TestStep 框架覆盖：

1. 添加本地 repository，创建远端基线 workspace，打开三个独立终端，切换 workspace 后验证进程仍存活。
2. 添加 Git URL repository，验证默认 clone 路径可修改并在重启后恢复。
3. 关联未检出的本地分支；对已检出分支显示占用路径并拒绝创建。
4. 恢复已有 linked worktree 页签和未分类页签。
5. 删除干净 workspace，分别验证保留分支、删除已合并分支和二次确认强制删除未合并分支。
6. 外部删除 worktree 后重启，验证错误状态而非静默 fallback。

### Commands

开发过程中运行最小针对性测试。交付前至少运行：

```bash
cargo nextest run --no-fail-fast --workspace --exclude command-signatures-v2
cargo check
```

若全工作区 nextest 受已知环境或无关失败阻塞，必须记录完整失败并补充所有受影响 crate 和 integration 测试的针对性结果，不得宣称全量测试通过。

## Risks and mitigations

- `Workspace` 大量代码直接访问当前 tabs 向量。通过活动/非活动集合整体交换保持其局部不变量，并为必须覆盖全部集合的少量生命周期路径增加统一遍历器，避免全文件改造成可见索引映射。
- Git 与 SQLite 无法形成真正原子事务。创建采用补偿操作，删除先做完整 preflight，并通过启动一致性检查暴露残留状态。
- 多窗口保存会重建 window/tab id。workspace 使用 UUID，Tab 归属直接随快照写入；窗口关联状态在同一保存 transaction 中使用新 window id 重建。
- 自动迁移可能遇到主仓库页签或不完整 cwd。仅对 linked worktree 建立确定关联，其余进入未分类，优先避免错误迁移。
- 项目组织面板与 Vertical Tabs 冲突。Flag 启用时只改变渲染选择，不覆盖用户设置，便于关闭 Flag 回滚。
- 树内拖拽若复用 TabBar 的 `TAB_BAR_POSITION_ID` 拆窗逻辑，会在第一期误开跨窗口拖。页签子节点拖拽必须限制在同一 workspace 的树节点范围内。
- 信息栏 Git 刷新过勤会打满 worktree IO。只订阅当前可见 workspace 的路径变更并 debounce，失败留空不重试打爆。
- 侧栏打开时没有顶栏页签，跨窗口拖和拆窗入口变少。PRODUCT 已把这些能力限定在侧栏收起后的 TabBar，测试需覆盖该回退路径。

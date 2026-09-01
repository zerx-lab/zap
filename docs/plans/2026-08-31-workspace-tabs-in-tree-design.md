# Workspace 页签进树 设计

## 背景

Repository Workspaces 已经把页签集合挂在 workspace 上，左侧树是 `repository → workspace`，顶部 TabBar 负责真正切页签。workspace 行用聚合后的最近 agent 头像表示活动，多个页签同时跑 agent 时会被误读成「这个 workspace 只在跑某一个 agent」。

参考 ThinkRail：`project → workspace → page` 三层树，选中落在子节点上；顶部不再列页签，改为展示当前工作区的 Git 信息。

本设计是 [页签 Chrome](../superpowers/specs/2026-08-25-repository-workspace-tab-chrome-design.md) 与 [Workspace Agent Activity](../superpowers/specs/2026-08-26-workspace-agent-activity-design.md) 的增量：页签导航从顶部栏下沉到树；agent 身份从 workspace 行下沉到页签子节点。产品行为以 `specs/repository-workspaces/PRODUCT.md` 为准。

## 已确认决策

- 点 workspace 父节点：切到该 workspace，恢复上次活动页签；未展开则展开，高亮对应子节点。
- 折叠时父节点不显示具体 agent 头像；只保留通用绿点（任一子页签有 agent InProgress/Blocked 或 shell 长任务）和页签数。
- 侧栏打开且当前是真正的 repository workspace：顶部 TabBar 换成信息栏。
- 侧栏收起，或当前是未归类：恢复顶部 TabBar。
- 信息栏第一期：当前分支名、`from <upstream>`、相对 HEAD 的 `+n −n`。不做 worktree 路径、内存、CPU。
- 新建页签入口：workspace 行上的 `+`。
- 页签子节点：hover 关闭；同一 workspace 内拖拽排序。跨 workspace 拖、拖出拆窗第一期只在侧栏收起后的 TabBar 上保留。
- 未归类页签第一期不改。
- Flag 仍是 `RepositoryWorkspaces`，不另加开关。
- Figma: none provided。

## 非目标

- 不在树上做跨 workspace 拖拽或拖出拆窗。
- 不把未归类改成页签子节点。
- 信息不栏不展示 worktree 路径或资源占用。
- 不叠多个头像、不加 +N。
- 不在树上双击重命名页签。
- 不改页签归属、持久化表结构或 Git worktree 生命周期。

## 树与 chrome

```text
repository
  └── workspace
        ├── tab
        ├── tab
        └── tab
```

侧栏打开且当前是 repository workspace：

```text
[ 通顶侧栏：树含页签 ] [ 信息栏：branch · from main · +n −n | 窗控 ]
                       [ 页签内容                               ]
```

侧栏收起或当前未归类：

```text
[ 通顶侧栏或收起 ] [ TabBar | 窗控 ]
                  [ 页签内容      ]
```

窗控、红绿灯、拖动窗口仍占用这条 34px 顶栏。`titlebar_height` 仍是整窗顶带。

空 workspace 没有子节点，父节点自己处于选中，右侧仍是现有空状态。关掉最后一个页签后 workspace 变空，不级联删除。

## 页签子节点

```text
[头像+环 | 绿点 | 类型图标]  页签标题                    [hover 关闭]
```

状态槽复用现有 TabBar / Vertical Tabs 的 `Indicator` / `IconWithStatus` 语义，不另造一套：

| 页签状态 | 左侧槽 |
|---------|--------|
| agent InProgress | 品牌头像 + 品牌色呼吸环 |
| agent Blocked | 同一头像 + 静态黄环 |
| agent 结束 / 取消 / 出错 | 头像立刻消失，回退到下面两行 |
| 无 agent、有 shell 长任务 | 6px 绿点 |
| 都没有 | 页签类型图标，与现在 TabBar 一致 |

- 标题与侧栏收起后的 TabBar 相同：自定义名、agent/对话标题、长任务进程名、最近有信息量的命令；空闲 terminal 按当前 workspace 编号为 Terminal 1、Terminal 2。不再用缩写路径当标题。
- **选中高亮在活动页签子节点上**，不在 workspace 父节点上。点父节点只是切 workspace 并高亮上次活动子节点。
- hover 右侧关闭；关闭规则跟现有页签相同（未保存确认等）。
- 同一 workspace 内拖拽排序。
- 右键沿用现有页签上下文菜单。重命名走现有快捷键。
- `+` 在 workspace 行：已是当前 workspace 则新建并激活；否则先切过去再新建。
- 后台 workspace 的子节点照常显示运行中状态。
- 快捷键（新建、关闭、Ctrl+Tab、Cmd+1..9）语义不变。

折叠的 workspace：不显示具体 agent 头像；任一子页签有活动时显示绿点；页签数仍在。

## 信息栏

```text
feature-593-multi_cloud  ·  from main  ·  +128  −34          [窗控]
```

- 分支名是 workspace 的真实 Git 分支，不是显示名。
- 有可用 upstream 时显示 `from <upstream 短名>`；没有、或 remote-tracking ref 已消失时省略，不编造。
- `+n −n` 相对 HEAD 的未提交改动（含暂存与 untracked 文本行），复用 `get_repo_git_summary`。都为 0 时不显示数字。
- Git 失败或 worktree 丢失：不显示假数字，树行走现有错误态。
- 只刷新当前可见 workspace，debounce，不阻塞渲染。

## 数据流

`Workspace::sync_project_tree` 除 tab count / 活动 workspace / 绿点外，再下发：

```text
workspace_id → [TabNode { id, title, activity, is_active }]
```

`id` 用 `PaneGroup` 的稳定 identity，不在 inactive 交换后失效。`ProjectTreePanel` 不遍历 terminal / conversation。父节点折叠绿点由子节点 activity 的 OR 得出，不再在父节点渲染 `WorkspaceAgentActivity` 头像。

新 `ProjectTreeEvent`：

- `TabSelected { workspace_id, tab_id }`
- `TabCloseRequested { workspace_id, tab_id }`
- `NewTabRequested { workspace_id }`
- `TabsReordered { workspace_id, from, to }`

点 workspace 仍走现有 `WorkspaceSelected`（恢复上次活动页签）。页签归属、顺序、活动下标继续由 `RepositoryWorkspaceTabSets` 持久化。

信息栏谓词 `use_workspace_info_bar` = 现有 `use_full_height_left_panel_chrome` **且** `active_repository_workspace_id` 为 `Some`。成立时顶栏中间换成信息栏；否则保持 TabBar。

Git：复用 `app/src/util/git.rs` 的 `get_repo_git_summary` 与 `app/src/project_organization/git.rs` 的 `branch_upstream`。所有 Git 子进程走 `crates/command`。

## 边界

- 切到其他 workspace 后，原 workspace 的子节点仍显示后台 agent / 长任务。
- 空 workspace：无子节点，父节点选中，`+` 仍可新建。
- 树上拖到其他 workspace / 拖出拆窗：第一期不做。
- 未归类：点它之后顶部仍是 TabBar，树不展开页签子节点。
- LeftPanel 切到文件树 / 搜索 / Drive 时仓库树被替换，信息栏谓词仍按「侧栏打开 + 当前 repository workspace」计算；顶栏不随工具页变成 TabBar，除非侧栏收起或切到未归类。

## 测试

单测：

- 折叠父节点无头像；有子页签活动时有绿点。
- 子节点 per-tab 状态：InProgress 头像、Blocked 黄环、结束后消失、shell 长任务绿点。
- 点父节点恢复上次活动页签并高亮该子节点。
- 点其他 workspace 的 `+`：先切换再新建。
- `use_workspace_info_bar` 谓词：通顶 chrome + 真实 workspace 为真；侧栏收起、未归类、flag 关为假。
- 无 upstream 省略 `from`；`+/-` 为 0 不显示；Git 失败不造数字。
- 同一 workspace 重排序更新 `RepositoryWorkspaceTabSets` 顺序。

视图：活动子节点高亮、父节点不高亮；hover 关闭；信息栏文案与窗控共存。

验证：相关 Rust 单测 + `cargo check`。

## 风险

- 树内拖拽排序要接现有 `Draggable` / 页签重排，不能误接成跨 workspace 拖。
- 信息栏 Git 轮询过勤会打满 worktree IO；必须只刷新当前 workspace 并 debounce。
- 16px 头像 + 缩进会挤标题；名称 ellipsis，不挤关闭按钮。
- 侧栏收起才有跨窗口拖页签，dogfood 用户可能暂时找不到该入口，PRODUCT 需写明。

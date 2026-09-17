# Repository Workspace 页签 Chrome 设计

## 背景

Repository Workspaces 已经把页签集合挂在 workspace 上：切换 workspace 时顶部 TabBar 只显示该 workspace 的页签。但窗口装配仍是「整窗宽 TabBar 在上、侧栏在下」：

```
column
  ├── TabBar          // 整窗宽标题栏
  └── row
        ├── ToolsPanel
        └── 页签内容
```

页签在视觉上属于窗口，不属于当前 workspace，和 Superset「侧栏通顶、页签在内容列」的从属关系不一致。

本设计只改窗口 chrome 装配，不改页签归属、持久化或 Git。

## 已确认决策

- 侧栏保持 Repository → Workspace 双层树，不以 workspace 作为一级导航。
- 侧栏打开时通顶；TabBar 只出现在右侧内容列。
- LeftPanel 仍是互斥工具页：切到文件树 / 搜索 / Drive 时仓库树被替换，chrome 不变。
- 侧栏收起时 TabBar 恢复通栏。
- 实现采用「上提 ToolsPanel，改 `Workspace::render` 主 flex」，不抽独立 Chrome 组件，不用 overlay 对位。
- 右侧栏（Code Review / AI assistant）仍在 TabBar 下方，和内容并排，不通顶。

## 目标

当 `RepositoryWorkspaces` 开启且左侧 ToolsPanel 打开时，窗口结构变为：

```
row
  ├── ToolsPanel      // 通顶，高度 = TabBar + 内容
  └── column
        ├── TabBar    // 只覆盖内容列
        └── 页签内容（不再含 ToolsPanel）
```

使当前 workspace 的页签出现在选中 workspace 的右侧，形成从属关系。Flag 关闭或侧栏收起时，视觉和行为都回到旧窗口。

## 启用条件

新增纯函数 `use_full_height_left_panel_chrome(app) -> bool`，同时满足才启用：

1. `FeatureFlag::RepositoryWorkspaces` 开启。
2. 当前窗口左侧 ToolsPanel 打开（`Workspace::is_left_panel_open`）。
3. 不是 WASM 简化标题栏（Drive 对象 / 共享会话 / transcript）。
4. 不是 vertical tabs（项目组织模式已强制水平 TabBar）。
5. 不是 mobile overlay 侧栏。

任一条件不满足，整棵树维持现状。

## 布局装配

改动集中在 `app/src/workspace/view.rs`。

`Workspace::render` 在新 chrome 下改为外层 `Flex::row`：第一个 child 是 `ChildView(left_panel_view)`，第二个 child 是 `column(TabBar, 其余 panels)`。旧路径保持 `column(TabBar, panels)`。新 chrome 下 ToolsPanel 始终通顶出现在窗口左侧，即使它在 header toolbar 配置里被放到右侧或不在 chip 列表中；只要 `is_left_panel_open` 为真就上提。

`render_banner_and_active_tab` 和 `render_empty_workspace_content` 遍历 `header_toolbar_chip_selection` 的 left 与 right items 时，若新 chrome 已启用，跳过 `HeaderToolbarItemKind::ToolsPanel`，避免侧栏画两次。其它 items 仍留在内容行。

`TAB_BAR_HEIGHT` 与 LeftPanel `PANE_HEADER_HEIGHT` 均为 34。侧栏工具条与右侧 TabBar 齐平，成为同一条顶栏：左为红绿灯避让 + 工具切换 + 关闭，右为页签和窗口按钮。`ProjectTreePanel` 的 “Repositories” 头仍在工具条下面，不进入标题栏。

侧栏宽度、拖拽改宽、header toolbar chip 配置继续用现有 `Resizable` 与 `HeaderToolbarChipSelection`。页签归属、`RepositoryWorkspaceTabSets`、workspace 切换和持久化不改。

## 红绿灯、拖拽、命中测试

窗口拖拽带继续 `set_titlebar_height(TOTAL_TAB_BAR_HEIGHT)`，这是整窗顶上一整条，不跟 TabBar 等宽。warpui 只在顶带内未被控件 handle 的 `LeftMouseDown` 上拖窗 / 双击最大化。工具切换、关闭侧栏、页签、`+`、设置、侧栏宽度拖条已经 handle 事件；侧栏头和 TabBar 的空白处仍拖窗口。

红绿灯仍贴窗口角，不贴 TabBar：

| 平台 | 位置 | 新 chrome（侧栏打开） | 侧栏收起或 flag 关 |
|------|------|----------------------|-------------------|
| macOS | 系统按钮在窗口左上，不随 zoom 缩放 | 避让 padding 从 TabBar 改到 LeftPanel 头；TabBar 左侧不再留红绿灯宽 | TabBar 左侧维持现有 padding |
| Windows / Linux | 自绘按钮叠在窗口右上 | TabBar 仍占据窗口右上，右侧 padding 不动 | 维持现状 |

`compute_tab_bar_left_padding` 在新 chrome 下不为 macOS 红绿灯留空。`LeftPanelView` 增加 `titlebar_leading_inset: f32`，由 Workspace 在渲染前写入 `TrafficLightData::width(zoom_factor)`（全屏 macOS 为 0），工具条头左侧加上该 inset。Windows / Linux 的 `maybe_render_traffic_lights` 仍锚在窗口 `TopRight`。

TabBar 的 Y 仍是窗口顶；X 从侧栏宽度之后开始。跨窗口拖页签、拖离拆窗继续用 `TAB_BAR_POSITION_ID` 的实测 bounds，不写死 `x = 0`。若 `DETACH_SENSITIVITY` 或相关 `drag_y` 逻辑假定 TabBar 通栏，改为相对该 SavePosition。

padding 与 flex 结构必须同一帧切换，避免某一帧控件叠在红绿灯上。

## 边界情况

- 空 workspace：右侧仍是 TabBar（含 `+`）和现有空状态。`render_empty_workspace_content` 同样跳过 ToolsPanel。
- 未归类页签：仍在树底部。选中后右侧 TabBar 换成未归类集合，chrome 不变，不加额外空态条。
- 侧栏开 / 关：一次 layout 切换，不做独立 chrome 动画。页签数据、活动页、滚动位置不变。
- 切文件树 / 搜索 / Drive：chrome 不变，只换侧栏内容。
- 多窗口：每个窗口使用自己的 `left_panel_open`。
- 主题选择器：仍在 TabBar 下方的内容行，不通顶。
- 跨窗口拖页签的 drop target 仍是 TabBar 元素本身。
- 侧栏拖宽：内容列和 TabBar 随 flex 变窄。

## 规格同步

实现时更新：

- `specs/repository-workspaces/PRODUCT.md` 行为 1：侧栏打开时 TabBar 只出现在内容列；侧栏收起时恢复通栏。项目组织模式仍不显示 Vertical Tabs。
- `specs/repository-workspaces/TECH.md`：补窗口装配（LeftPanel 上提、ToolsPanel 去重、红绿灯 padding）。不改领域模型、持久化或 Git 章节。

## 测试

`use_full_height_left_panel_chrome` 用真值表单测：flag、侧栏开、简化 WASM、vertical tabs 的组合。

另外覆盖：

1. 新 chrome 时 inner left/right items 都不含 `ToolsPanel`；关闭侧栏或关 flag 时按原配置包含。
2. 新 chrome 时 `compute_tab_bar_left_padding` 不含 macOS 红绿灯宽；侧栏收起后含。
3. 空页签路径同样省略 ToolsPanel。
4. 现有 `repository_workspace_mode_keeps_setting_but_uses_horizontal_tabbar` 继续通过。

不引入截图测试。验证以相关单测和 `cargo check` 为准。

## 非目标

- 不把仓库树改成常驻主导航或活动栏。
- 不把页签嵌进 workspace 树节点。
- 不抽 `WorkspaceChrome` 布局组件。
- 不用 overlay / 负 margin 把侧栏叠进标题栏。
- 不改页签归属、workspace 切换、持久化、Git worktree。
- 不让右侧栏通顶。
- 不修改用户的 Vertical Tabs 设置值。
- 不引入 chrome 开合动画或截图测试。

## 验收标准

- Flag 开且侧栏打开时，侧栏从窗口顶铺到窗口底，TabBar 只出现在侧栏右侧。
- 选中 workspace 后，其页签出现在该 workspace 的右侧内容列，不再横跨整个窗口。
- 侧栏收起或 Flag 关时，TabBar 恢复通栏，红绿灯不被页签或工具按钮挡住。
- 切到文件树等其它左侧工具时，chrome 保持通顶侧栏 + 内容列 TabBar。
- 空 workspace、未归类页签、跨窗口拖页签、窗口拖拽和侧栏改宽仍可用。
- 相关单测和 `cargo check` 通过。

# Workspace Terminal Activity Indicator 设计

## 背景

用户希望 terminal tab 的运行状态能反馈到 workspace 层级。例如某个 terminal 正在执行长期任务时，workspace 名称旁展示一个现代化、极客风的运行状态，帮助用户在多个窗口之间快速知道当前 workspace 下有任务在跑。

当前 workspace 名称旁已有 tab 数量统计，但视觉表现较粗糙。本设计保留“数字表示 workspace tab 数量”的语义，替换旧 UI；terminal long-running 状态只控制动画和发光是否启用。

现有代码已经具备 terminal 长任务状态链路：

- `TerminalViewState::LongRunning` 表示 terminal 当前有长任务。
- `TerminalView::is_long_running()` 能读取当前 terminal 是否长任务中。
- `TerminalViewStateChanged` 会从 `TerminalView` 上抛到 `PaneGroup`，再触发 `Workspace` 刷新。
- `SessionNavigationData::all_sessions(ctx)` 与 `RunningSessionSummary` 已用于跨窗口 running session 汇总。

本设计复用这些能力，不新增 PTY 输出监听，不轮询 terminal 输出。

## 目标

- 在 workspace 名称旁用新的 C3 Neon Capsule 展示 workspace tab 数量。
- 移除 workspace 名称旁现有 tab 数量统计的旧 UI，保留其“tab count”数值语义。
- 为选中状态的 workspace 外围增加现代化、极客风的光影效果。
- 当当前 workspace/团队下任意窗口存在 long-running terminal 时，启用 spinner 动画、边框高亮和弱辉光。
- 使用 C3 Neon Capsule 视觉方向：圆角胶囊、弱青绿色辉光、细旋转状态环、数量 badge。
- 只做纯展示，不提供点击、hover tooltip 或导航交互。
- 空闲时继续展示 tab 数字，但不显示 spinner 动画、不启用高亮辉光。
- 保持现有 tab indicator 语义不变：tab indicator 仍表示单 tab 状态，workspace chip 只表示 workspace 聚合状态。

## 非目标

- 不统计“最近几秒正在输出内容”的 PTY 活跃度。
- 不改变 long-running command 的判定阈值。
- 不引入新的 command lifecycle 状态。
- 不新增 workspace 级任务管理面板或 running process 列表入口。
- 不替换现有 tab、vertical tabs、pane header 的状态指示。
- 不让数字表示 long-running terminal 数量。

## 用户确认的设计选择

- 位置：workspace 名称/标题 chip 旁。
- 触发规则：使用现有 long-running command 状态。
- 数字统计范围：沿用当前 workspace 的 tab count。
- 动画统计范围：当前 workspace/团队下所有窗口的 long-running terminals。
- 交互：纯展示，不点击、不 hover。
- 视觉：C3 Neon Capsule，极客风但保持克制。
- 数字语义：表示 workspace 中的 tab 数量，不表示 long-running terminal 数量。
- Running 状态：只控制 spinner/辉光动画，空闲时数字仍显示。
- 选中状态：workspace 容器外围增加独立光影，不复用 running 的 spinner/绿色脉冲。
- 旧 UI：移除 workspace 名称旁现有 tab count 的丑旧渲染，替换为新的 C3 数字 badge。

## UI 设计

新的 tab count 状态出现在 workspace 名称旁：

```text
Team Lab  ⟳  3
```

渲染细节：

- 胶囊高度约 24px，匹配标题栏控件高度。
- 数字 badge 固定最小宽度，避免 1 位到 2 位数字造成明显跳动；该数字只表示 workspace tab 数量。
- tab 数量 `1..=99` 显示精确值，超过 99 显示 `99+`。
- 沿用现有 workspace tab count 的可见性规则，但旧 tab count UI 必须被新 badge 替换；不允许两个数字同时出现。
- running 状态激活时：胶囊使用半透明深色底、低透明度青绿色边框、弱辉光，并显示细环 spinner 动画。
- 空闲状态时：保留同一个数字 badge，去掉 spinner 动画，边框和背景降噪，不显示辉光。
- 发光效果必须弱化，避免长期运行任务时干扰注意力。

选中状态光影：

- 选中 workspace 的整体容器外围增加静态电蓝/冷白光影，表现为细描边、内侧高光和非常弱的外发光。
- 选中光影必须落在 workspace 容器层，不落在数字 badge 层；数字 badge 仍只负责 tab count 与 running 动画。
- 选中效果不旋转、不闪烁、不使用绿色脉冲，避免与 long-running 状态混淆。
- long-running 效果使用 badge 内 spinner 和青绿色弱辉光；selected 效果使用容器外围静态电蓝/冷白边缘光。
- 当 workspace 同时 selected 且 has running terminal 时，两层效果同时存在：容器外围显示 selected 光影，数字 badge 显示 spinner/青绿色 running 状态。
- 未选中但 has running terminal 的 workspace 不显示 selected 外围光影，只显示数字 badge 的 running 动画。

若标题栏空间不足：

- 保留 workspace 名称的现有截断策略。
- 活动状态保持固定尺寸约束。
- 不挤压右侧固定 controls；必要时只截断 workspace 名称，不截断数字 badge。

## 数据流

tab count 数据：

1. Workspace 复用现有 workspace tab 数量统计来源。
2. 标题栏重新 render 时读取当前 workspace 的 tab count。
3. tab count 通过新的 C3 badge 渲染；数字不读取 running terminal 数量。

selected 状态：

1. Workspace 渲染时读取现有 active/selected workspace 状态。
2. 当前 workspace 被选中时，对 workspace 容器启用 selected 光影样式。
3. selected 光影不改变 tab count 数字，也不改变 running presence 计算。

running 动画状态：

1. Terminal 命令进入 long-running 状态。
2. `TerminalView` 设置 `TerminalViewState::LongRunning` 并发出 `TerminalViewStateChanged`。
3. `TerminalPane` 将事件上抛为 `pane_group::Event::TerminalViewStateChanged`。
4. `Workspace` 已有事件处理会调用 `update_active_session(ctx)` 与 `ctx.notify()`。
5. Workspace 标题栏重新 render 时计算当前 workspace 是否存在 long-running terminal。
6. 存在 long-running terminal 时启用 spinner/辉光；否则只显示静态 tab count badge。

命令完成后：

1. `TerminalView` 回到 `TerminalViewState::Normal` 或 `Errored`。
2. 同一事件链触发 workspace render。
3. 动画状态关闭；tab count 数字保持显示。

## 聚合规则

聚合分为两个独立语义：tab count 与 running presence。

tab count：

- 数字应复用当前 workspace 名称旁已有 tab 数量统计的数据来源。
- 如果当前旧 UI 统计的是 repository workspace 的 active/inactive tabs，则新 UI 沿用同一统计范围。
- 不重新用 terminal session 数量推导 tab count。

running presence：

- 入口应优先复用 `SessionNavigationData::all_sessions(ctx)` 与 `RunningSessionSummary`。
- 只输出布尔值 `has_running_terminal`，不把 running session 数量暴露给 UI 数字。

running presence 计入规则：

- 只统计 `CommandContext::RunningCommand` 和 `CommandContext::RunningAIBlock`。
- 不统计 read-only terminal。
- 不统计 shared session viewer。
- 同一 workspace/团队下跨窗口统计。

workspace 归属过滤：

- 以 `UserWorkspaces::current_workspace_uid()` 作为当前 workspace/团队归属基准。
- 如果现有 `SessionNavigationData` 无法直接表达 workspace uid，则新增明确的 workspace 归属字段或筛选入口。
- 不通过窗口标题、workspace 名称字符串、tab title 或 command text 做推断。
- 如果实现时无法通过结构化字段确定 session 的 workspace 归属，则停止实现并回到设计评审；不允许用“默认不计入”或字符串推断掩盖数据模型缺口。

## 组件边界

建议新增两个小边界：

- `WorkspaceTerminalActivitySummary`：负责汇总 `tab_count` 与 `has_running_terminal`。
- `WorkspaceVisualState`：负责描述 `is_selected` 与 `has_running_terminal` 组合后的视觉状态。
- `render_workspace_activity_chip(...)`：负责 tab count badge UI 渲染，不持有业务状态。
- `render_workspace_selection_frame(...)` 或等价 helper：负责 selected workspace 外围光影，不持有 running 业务状态。

职责约束：

- 聚合逻辑不进入渲染 helper。
- UI helper 不直接遍历 terminal sessions。
- 不在 terminal 输出路径中更新 workspace 状态。
- 数字必须复用旧 tab count 的数值含义；旧 tab count 的视觉渲染路径应被移除或替换。
- `has_running_terminal` 只能控制动画/高亮，不影响数字值。
- `is_selected` 只能控制 workspace 容器外围光影，不影响数字值，也不启停 spinner。
- 不新增全局 mutable cache，除非实现时发现 render 阶段重复遍历造成明确性能问题。

## 边界情况

- workspace 有 3 个 tabs 且没有 long-running terminal：显示静态 `3`，不显示 spinner/辉光。
- workspace 有 3 个 tabs 且存在 1 个或多个 long-running terminals：仍显示 `3`，并启用 spinner/辉光。
- workspace 被选中但没有 long-running terminal：显示 selected 外围光影和静态 tab count badge。
- workspace 被选中且存在 long-running terminal：selected 外围光影与 badge running 动画同时显示，颜色和动效保持区分。
- workspace 未选中但存在 long-running terminal：不显示 selected 外围光影，只显示 badge running 动画。
- 多窗口同一 workspace 有 running sessions：只影响动画状态，不改变 tab count 数字。
- 多窗口不同 workspace 的 session：只计入当前 workspace/团队归属一致的 running presence。
- read-only shared session viewer：不计入。
- tab 数量超过 99：显示 `99+`。
- terminal 从 long-running 变为 errored：关闭 workspace activity 动画；tab indicator 仍可展示错误状态。
- vertical tabs 模式和 horizontal tabs 模式都应使用同一聚合规则。

## 测试计划

单元测试：

- workspace tab count 为 3 时，summary 的 `tab_count` 为 3。
- 存在 long-running session 时，summary 的 `has_running_terminal` 为 true，但 `tab_count` 不变。
- read-only session 不会让 `has_running_terminal` 变为 true。
- shared session viewer 不会让 `has_running_terminal` 变为 true。
- `99+` 显示格式正确。

视图测试：

- 没有 long-running terminal 时渲染静态 tab count badge。
- 有 long-running terminal 时渲染同一个 tab count badge，并启用 spinner/辉光。
- selected workspace 渲染外围光影。
- selected workspace 的光影不启用 spinner。
- selected 且 has running terminal 时，同时渲染外围 selected 光影和 badge running 动画。
- tab count badge 显示 tab count，而不是 running terminal count。
- 旧 tab count UI 不再渲染，避免同一位置出现两个数字。
- horizontal tabs 与 vertical tabs 模式下渲染入口一致。

验证命令：

- 运行相关 Rust 单测。
- 至少运行 `cargo check`。

## 风险

- 当前 workspace/团队归属如果是全局 singleton，而不是每个窗口独立状态，跨窗口统计应明确表示“当前选中 workspace/团队”的全局运行态。
- C3 视觉存在感较强，实现时必须使用低透明度边框和弱辉光，避免标题栏变成常亮状态灯。
- selected 光影和 running 光影如果使用相同颜色或相同动画，会造成语义混淆；实现时应固定 selected 为静态电蓝/冷白外围光，running 为 badge 内 spinner/青绿色弱辉光。
- 标题栏挂载点必须先定位到现有 workspace 名称/tab count 渲染入口；若该入口分散在 horizontal tabs 与 vertical tabs 两条路径中，应先抽出共享渲染 helper，再移除旧 tab count 并接入 activity 状态，避免在大型 `Workspace::render_tab_bar_contents` 中堆叠重复逻辑。
- 最大语义风险是把 running terminal 数量误接到数字 badge；实现时应通过类型命名区分 `tab_count` 与 `has_running_terminal`。

## 验收标准

- workspace 名称旁显示新的 C3 数字 badge，数字表示 workspace tab 数量。
- 选中的 workspace 外围显示极客风静态光影效果。
- 当同一 workspace/团队下任意窗口有 terminal long-running command 时，同一个 badge 启用 spinner/辉光。
- selected 光影和 long-running badge 动画在颜色、位置和动效上有明确区分。
- 所有 long-running command 结束后，spinner/辉光消失，tab count 数字保留。
- workspace 名称旁不再显示旧 tab 数量统计 UI；新数字 badge 只代表 tab 数量。
- 点击或 hover 胶囊没有新增交互。
- 现有 tab indicator、pane header、running process warning 行为不变。

# Workspace Agent Activity 设计

## 背景

Repository Workspaces 的左侧树已经能把 terminal 长任务反馈到 workspace 行：名称左侧有 6px 绿点，右侧是 tab count。绿点只回答「有没有长任务」，看不出是哪个 agent 在跑。

页签层已经认识 agent 身份和会话状态：

- CLI 会话通过 `CLIAgent` 提供品牌图标（Claude / Codex / Grok 等）。
- 原生 Oz / Warp Agent 通过 `Indicator::Agent` 与 `ConversationStatus` 提供运行 / 阻塞 / 结束。
- 通知与竖直页签已经用 `IconWithStatus` 画圆形品牌头像。

用户希望：某个 workspace 的页签里有 agent 在跑时，把该 agent 的品牌图标反馈到对应 workspace 行，并让 UI 能看出「正在跑」或「卡住等审批」。切到别的 workspace 后，后台仍在跑的 agent 必须继续显示在原来那一行。

本设计是 [Workspace Terminal Activity](2026-07-20-workspace-terminal-activity-design.md) 的增量：绿点语义保留，作为「非 agent 长任务」的回退；agent 活动用更富的头像槽替换绿点。

## 已确认决策

- 视觉：16px 圆形品牌头像替换绿点；InProgress 时头像外一圈 1.5px 品牌色细环做透明度呼吸，不旋转。
- 阻塞：头像留下，环改成静态黄，不呼吸。
- 结束 / 取消 / 出错：头像和环立刻消失；出错继续由页签 indicator 承担。
- 聚合：统计该 workspace 全部页签（含切走后仍挂在后台的 inactive tabs），不是只看当前聚焦页签。
- 多 agent：只显示最近进入 InProgress / Blocked 的那个。首期不做叠头像、不做 +N。
- 身份：跟页签已经认识的 agent 走。CLI 用 `CLIAgent` 品牌图标；原生 Oz / Warp Agent 用 Oz（ambient 用 OzCloud）。不按底层模型（例如 Grok 4.6）显示品牌。
- 绿点与头像互斥：有 agent 活动时只显示头像；没有 agent 活动但有 shell 长任务时显示绿点。
- 交互：纯展示。点击 workspace 仍是切换 workspace，不新增「点头像跳到那个 agent 页签」，不加 tooltip 列表。
- tab count 语义不变，仍表示页签数。

## 目标

- 在 workspace 行左侧活动槽展示正在跑或阻塞中的 agent 品牌身份。
- 用呼吸环表示 InProgress，用静态黄环表示 Blocked，两者在未选中的 workspace 上也可见。
- 没有 agent 活动时，保持现有绿点 / 空槽行为。
- 选中高亮与活动环独立，颜色不混用。
- 行高和左侧槽宽度在空槽、绿点、头像之间保持稳定，避免树左右跳。

## 非目标

- 不叠多个头像，不加 +N。
- 不按 Oz 对话的底层模型显示品牌。
- Error / Cancelled 不残留头像。
- 不做整圈旋转环，不复用角标 ClockLoader 作为本槽的运行态。
- 不改变 tab、vertical tabs、pane header 的 indicator。
- 不改变 long-running shell 的判定阈值。
- 不新增 workspace 级任务管理或运行中会话列表。
- 不让 tab count 表示 agent 数量。

## UI

Workspace 行左侧现有活动槽（绿点位置）升级为同一宽度的身份槽：

```text
[头像+环]  feature-595-multi-cloud          2
[绿点    ]  feature-597-task-complete-validate  0
[空槽    ]  feature-596-check-complete         0
```

渲染细节：

- 头像直径 16px，圆形，复用 `IconWithStatus` / `render_cli_agent_logo` 的品牌圆形画法。
- 呼吸环：1.5px 描边，颜色取该 agent 品牌色（Oz 用 theme accent），透明度约 0.4 → 1.0，周期约 1.6s。只给当前可见且处于 InProgress 的行挂呼吸；Blocked 静态，不每帧重绘。
- Blocked 环使用 theme 警告黄（与页签 / 对话 Blocked 色一致），不呼吸。
- 空槽、绿点、头像占用同一约束宽度（按 16px 头像对齐），绿点在槽内居中。
- 右侧 tab count、hover 删除按钮、选中行强调色保持现状。
- 选中是名字 / 行的 accent；运行环是品牌色；阻塞环是黄。三套颜色不得复用。

状态表：

| workspace 活动 | 左侧槽 | 环 |
|----------------|--------|----|
| agent InProgress | 品牌头像 | 品牌色呼吸环 |
| agent Blocked | 同一头像 | 静态黄环 |
| 无 agent 活动，有 shell 长任务 | 6px 绿点 | 无 |
| 都没有 | 空槽 | 无 |

身份映射：

| 页签 agent | 头像 |
|------------|------|
| `CLIAgent::Claude` 等已有 logo 的 CLI | 对应品牌图标 |
| `CLIAgent::Grok` | Grok logo（当前 `icon()` 为 `None`，必须先补） |
| `CLIAgent::Unknown` | 通用 Terminal 图标 |
| 原生 Oz | `Icon::Oz` |
| ambient Oz | `Icon::OzCloud` |

## 聚合

每个 workspace 独立聚合，范围是该 workspace 的全部 tabs：当前窗口 `self.tabs` 中归属该 id 的页签，加上 `RepositoryWorkspaceTabSets` 里该 id 的 inactive tabs。

计入规则：

- 只统计能解析出 agent 身份、且会话状态为 InProgress 或 Blocked 的页签。
- 只读 terminal、shared session viewer 不计，与现有 long-running 过滤一致。
- 原生 Oz 空对话或 entirely passive 对话不计，与 `TabComponent::agent_indicator` 一致。
- 多个命中时取 **最近进入 InProgress / Blocked** 的那一个。实现用会话进入该状态的时间戳；没有现成时间戳时，用页签从左到右扫描中最后一个仍处于该状态的页签，并在 TECH / 实现计划里写死这一回退，避免按「当前聚焦」误实现。
- 当前显示的那个结束后，立刻改显示下一个仍命中的；都没有则回到绿点或空槽。

绿点判定继续走现有 `repository_workspace_ids_with_long_running_terminal`。渲染时若该 workspace 已有 agent 头像，则不画绿点。

## 数据流

1. Terminal / Agent 会话状态变化，沿用现有 `TerminalViewStateChanged` 与 conversation status 更新，触发 `Workspace` 刷新。
2. Workspace 在刷新 project tree 时，除现有 `running_workspace_ids` 外，再计算 `workspace_id → WorkspaceAgentActivity`。
3. `ProjectTreePanel` 按 workspace 读取该活动：有 agent 则画头像槽，否则回退绿点 / 空槽。
4. InProgress 行持有跨 render 的呼吸动画 handle（与 `SpinnerStateHandle` 同模式）；Blocked 与空闲不创建动画 handle。

`WorkspaceAgentActivity` 只表达展示所需字段：

- agent 身份（CLI 变体或 Oz / ambient）
- `InProgress` 或 `Blocked`
- 用于多 agent 决胜的次序（时间戳或稳定扫描回退）

不把 running agent 数量暴露给 tab count。

## 组件边界

- 聚合逻辑留在 `Workspace`（与 `repository_workspace_ids_with_long_running_terminal` 并列），不进入 `ProjectTreePanel` 渲染 helper。
- `ProjectTreePanel` 只消费已经算好的 `HashMap<RepositoryWorkspaceId, WorkspaceAgentActivity>`。
- 头像与呼吸环是纯 UI helper，不遍历 tabs，不读 `TerminalView`。
- 不在 terminal 输出路径里更新 workspace 状态。
- `CLIAgent::Grok.icon()` 必须补上 logo，否则本功能的主示例路径无法绘制。

## 边界情况

- 选中且 InProgress：选中强调色与呼吸环同时存在。
- 未选中且 InProgress / Blocked：只显示头像槽，没有选中光影。
- 切到别的 workspace：原 workspace 的后台 agent 继续显示在原行。
- 同一 workspace 里 Grok 在跑、Claude 随后也开始跑：改显示 Claude（更晚进入状态的那个）；Claude 结束后回到 Grok。
- Grok 在跑同时有 `npm install` 长任务：只显示 Grok 头像，不叠绿点。
- Grok 结束后 `npm install` 仍在跑：头像消失，绿点出现。
- Blocked 之后用户批准、会话回到 InProgress：黄环改回品牌色呼吸环，头像不变。
- 会话 Success / Error / Cancelled：头像立即消失。
- 未知 CLI：显示 Terminal 图标，仍可有呼吸 / 黄环。
- 行宽不足：截断 workspace 名称，不截断头像槽和 tab count。

## 测试

单元：

- 无 agent、无长任务 → 无头像、无绿点。
- 无 agent、有长任务 → 绿点、无头像。
- 单个 InProgress CLI agent → 对应头像 + InProgress。
- 单个 Blocked Oz agent → Oz 头像 + Blocked。
- InProgress 与长任务同时存在 → 只输出 agent 活动，不输出「当作绿点」的互斥结果。
- 两个 agent 同时命中 → 输出更晚进入状态的那个；缺少时间戳时输出扫描回退规则下的那一个。
- Success / Error / Cancelled 的页签不进入活动 map。
- inactive workspace 的 running agent 仍计入对应 workspace id。

视图：

- InProgress 行渲染品牌头像，不渲染绿点。
- Blocked 行渲染头像且不启用呼吸动画。
- 空闲有长任务的行渲染绿点。
- 空闲无长任务的行不渲染头像也不渲染绿点，槽宽仍在。
- 选中 + InProgress 同时渲染选中强调与头像槽。

验证：相关 Rust 单测 + `cargo check`。

## 风险

- `CLIAgent::Grok` 目前没有 icon，不补 logo 则示例路径空白。
- WarpUI 没有现成呼吸环 primitive，需要按 `BrailleSpinner` 的 `repaint_after` 模式做一层很薄的环，避免在 `render_workspace_row` 里堆一次性动画。
- 多 agent 若误用「当前聚焦页签」决胜，会让切走后的后台 workspace 显示错误身份。
- 呼吸环过亮会把侧栏变成常亮灯带；必须用低对比品牌色和 1.6s 慢周期。
- 身份槽从 6px 升到 16px 会挤名称；必须用同一约束宽度并继续 ellipsis 名称，不能挤掉 tab count。

## 验收

- workspace 中有 Grok CLI 在跑时，对应行显示 Grok 头像和呼吸环。
- 该会话进入 Blocked 后，头像留下，环变静态黄。
- 会话结束后头像消失；若仍有 shell 长任务则出现绿点。
- 切到其他 workspace 后，原行仍然显示正在跑或阻塞中的 agent。
- 原生 Oz 会话显示 Oz 图标，不显示当前模型品牌。
- 点击行仍然只切换 workspace。
- tab count 数字不变。

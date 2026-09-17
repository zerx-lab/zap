# Repository Workspace Add Button Design

## 背景

Repository workspace 左侧树中，repository 行末的“+”图标会显示手形光标，但点击后不会打开创建 workspace 对话框，且 `Create workspace` tooltip 不会稳定显示。顶部 `Add repository` 按钮正常工作。

`ProjectTreePanel` 为 repository 行、workspace 行及其行内按钮分别维护 `HashMap<Id, MouseStateHandle>`。当前渲染代码只用 `get(...).unwrap_or_default()` 读取这些 map，从未插入新句柄。因此每一次重绘都会为交互元素创建新的状态句柄。鼠标按下触发重绘后，鼠标释放面对的是新句柄，无法组成一次完整点击；tooltip 的 hover 状态也会在重绘后丢失。

## 目标

- repository 行末的“+”在一次正常点击后打开创建 workspace 对话框。
- “+”的 `Create workspace` tooltip 使用稳定的 hover 状态。
- repository 行、workspace 行及 workspace 删除按钮使用同一正确的状态缓存机制，避免保留相同缺陷。
- 删除已不存在 repository/workspace 的状态句柄，避免缓存无限增长。

## 非目标

- 不修改创建 workspace 的 Git、持久化或对话框表单逻辑。
- 不修改 repository workspace 的视觉设计、Feature Flag 或顶部 `Add repository` 流程。
- 不引入新的子 View 或内部可变 render 状态。

## 方案

在 `ProjectTreePanel::refresh_tree` 完成树状态重建后，根据当前 repository/workspace ID 同步四组 `MouseStateHandle` 缓存：

1. 清理不再出现在树中的 repository/workspace ID。
2. 为当前的 repository 行、repository 行内创建按钮、workspace 行和 workspace 删除按钮通过 `entry(id).or_default()` 初始化稳定句柄。
3. 渲染函数只从已同步缓存读取句柄；若同步逻辑被破坏则快速失败，而不在 render 路径临时创建状态。

缓存写入放在状态刷新阶段，而非 render 阶段。这样 render 保持只读，且每次重绘都复用同一 `Arc<Mutex<MouseState>>`，符合其他动态列表的既有模式。

## 测试策略

新增或调整 `project_tree_tests.rs` 的 UI 事件测试：

1. 渲染带有 repository 行的 `ProjectTreePanel`。
2. 在行末“+”的位置发送 `LeftMouseDown`。
3. 强制重新构建 scene，模拟真实 UI 在按下后发生的重绘。
4. 在相同位置发送 `LeftMouseUp`。
5. 断言仅收到对应 repository 的 `CreateWorkspaceRequested` 事件，并确认 repository 的展开状态未变化。

该测试在修复前必须失败，因为重绘会替换按下状态的句柄；修复后通过。

## 风险与验证

修改仅限 `ProjectTreePanel` 的交互状态生命周期，不改变 action、event 或 modal 链路。验证包括该回归测试和 `cargo check -p warp`；环境未安装 `cargo-nextest` 时，针对性测试使用 `cargo test -p warp --lib` 运行。

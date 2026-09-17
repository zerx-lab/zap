# ProjectTreePanel 工作区列表滚动设计

## 背景

`ProjectTreePanel` 用两级树展示 repository 和 workspace。当前树内容直接放在可收缩区域中，没有绑定 WarpUI 的滚动容器。当展开的 workspace 数量超过窗口可用高度时，超出部分会被裁剪，鼠标滚轮也不会改变列表位置。

## 目标

- workspace 数量超过可视区域时，树列表支持垂直滚轮滚动。
- 顶部 `Repositories` 标题和底部未分类 workspace 区域保持固定。
- 保留现有 repository 展开/折叠、workspace 选择、悬停和操作按钮行为。
- 不引入虚拟列表或改变项目组织数据模型。

## 方案

在 `ProjectTreePanel` 中增加一个 `ClippedScrollStateHandle`，并使用现有的 `ClippedScrollable::vertical` 包裹中间树内容。最终布局保持三段结构：

1. 顶部固定标题和 `Add repository` 按钮。
2. 中间由 `Shrinkable` 约束高度的垂直滚动树列表。
3. 底部固定未分类 workspace 行。

滚动条使用 WarpUI 现有默认宽度和主题颜色，不新增局部主题或颜色常量。滚动状态由视图持有，以便重绘之间保留当前位置。

## 测试

增加一个视图回归测试，构造超过窗口高度的多个 workspace，完成场景布局后在树列表区域派发垂直 `ScrollWheel` 事件，并断言 `ClippedScrollStateHandle` 的滚动起点大于零。现有树状态、渲染和交互测试继续运行。

## 验收标准

- 大量 workspace 时可以通过鼠标滚轮上下移动列表。
- 标题和底部固定区域不会随列表滚动。
- workspace 行仍可点击，repository 仍可展开/折叠。
- 相关 Rust 测试通过，且 `cargo check` 通过。

## 非目标

- 不修改 workspace 排序、持久化或项目组织模型。
- 不重新设计滚动条视觉样式。
- 不将树改造成虚拟化列表。

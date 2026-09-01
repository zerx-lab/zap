# Workspace 选中态设计

## 目标

让项目树中的当前 workspace 在不依赖文字颜色的情况下快速可识别，并保持现有主题、悬停删除按钮和行布局行为不变。

## 选中态

选中的 workspace 行同时使用三种视觉信号：

- 使用主题的 `surface_2` 作为整行背景。
- 使用主题的 `accent` 绘制 1px 边框。
- 使用主题的 `accent` 在行左侧绘制 3px 强调条。

workspace 名称继续使用 `accent` 文字颜色，分支名称和 tab 数量继续使用次级文字颜色。未选中行不增加新的背景、边框或强调条。

## 交互与布局

- 选中态只由 `ProjectTreeState::selected_workspace_id` 决定。
- 鼠标悬停时仍显示 workspace 删除按钮，未悬停时保留原有尺寸占位。
- 选中态不会改变行的宽高、文字换行或点击区域。
- repository 行和未分类 workspace 行不受本次视觉调整影响。

## 验证

- 增加或更新项目树选中态测试，确认选中 workspace 的视觉状态判断正确。
- 运行项目树相关单元测试。
- 运行 `cargo check -p warp` 和 `git diff --check`。

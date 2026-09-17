# Repository Workspace 页签 Chrome Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 当 Repository Workspaces 开启且左侧栏打开时，侧栏通顶、TabBar 只出现在内容列，让页签在视觉上属于当前 workspace。

**Architecture:** 抽出可单测的 chrome 纯函数，再改 `Workspace::render`：把 `ToolsPanel` 从 TabBar 下方的内容行上提到与 `column(TabBar, 内容)` 并列。LeftPanel 工具条头承担 macOS 红绿灯避让；TabBar 不再为左侧红绿灯留空。不抽独立 Chrome 组件，不改页签归属或 Git。

**Tech Stack:** Rust, WarpUI Flex/SavePosition/Presenter, `FeatureFlag::RepositoryWorkspaces`, cargo nextest, cargo check.

**Spec:** `docs/superpowers/specs/2026-08-25-repository-workspace-tab-chrome-design.md`

## Global Constraints

- 注释用简体中文；日志和断言消息用英文。
- `format!`/`println!` 使用内联参数 `"{x}"`。
- `match` 禁止无必要的 `_` 通配。
- 未使用参数直接删除，不加 `_` 前缀。
- 不改页签归属、`RepositoryWorkspaceTabSets`、持久化或 Git。
- 不抽 `WorkspaceChrome` 布局组件，不用 overlay 把侧栏叠进标题栏。
- 右侧栏不通顶。LeftPanel 仍是互斥工具页。
- 验证以相关单测和 `cargo check` 为准；不引入截图测试。
- 提交信息遵循仓库现有风格，只包含本任务文件。

## File Structure

- Create: `app/src/workspace/view/full_height_left_panel_chrome.rs` — chrome 判断、ToolsPanel 过滤、TabBar / 侧栏红绿灯 padding。
- Create: `app/src/workspace/view/full_height_left_panel_chrome_tests.rs` — 上述纯函数真值表。
- Modify: `app/src/workspace/view.rs` — 声明模块、`render` 上提侧栏、inner items 去重、padding 委托、同步侧栏 inset。
- Modify: `app/src/workspace/view/left_panel.rs` — `titlebar_leading_inset` 与工具条头左侧 padding。
- Modify: `app/src/workspace/view_test.rs` — 布局回归：TabBar 在侧栏右侧且顶对齐。
- Modify: `specs/repository-workspaces/PRODUCT.md` — 行为 1。
- Modify: `specs/repository-workspaces/TECH.md` — 窗口装配说明。

---

### Task 1: Chrome 纯函数

**Files:**
- Create: `app/src/workspace/view/full_height_left_panel_chrome.rs`
- Create: `app/src/workspace/view/full_height_left_panel_chrome_tests.rs`
- Modify: `app/src/workspace/view.rs` (只加 `mod` 声明，约第 7 行 `left_panel` 旁)

**Interfaces:**
- Consumes: `crate::workspace::header_toolbar_item::HeaderToolbarItemKind`，`super::TAB_BAR_PADDING_LEFT`（值为 `4.`）
- Produces:
  - `pub(crate) fn use_full_height_left_panel_chrome(repository_workspaces_enabled: bool, left_panel_open: bool, simplified_wasm_tab_bar: bool, vertical_tabs_active: bool, mobile_overlay: bool) -> bool`
  - `pub(crate) fn header_items_excluding_lifted_tools_panel(items: impl IntoIterator<Item = HeaderToolbarItemKind>, full_height_chrome: bool) -> Vec<HeaderToolbarItemKind>`
  - `pub(crate) fn tab_bar_leading_padding(full_height_chrome: bool, theme_chooser_open: bool, is_macos_fullscreen: bool, left_traffic_light_width: f32) -> f32`
  - `pub(crate) fn left_panel_titlebar_leading_inset(full_height_chrome: bool, is_macos_fullscreen: bool, left_traffic_light_width: f32) -> f32`

- [ ] **Step 1: Write the failing tests**

在 `app/src/workspace/view.rs` 顶部模块列表中、`pub(crate) mod left_panel;` 旁加入：

```rust
pub(crate) mod full_height_left_panel_chrome;
```

创建 `full_height_left_panel_chrome.rs`，暂时只放空模块和测试入口（函数尚未定义，测试应编译失败或链接失败）：

```rust
use crate::workspace::header_toolbar_item::HeaderToolbarItemKind;

#[cfg(test)]
#[path = "full_height_left_panel_chrome_tests.rs"]
mod tests;
```

创建 `full_height_left_panel_chrome_tests.rs`：

```rust
use super::{
    header_items_excluding_lifted_tools_panel, left_panel_titlebar_leading_inset,
    tab_bar_leading_padding, use_full_height_left_panel_chrome,
};
use crate::workspace::header_toolbar_item::HeaderToolbarItemKind;

#[test]
fn use_full_height_left_panel_chrome_truth_table() {
    assert!(use_full_height_left_panel_chrome(
        true, true, false, false, false
    ));
    assert!(!use_full_height_left_panel_chrome(
        false, true, false, false, false
    ));
    assert!(!use_full_height_left_panel_chrome(
        true, false, false, false, false
    ));
    assert!(!use_full_height_left_panel_chrome(
        true, true, true, false, false
    ));
    assert!(!use_full_height_left_panel_chrome(
        true, true, false, true, false
    ));
    assert!(!use_full_height_left_panel_chrome(
        true, true, false, false, true
    ));
}

#[test]
fn header_items_excluding_lifted_tools_panel_drops_tools_panel_only_when_chrome_is_on() {
    let items = vec![
        HeaderToolbarItemKind::TabsPanel,
        HeaderToolbarItemKind::ToolsPanel,
        HeaderToolbarItemKind::CodeReview,
    ];

    assert_eq!(
        header_items_excluding_lifted_tools_panel(items.clone(), true),
        vec![
            HeaderToolbarItemKind::TabsPanel,
            HeaderToolbarItemKind::CodeReview,
        ]
    );
    assert_eq!(
        header_items_excluding_lifted_tools_panel(items, false),
        vec![
            HeaderToolbarItemKind::TabsPanel,
            HeaderToolbarItemKind::ToolsPanel,
            HeaderToolbarItemKind::CodeReview,
        ]
    );
}

#[test]
fn tab_bar_leading_padding_omits_traffic_lights_when_chrome_is_on() {
    let traffic_light_width = 64.;
    assert_eq!(
        tab_bar_leading_padding(true, false, false, traffic_light_width),
        super::super::TAB_BAR_PADDING_LEFT
    );
    assert_eq!(
        tab_bar_leading_padding(false, false, false, traffic_light_width),
        traffic_light_width + 16.
    );
    assert_eq!(
        tab_bar_leading_padding(false, true, false, traffic_light_width),
        0.
    );
    assert_eq!(
        tab_bar_leading_padding(false, false, true, traffic_light_width),
        super::super::TAB_BAR_PADDING_LEFT
    );
}

#[test]
fn left_panel_titlebar_leading_inset_takes_macos_traffic_lights_when_chrome_is_on() {
    assert_eq!(
        left_panel_titlebar_leading_inset(true, false, 64.),
        64.
    );
    assert_eq!(
        left_panel_titlebar_leading_inset(true, true, 64.),
        0.
    );
    assert_eq!(
        left_panel_titlebar_leading_inset(false, false, 64.),
        0.
    );
    assert_eq!(
        left_panel_titlebar_leading_inset(true, false, 0.),
        0.
    );
}
```

把 `view.rs` 约 521 行的 `const TAB_BAR_PADDING_LEFT: f32 = 4.;` 改成 `pub(crate) const TAB_BAR_PADDING_LEFT: f32 = 4.;`，供 chrome 模块和测试使用。

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo nextest run -p warp -E 'test(use_full_height_left_panel_chrome_truth_table) or test(header_items_excluding_lifted_tools_panel_drops_tools_panel_only_when_chrome_is_on) or test(tab_bar_leading_padding_omits_traffic_lights_when_chrome_is_on) or test(left_panel_titlebar_leading_inset_takes_macos_traffic_lights_when_chrome_is_on)'
```

Expected: FAIL，未找到函数或模块编译失败。

- [ ] **Step 3: Write minimal implementation**

把 `full_height_left_panel_chrome.rs` 写成：

```rust
use crate::workspace::header_toolbar_item::HeaderToolbarItemKind;

use super::TAB_BAR_PADDING_LEFT;

/// 项目组织模式下，侧栏是否通顶、TabBar 是否只出现在内容列。
pub(crate) fn use_full_height_left_panel_chrome(
    repository_workspaces_enabled: bool,
    left_panel_open: bool,
    simplified_wasm_tab_bar: bool,
    vertical_tabs_active: bool,
    mobile_overlay: bool,
) -> bool {
    repository_workspaces_enabled
        && left_panel_open
        && !simplified_wasm_tab_bar
        && !vertical_tabs_active
        && !mobile_overlay
}

/// 新 chrome 已把 ToolsPanel 提到窗口左侧时，从 header toolbar 配置中去掉它，避免画两次。
pub(crate) fn header_items_excluding_lifted_tools_panel(
    items: impl IntoIterator<Item = HeaderToolbarItemKind>,
    full_height_chrome: bool,
) -> Vec<HeaderToolbarItemKind> {
    items
        .into_iter()
        .filter(|item| !(full_height_chrome && *item == HeaderToolbarItemKind::ToolsPanel))
        .collect()
}

/// TabBar 左侧 padding。新 chrome 下红绿灯改由侧栏头承担。
pub(crate) fn tab_bar_leading_padding(
    full_height_chrome: bool,
    theme_chooser_open: bool,
    is_macos_fullscreen: bool,
    left_traffic_light_width: f32,
) -> f32 {
    if theme_chooser_open {
        0.
    } else if full_height_chrome || is_macos_fullscreen {
        TAB_BAR_PADDING_LEFT
    } else {
        left_traffic_light_width + 16.
    }
}

/// 侧栏工具条头为 macOS 窗口左上红绿灯预留的宽度。Windows/Linux 传入 0。
pub(crate) fn left_panel_titlebar_leading_inset(
    full_height_chrome: bool,
    is_macos_fullscreen: bool,
    left_traffic_light_width: f32,
) -> f32 {
    if full_height_chrome && !is_macos_fullscreen {
        left_traffic_light_width
    } else {
        0.
    }
}

#[cfg(test)]
#[path = "full_height_left_panel_chrome_tests.rs"]
mod tests;
```

`HeaderToolbarItemKind` 未 derive `Copy`，`filter` 比较引用即可。

- [ ] **Step 4: Run tests to verify they pass**

Run: 与 Step 2 相同的 nextest 命令。

Expected: PASS，4 个测试全绿。

- [ ] **Step 5: Commit**

```bash
git add app/src/workspace/view.rs app/src/workspace/view/full_height_left_panel_chrome.rs app/src/workspace/view/full_height_left_panel_chrome_tests.rs
git commit -m "feat: add full-height left panel chrome helpers"
```

---

### Task 2: 把 helpers 接到 TabBar padding、header items 和侧栏头

**Files:**
- Modify: `app/src/workspace/view.rs`
  - `compute_tab_bar_left_padding`（约 17755）
  - `render_banner_and_active_tab` 的 `left_items`/`right_items` 循环（约 18439、18457）
  - `render_empty_workspace_content` 的 `left_items` 循环（约 18386）
  - `update_titlebar_height`（约 12638）
  - `open_left_panel` / `close_left_panel`（约 8594、8651）
- Modify: `app/src/workspace/view/left_panel.rs`
  - `LeftPanelView` 字段（约 217）
  - `LeftPanelView::new`（约 253）
  - `render` 工具条 `padding_left`（约 1477）

**Interfaces:**
- Consumes: Task 1 的四个 `pub(crate)` 函数
- Produces:
  - `Workspace::use_full_height_left_panel_chrome(&self, app: &AppContext) -> bool`
  - `Workspace::sync_left_panel_titlebar_inset(&self, ctx: &mut ViewContext<Self>)`
  - `Workspace::left_traffic_light_width(&self, ctx: &AppContext) -> f32`
  - `LeftPanelView::set_titlebar_leading_inset(&mut self, inset: f32, ctx: &mut ViewContext<Self>)`
  - `LeftPanelView::titlebar_leading_inset(&self) -> f32`（供测试）

- [ ] **Step 1: Write the failing tests**

在 `full_height_left_panel_chrome_tests.rs` 已覆盖 padding 公式。本任务补一个 Workspace 委托测试，写在 `app/src/workspace/view_test.rs` 末尾：

```rust
#[test]
fn compute_tab_bar_left_padding_omits_macos_traffic_lights_when_repository_chrome_is_on() {
    let _flag = FeatureFlag::RepositoryWorkspaces.override_enabled(true);

    App::test((), |mut app| async move {
        initialize_app(&mut app);
        let workspace = mock_workspace(&mut app);

        workspace.update(&mut app, |workspace, ctx| {
            workspace.open_left_panel(ctx);
            assert!(
                workspace.use_full_height_left_panel_chrome(ctx),
                "repository workspaces with an open left panel should use full-height chrome"
            );
            assert_eq!(
                workspace.compute_tab_bar_left_padding(ctx),
                super::TAB_BAR_PADDING_LEFT
            );
        });
    });
}
```

`compute_tab_bar_left_padding` 目前是私有方法。测试模块是 `view.rs` 里的 `#[path = "view_test.rs"] mod tests;`，可以访问私有方法，无需改可见性。

若 `open_left_panel` 对测试模块是私有的：`view_test.rs` 已在现有测试里调用 `workspace.open_left_panel(ctx)`（约 1801 行），可见。

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo nextest run -p warp -E 'test(compute_tab_bar_left_padding_omits_macos_traffic_lights_when_repository_chrome_is_on)'
```

Expected: FAIL，因为 `Workspace::use_full_height_left_panel_chrome` 还不存在，或 `compute_tab_bar_left_padding` 仍走旧公式。

- [ ] **Step 3: Write minimal implementation**

在 `view.rs` 的 `impl Workspace` 中、`vertical_tabs_active` 附近加入：

```rust
fn use_full_height_left_panel_chrome(&self, app: &AppContext) -> bool {
    cfg_if::cfg_if! {
        if #[cfg(target_family = "wasm")] {
            let simplified_wasm_tab_bar =
                self.get_simplified_wasm_tab_bar_content(app).is_some();
        } else {
            let simplified_wasm_tab_bar = false;
        }
    }

    full_height_left_panel_chrome::use_full_height_left_panel_chrome(
        FeatureFlag::RepositoryWorkspaces.is_enabled(),
        self.is_left_panel_open(app),
        simplified_wasm_tab_bar,
        Self::vertical_tabs_active(app),
        warpui::platform::is_mobile_device(),
    )
}

fn left_traffic_light_width(&self, ctx: &AppContext) -> f32 {
    let zoom_factor = WindowSettings::as_ref(ctx).zoom_level.as_zoom_factor();
    traffic_light_data(ctx, self.window_id)
        .as_ref()
        .filter(|data| data.side == TrafficLightSide::Left)
        .map(|data| data.width(zoom_factor))
        .unwrap_or(0.)
}

fn is_macos_fullscreen(&self, ctx: &AppContext) -> bool {
    let is_window_fullscreen = ctx
        .windows()
        .platform_window(self.window_id)
        .map(|window| window.fullscreen_state() == FullscreenState::Fullscreen)
        .unwrap_or(false);
    is_window_fullscreen && cfg!(target_os = "macos")
}

fn sync_left_panel_titlebar_inset(&self, ctx: &mut ViewContext<Self>) {
    let inset = full_height_left_panel_chrome::left_panel_titlebar_leading_inset(
        self.use_full_height_left_panel_chrome(ctx),
        self.is_macos_fullscreen(ctx),
        self.left_traffic_light_width(ctx),
    );
    self.left_panel_view.update(ctx, |left_panel, ctx| {
        left_panel.set_titlebar_leading_inset(inset, ctx);
    });
}
```

替换 `compute_tab_bar_left_padding` 体为：

```rust
fn compute_tab_bar_left_padding(&self, ctx: &AppContext) -> f32 {
    full_height_left_panel_chrome::tab_bar_leading_padding(
        self.use_full_height_left_panel_chrome(ctx),
        self.current_workspace_state.is_left_panel_open(),
        self.is_macos_fullscreen(ctx),
        self.left_traffic_light_width(ctx),
    )
}
```

`render_banner_and_active_tab` 中把：

```rust
for item in config.left_items() {
```

改为：

```rust
let full_height_chrome = self.use_full_height_left_panel_chrome(app);
for item in full_height_left_panel_chrome::header_items_excluding_lifted_tools_panel(
    config.left_items(),
    full_height_chrome,
) {
```

对 `config.right_items()` 和 `render_empty_workspace_content` 里的 `config.left_items()` 做同样过滤。不要漏 right_items：ToolsPanel 被用户配到右侧时也必须去掉。

`update_titlebar_height` 末尾、`open_left_panel` 的 `ctx.notify()` 之前、`close_left_panel` 的 `ctx.notify()` 之前各加：

```rust
self.sync_left_panel_titlebar_inset(ctx);
```

`LeftPanelView` 增加字段 `titlebar_leading_inset: f32`，`new` 里初始化为 `0.`。增加：

```rust
pub fn set_titlebar_leading_inset(&mut self, inset: f32, ctx: &mut ViewContext<Self>) {
    if (self.titlebar_leading_inset - inset).abs() > f32::EPSILON {
        self.titlebar_leading_inset = inset;
        ctx.notify();
    }
}

pub fn titlebar_leading_inset(&self) -> f32 {
    self.titlebar_leading_inset
}
```

`render` 里工具条头把 `.with_padding_left(10.)` 改成 `.with_padding_left(10. + self.titlebar_leading_inset)`。

- [ ] **Step 4: Run tests to verify they pass**

Run:

```bash
cargo nextest run -p warp -E 'test(compute_tab_bar_left_padding_omits_macos_traffic_lights_when_repository_chrome_is_on) or test(use_full_height_left_panel_chrome_truth_table) or test(header_items_excluding_lifted_tools_panel_drops_tools_panel_only_when_chrome_is_on) or test(tab_bar_leading_padding_omits_traffic_lights_when_chrome_is_on) or test(left_panel_titlebar_leading_inset_takes_macos_traffic_lights_when_chrome_is_on)'
```

Expected: PASS。

- [ ] **Step 5: Commit**

```bash
git add app/src/workspace/view.rs app/src/workspace/view/left_panel.rs app/src/workspace/view_test.rs
git commit -m "feat: apply full-height chrome padding and skip inline ToolsPanel"
```

---

### Task 3: 上提 ToolsPanel 到与 TabBar 并列

**Files:**
- Modify: `app/src/workspace/view.rs`
  - `TAB_BAR_POSITION_ID` 旁新增 `LEFT_PANEL_POSITION_ID`
  - `fn render`（约 21769）

**Interfaces:**
- Consumes: `Workspace::use_full_height_left_panel_chrome`，`ChildView` of `left_panel_view`
- Produces: `pub(crate) const LEFT_PANEL_POSITION_ID: &str = "workspace_view:left_panel";` 新 chrome 下外层为 `row(SavePosition(left_panel), column(TabBar, panels))`

- [ ] **Step 1: Write the failing test**

在 `view_test.rs` 增加布局测试。需要 `Presenter`、`WindowInvalidation`、`vec2f`：

```rust
use pathfinder_geometry::vector::vec2f;
use warpui::{Presenter, WindowInvalidation};
use std::cell::RefCell;
use std::rc::Rc;
```

若文件顶部已有部分 import，只补缺失的。测试：

```rust
#[test]
fn full_height_left_panel_places_tab_bar_in_the_content_column() {
    let _flag = FeatureFlag::RepositoryWorkspaces.override_enabled(true);

    App::test((), |mut app| async move {
        initialize_app(&mut app);
        let workspace = mock_workspace(&mut app);
        let window_id = workspace.read(&app, |workspace, _| workspace.window_id);
        let root_view_id = app
            .root_view_id(window_id)
            .expect("window should have a root view");

        workspace.update(&mut app, |workspace, ctx| {
            workspace.open_left_panel(ctx);
        });

        let presenter = Rc::new(RefCell::new(Presenter::new(window_id)));
        app.update({
            let presenter = presenter.clone();
            let workspace = workspace.clone();
            move |ctx| {
                presenter.borrow_mut().invalidate(
                    WindowInvalidation {
                        updated: [root_view_id, workspace.id()].into_iter().collect(),
                        ..Default::default()
                    },
                    ctx,
                );
                presenter
                    .borrow_mut()
                    .build_scene(vec2f(1200., 800.), 1., None, ctx);

                let tab_bar = presenter
                    .borrow()
                    .position_cache()
                    .get_position(TAB_BAR_POSITION_ID)
                    .expect("tab bar should have a saved position");
                let left_panel = presenter
                    .borrow()
                    .position_cache()
                    .get_position(LEFT_PANEL_POSITION_ID)
                    .expect("left panel should have a saved position");

                assert!(
                    tab_bar.min_x() >= left_panel.max_x() - 2.,
                    "tab bar should sit to the right of the full-height left panel, tab_bar={tab_bar:?} left_panel={left_panel:?}"
                );
                assert!(
                    (tab_bar.min_y() - left_panel.min_y()).abs() < 2.,
                    "tab bar and left panel should share the window top, tab_bar_y={} left_panel_y={}",
                    tab_bar.min_y(),
                    left_panel.min_y()
                );
            }
        });
    });
}
```

`LEFT_PANEL_POSITION_ID` 此时还不存在，测试应编译失败。先在 `view.rs` 的 `TAB_BAR_POSITION_ID` 旁加上常量（测试才能编译），但 `render` 先不要包 `SavePosition`，这样测试会在 `expect("left panel should have a saved position")` 处失败。

`Workspace.window_id` 是私有字段 `WindowId`（`warpui::WindowId`）。`view_test.rs` 是 `view.rs` 的 `mod tests`，可以直接读。

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo nextest run -p warp -E 'test(full_height_left_panel_places_tab_bar_in_the_content_column)'
```

Expected: FAIL，`left panel should have a saved position`（常量已加但 render 未包 SavePosition），或 TabBar 的 `min_x` 仍接近 0。

- [ ] **Step 3: Write minimal implementation**

在 `TAB_BAR_POSITION_ID` 旁：

```rust
pub(crate) const LEFT_PANEL_POSITION_ID: &str = "workspace_view:left_panel";
```

改 `fn render` 里 `use_simplified_wasm_tab_bar == false` 的分支。保留简化 WASM 分支不动。

把：

```rust
} else {
    let mut outer_column = Flex::column();
    if tab_bar_mode == ShowTabBar::Stacked {
        outer_column.add_child(self.render_tab_bar(self.tab_fixed_width, appearance, app));
    }
    let content = if self.tabs.is_empty() {
        self.render_empty_workspace_content(app, appearance)
    } else {
        self.render_banner_and_active_tab(app, appearance)
    };
    let panels_row = self.render_panels(app, Shrinkable::new(1.0, content).finish(), false);
    outer_column.add_child(Shrinkable::new(1.0, panels_row).finish());
    Container::new(outer_column.finish())
        .with_background(util::get_terminal_background_fill(self.window_id, app))
        .finish()
};
```

换成：

```rust
} else {
    let content = if self.tabs.is_empty() {
        self.render_empty_workspace_content(app, appearance)
    } else {
        self.render_banner_and_active_tab(app, appearance)
    };
    let panels_row = self.render_panels(app, Shrinkable::new(1.0, content).finish(), false);
    let tab_bar = if tab_bar_mode == ShowTabBar::Stacked {
        Some(self.render_tab_bar(self.tab_fixed_width, appearance, app))
    } else {
        None
    };
    let background = util::get_terminal_background_fill(self.window_id, app);

    if self.use_full_height_left_panel_chrome(app) {
        let mut content_column = Flex::column().with_main_axis_size(MainAxisSize::Max);
        if let Some(tab_bar) = tab_bar {
            content_column.add_child(tab_bar);
        }
        content_column.add_child(Shrinkable::new(1.0, panels_row).finish());

        let left_panel = SavePosition::new(
            ChildView::new(&self.left_panel_view).finish(),
            LEFT_PANEL_POSITION_ID,
        )
        .finish();

        Container::new(
            Flex::row()
                .with_main_axis_size(MainAxisSize::Max)
                .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                .with_child(left_panel)
                .with_child(Shrinkable::new(1.0, content_column.finish()).finish())
                .finish(),
        )
        .with_background(background)
        .finish()
    } else {
        let mut outer_column = Flex::column();
        if let Some(tab_bar) = tab_bar {
            outer_column.add_child(tab_bar);
        }
        outer_column.add_child(Shrinkable::new(1.0, panels_row).finish());
        Container::new(outer_column.finish())
            .with_background(background)
            .finish()
    }
};
```

确认 `SavePosition`、`ChildView`、`CrossAxisAlignment` 已在 `view.rs` 的 `use` 中。若 `CrossAxisAlignment` 未导入，从 `warpui::elements` 补上。

Task 2 已从 inner items 去掉 ToolsPanel，这里不会画两次。空 workspace 走 `render_empty_workspace_content`，同一套外层 row，无需再改空态函数结构。

`on_tab_drag` 已用 `tab_bar_rects_for_window` 的 SavePosition；Y 回退只用 `TAB_BAR_HEIGHT`，不假定 `x = 0`。本任务不改拖拽。

- [ ] **Step 4: Run tests to verify they pass**

Run:

```bash
cargo nextest run -p warp -E 'test(full_height_left_panel_places_tab_bar_in_the_content_column) or test(compute_tab_bar_left_padding_omits_macos_traffic_lights_when_repository_chrome_is_on) or test(repository_workspace_mode_keeps_setting_but_uses_horizontal_tabbar)'
```

Expected: PASS。若 SavePosition 要等 LeftPanel 真正 layout，确认 `open_left_panel` 后 `is_left_panel_open` 为 true，且 Resizable 有默认宽度（`new` 失败时 fallback `600.0`）。场景宽度用 `1200.`，避免挤没。

- [ ] **Step 5: Commit**

```bash
git add app/src/workspace/view.rs app/src/workspace/view_test.rs
git commit -m "feat: lift ToolsPanel beside the content-column tab bar"
```

---

### Task 4: 空 workspace 与关侧栏回归

**Files:**
- Modify: `app/src/workspace/view_test.rs`

**Interfaces:**
- Consumes: `LEFT_PANEL_POSITION_ID`、`TAB_BAR_POSITION_ID`、`Workspace::open_left_panel` / `close_left_panel`
- Produces: 空页签与侧栏收起两条回归测试

- [ ] **Step 1: Write the failing tests**

在 `view_test.rs` 追加。抽取 Task 3 测试里的 `build_scene` 逻辑为测试模块内私有函数，避免复制：

```rust
fn layout_workspace_scene(
    app: &mut App,
    workspace: &ViewHandle<Workspace>,
    window_id: WindowId,
) -> (pathfinder_geometry::rect::RectF, Option<pathfinder_geometry::rect::RectF>) {
    let root_view_id = app
        .root_view_id(window_id)
        .expect("window should have a root view");
    let presenter = Rc::new(RefCell::new(Presenter::new(window_id)));
    app.update({
        let presenter = presenter.clone();
        let workspace = workspace.clone();
        move |ctx| {
            presenter.borrow_mut().invalidate(
                WindowInvalidation {
                    updated: [root_view_id, workspace.id()].into_iter().collect(),
                    ..Default::default()
                },
                ctx,
            );
            presenter
                .borrow_mut()
                .build_scene(vec2f(1200., 800.), 1., None, ctx);
        }
    });
    let cache = presenter.borrow();
    let cache = cache.position_cache();
    let tab_bar = cache
        .get_position(TAB_BAR_POSITION_ID)
        .expect("tab bar should have a saved position");
    let left_panel = cache.get_position(LEFT_PANEL_POSITION_ID);
    (tab_bar, left_panel)
}
```

`WindowId` 已通过 `view.rs` 的 `use warpui::WindowId` 进入测试模块。把 Task 3 的布局测试改为调用这个 helper。

新测试：

```rust
#[test]
fn full_height_chrome_keeps_content_column_tab_bar_when_workspace_has_no_tabs() {
    let _flag = FeatureFlag::RepositoryWorkspaces.override_enabled(true);

    App::test((), |mut app| async move {
        initialize_app(&mut app);
        let workspace = mock_workspace(&mut app);
        let window_id = workspace.read(&app, |workspace, _| workspace.window_id);

        workspace.update(&mut app, |workspace, ctx| {
            workspace.open_left_panel(ctx);
            // close_tab 在最后一页且 CloseWindow 关闭时会拒绝关闭，测试直接清空集合以走到空态渲染。
            workspace.tabs.clear();
            workspace.active_tab_index = 0;
        });

        let (tab_bar, left_panel) = layout_workspace_scene(&mut app, &workspace, window_id);
        let left_panel = left_panel.expect("left panel should stay full-height with no tabs");
        assert!(
            tab_bar.min_x() >= left_panel.max_x() - 2.,
            "empty workspace should still keep the tab bar in the content column"
        );
    });
}

#[test]
fn closing_left_panel_restores_full_width_tab_bar() {
    let _flag = FeatureFlag::RepositoryWorkspaces.override_enabled(true);

    App::test((), |mut app| async move {
        initialize_app(&mut app);
        let workspace = mock_workspace(&mut app);
        let window_id = workspace.read(&app, |workspace, _| workspace.window_id);

        workspace.update(&mut app, |workspace, ctx| {
            workspace.open_left_panel(ctx);
        });
        let (open_tab_bar, open_left) = layout_workspace_scene(&mut app, &workspace, window_id);
        assert!(open_left.is_some(), "open left panel should publish LEFT_PANEL_POSITION_ID");
        assert!(open_tab_bar.min_x() > 40., "open chrome should push the tab bar rightward");

        workspace.update(&mut app, |workspace, ctx| {
            workspace.close_left_panel(ctx);
        });
        let (closed_tab_bar, closed_left) = layout_workspace_scene(&mut app, &workspace, window_id);
        assert!(
            closed_left.is_none(),
            "closed left panel should not keep the lifted SavePosition"
        );
        assert!(
            closed_tab_bar.min_x() < open_tab_bar.min_x(),
            "closing the left panel should restore a full-width tab bar, open_x={} closed_x={}",
            open_tab_bar.min_x(),
            closed_tab_bar.min_x()
        );
    });
}
```

`render` 用 `self.tabs.is_empty()` 选择 `render_empty_workspace_content`。不要调用 `close_tab`：最后一页会因 `ContextFlag::CloseWindow` 被拒绝。

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo nextest run -p warp -E 'test(full_height_chrome_keeps_content_column_tab_bar_when_workspace_has_no_tabs) or test(closing_left_panel_restores_full_width_tab_bar)'
```

Expected: 若 Task 3 已正确实现，这两条可能直接 PASS。若空页签路径仍把 ToolsPanel 画在内容行里，第一条 FAIL；若关侧栏后 `LEFT_PANEL_POSITION_ID` 仍在（因为 SavePosition 包在未打开的面板上），第二条 FAIL。关侧栏时 `use_full_height_left_panel_chrome` 为 false，应走旧 `column` 布局，不再包 `LEFT_PANEL_POSITION_ID`。

- [ ] **Step 3: Fix only if tests fail**

空页签：确认 `render_empty_workspace_content` 已用 `header_items_excluding_lifted_tools_panel`。关侧栏：确认 `render` 在 chrome 为 false 时不包 `LEFT_PANEL_POSITION_ID`。不要为了测试强行 `clear` 生产状态。

- [ ] **Step 4: Run tests to verify they pass**

Run:

```bash
cargo nextest run -p warp -E 'test(full_height_left_panel) or test(compute_tab_bar_left_padding_omits_macos_traffic_lights) or test(repository_workspace_mode_keeps_setting_but_uses_horizontal_tabbar)'
```

Expected: PASS。

- [ ] **Step 5: Commit**

```bash
git add app/src/workspace/view.rs app/src/workspace/view_test.rs
git commit -m "test: cover empty workspace and collapsed sidebar chrome"
```

---

### Task 5: 规格同步与 check

**Files:**
- Modify: `specs/repository-workspaces/PRODUCT.md`
- Modify: `specs/repository-workspaces/TECH.md`

**Interfaces:**
- Consumes: 已实现的窗口装配
- Produces: 与代码一致的 PRODUCT / TECH 描述

- [ ] **Step 1: Update PRODUCT.md behavior 1**

把行为 1：

```
1. 当 `RepositoryWorkspaces` Feature Flag 启用时，主窗口左侧显示 repository → workspace 双层树；右侧继续使用顶部现有 TabBar。项目组织模式不显示 Vertical Tabs，也不修改用户原有的 Vertical Tabs 设置值。
```

换成：

```
1. 当 `RepositoryWorkspaces` Feature Flag 启用时，主窗口左侧显示 repository → workspace 双层树。左侧 ToolsPanel 打开时，侧栏通顶，TabBar 只出现在右侧内容列；侧栏收起时 TabBar 恢复整窗通栏。项目组织模式不显示 Vertical Tabs，也不修改用户原有的 Vertical Tabs 设置值。
```

- [ ] **Step 2: Update TECH.md**

在 `### 6. 左侧树与弹窗` 之前插入：

```
### 6a. 窗口 chrome 装配

`FeatureFlag::RepositoryWorkspaces` 开启且 `Workspace::is_left_panel_open` 为真、且不是简化 WASM 标题栏 / vertical tabs / mobile overlay 时，`Workspace::render` 使用：

```
row
  ├── ToolsPanel（通顶，SavePosition `LEFT_PANEL_POSITION_ID`）
  └── column
        ├── TabBar（`TAB_BAR_POSITION_ID`）
        └── 其余 panels（header items 已去掉 ToolsPanel）
```

判断与 padding 公式在 `app/src/workspace/view/full_height_left_panel_chrome.rs`。macOS 红绿灯避让从 TabBar 改到 `LeftPanelView.titlebar_leading_inset`；Windows/Linux 右侧红绿灯仍由 TabBar 右侧 padding 承担。`titlebar_height` 仍是整窗顶带 `TOTAL_TAB_BAR_HEIGHT`。

侧栏收起或 Flag 关闭时恢复 `column(TabBar, panels)`。
```

原「### 6. 左侧树与弹窗」编号保持不变，用 `6a` 避免大规模重编号。

- [ ] **Step 3: Run check and the chrome tests**

Run:

```bash
cargo check -p warp
cargo nextest run -p warp -E 'test(full_height_left_panel) or test(use_full_height_left_panel_chrome) or test(header_items_excluding_lifted_tools_panel) or test(tab_bar_leading_padding) or test(left_panel_titlebar_leading_inset) or test(compute_tab_bar_left_padding_omits_macos_traffic_lights) or test(repository_workspace_mode_keeps_setting_but_uses_horizontal_tabbar)'
git diff --check
```

Expected: `cargo check` 成功，列出的测试 PASS，`git diff --check` 无空白错误。

- [ ] **Step 4: Commit**

```bash
git add specs/repository-workspaces/PRODUCT.md specs/repository-workspaces/TECH.md
git commit -m "docs: describe full-height left panel tab chrome"
```

---

## Spec coverage

| Spec 要求 | Task |
|-----------|------|
| `use_full_height_left_panel_chrome` 真值表 | Task 1 |
| inner left/right items 去掉 ToolsPanel | Task 1 + 2 |
| TabBar 不再为 macOS 红绿灯留空 | Task 1 + 2 |
| LeftPanel 头承担 macOS 红绿灯 inset | Task 1 + 2 |
| `render` 上提 ToolsPanel，TabBar 在内容列 | Task 3 |
| 空 workspace 同样省略 inline ToolsPanel | Task 2 + 4 |
| 侧栏收起恢复通栏 | Task 4 |
| 不改拖拽坐标系（SavePosition） | Task 3 说明，不改 `on_tab_drag` |
| PRODUCT.md 行为 1 | Task 5 |
| TECH.md 窗口装配 | Task 5 |
| 现有 horizontal tabbar 测试 | Task 3/4/5 回归 |
| 不抽 Chrome 组件 / 不 overlay / 右侧栏不通顶 | 全任务遵守 |

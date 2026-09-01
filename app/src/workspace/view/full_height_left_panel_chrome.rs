use crate::project_organization::view::project_tree::{
    resolved_project_organization_tab_layout, TabLayout,
};
use crate::util::traffic_lights::{traffic_light_data, TrafficLightSide};
use crate::window_settings::WindowSettings;
use crate::workspace::header_toolbar_item::HeaderToolbarItemKind;
use crate::workspace::tab_settings::TabSettings;
use warp_core::features::FeatureFlag;
use warpui::platform::FullscreenState;
use warpui::{AppContext, SingletonEntity, WindowId};

use super::TAB_BAR_PADDING_LEFT;
use crate::project_organization::domain::RepositoryWorkspaceId;

#[derive(Clone, Debug, Default)]
pub(crate) struct WorkspaceInfoBarGitStats {
    pub workspace_id: Option<RepositoryWorkspaceId>,
    pub upstream: Option<String>,
    pub lines_added: Option<u32>,
    pub lines_removed: Option<u32>,
}

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

/// 侧栏打开且当前是真正的 repository workspace 时,顶栏中间换成 Git 信息栏。
pub(crate) fn use_workspace_info_bar(
    full_height_chrome: bool,
    has_active_repository_workspace: bool,
) -> bool {
    full_height_chrome && has_active_repository_workspace
}

/// `refs/remotes/origin/main` → `main`。没有 remote 前缀时原样返回。
pub(crate) fn short_upstream_name(upstream: &str) -> String {
    let trimmed = upstream.trim();
    trimmed
        .strip_prefix("refs/remotes/")
        .and_then(|rest| rest.split_once('/').map(|(_, branch)| branch.to_string()))
        .unwrap_or_else(|| {
            trimmed
                .strip_prefix("refs/heads/")
                .unwrap_or(trimmed)
                .to_string()
        })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WorkspaceInfoBarParts {
    pub branch: String,
    pub from_upstream: Option<String>,
    pub lines_added: u32,
    pub lines_removed: u32,
}

impl WorkspaceInfoBarParts {
    pub(crate) fn has_diff(&self) -> bool {
        self.lines_added > 0 || self.lines_removed > 0
    }
}

/// 信息栏分段。无 upstream 省略 from; +/- 都为 0 时 diff 为 0。
pub(crate) fn workspace_info_bar_parts(
    branch: &str,
    upstream: Option<&str>,
    lines_added: Option<u32>,
    lines_removed: Option<u32>,
) -> WorkspaceInfoBarParts {
    let from_upstream = upstream
        .map(short_upstream_name)
        .filter(|name| !name.is_empty());
    let added = lines_added.unwrap_or(0);
    let removed = lines_removed.unwrap_or(0);
    let (lines_added, lines_removed) = if added == 0 && removed == 0 {
        (0, 0)
    } else {
        (added, removed)
    };
    WorkspaceInfoBarParts {
        branch: branch.to_string(),
        from_upstream,
        lines_added,
        lines_removed,
    }
}

/// 信息栏纯文本回退。无 upstream 省略 from; +/- 都为 0 或未知时不显示数字。
pub(crate) fn workspace_info_bar_label(
    branch: &str,
    upstream: Option<&str>,
    lines_added: Option<u32>,
    lines_removed: Option<u32>,
) -> String {
    let parts = workspace_info_bar_parts(branch, upstream, lines_added, lines_removed);
    let mut chunks = vec![parts.branch.clone()];
    if let Some(upstream) = &parts.from_upstream {
        chunks.push(format!("from {upstream}"));
    }
    if parts.has_diff() {
        chunks.push(format!("+{}  −{}", parts.lines_added, parts.lines_removed));
    }
    chunks.join("  ·  ")
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

fn vertical_tabs_layout_active(app: &AppContext) -> bool {
    matches!(
        resolved_project_organization_tab_layout(
            FeatureFlag::RepositoryWorkspaces.is_enabled(),
            FeatureFlag::VerticalTabs.is_enabled() && *TabSettings::as_ref(app).use_vertical_tabs,
        ),
        TabLayout::Vertical
    )
}

fn left_traffic_light_width(app: &AppContext, window_id: WindowId) -> f32 {
    let zoom_factor = WindowSettings::as_ref(app).zoom_level.as_zoom_factor();
    traffic_light_data(app, window_id)
        .as_ref()
        .filter(|data| data.side == TrafficLightSide::Left)
        .map(|data| data.width(zoom_factor))
        .unwrap_or(0.)
}

fn is_macos_fullscreen(app: &AppContext, window_id: WindowId) -> bool {
    let is_window_fullscreen = app
        .windows()
        .platform_window(window_id)
        .map(|window| window.fullscreen_state() == FullscreenState::Fullscreen)
        .unwrap_or(false);
    is_window_fullscreen && cfg!(target_os = "macos")
}

/// 与 Workspace 相同的 chrome 谓词，从 AppContext 当场计算侧栏头 inset。
///
/// `left_panel_showing` 表示本侧栏当前正在显示（`LeftPanelView::render` 里为 true）。
pub(crate) fn left_panel_titlebar_leading_inset_from_app(
    app: &AppContext,
    left_panel_showing: bool,
    simplified_wasm_tab_bar: bool,
    window_id: WindowId,
) -> f32 {
    let full_height_chrome = use_full_height_left_panel_chrome(
        FeatureFlag::RepositoryWorkspaces.is_enabled(),
        left_panel_showing,
        simplified_wasm_tab_bar,
        vertical_tabs_layout_active(app),
        warpui::platform::is_mobile_device(),
    );
    left_panel_titlebar_leading_inset(
        full_height_chrome,
        is_macos_fullscreen(app, window_id),
        left_traffic_light_width(app, window_id),
    )
}

#[cfg(test)]
#[path = "full_height_left_panel_chrome_tests.rs"]
mod tests;

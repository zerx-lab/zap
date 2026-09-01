use super::{
    header_items_excluding_lifted_tools_panel, left_panel_titlebar_leading_inset,
    short_upstream_name, tab_bar_leading_padding, use_full_height_left_panel_chrome,
    use_workspace_info_bar, workspace_info_bar_label, workspace_info_bar_parts,
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
fn use_workspace_info_bar_requires_full_height_chrome_and_active_workspace() {
    assert!(use_workspace_info_bar(true, true));
    assert!(!use_workspace_info_bar(true, false));
    assert!(!use_workspace_info_bar(false, true));
    assert!(!use_workspace_info_bar(false, false));
}

#[test]
fn short_upstream_name_strips_remote_ref_prefix() {
    assert_eq!(short_upstream_name("refs/remotes/origin/main"), "main");
    assert_eq!(
        short_upstream_name("refs/remotes/origin/feature/a"),
        "feature/a"
    );
    assert_eq!(short_upstream_name("main"), "main");
}

#[test]
fn workspace_info_bar_label_omits_missing_upstream_and_zero_diff() {
    assert_eq!(
        workspace_info_bar_label("feature-a", None, Some(0), Some(0)),
        "feature-a"
    );
    assert_eq!(
        workspace_info_bar_label("feature-a", Some("refs/remotes/origin/main"), Some(12), Some(3)),
        "feature-a  ·  from main  ·  +12  −3"
    );
    assert_eq!(
        workspace_info_bar_label("feature-a", Some("refs/remotes/origin/main"), None, None),
        "feature-a  ·  from main"
    );
}

#[test]
fn workspace_info_bar_parts_exposes_diff_counts_for_colored_render() {
    let parts = workspace_info_bar_parts(
        "fix",
        Some("refs/remotes/origin/main"),
        Some(46),
        Some(1),
    );
    assert_eq!(parts.from_upstream.as_deref(), Some("main"));
    assert_eq!(parts.lines_added, 46);
    assert_eq!(parts.lines_removed, 1);
    assert!(parts.has_diff());
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

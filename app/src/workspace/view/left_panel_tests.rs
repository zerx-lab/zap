use super::{LEFT_PANEL_DIVIDER_WIDTH, LEFT_PANEL_DRAGBAR_OUTWARD_OFFSET};

#[test]
fn left_panel_divider_matches_pane_split_thickness() {
    assert_eq!(LEFT_PANEL_DIVIDER_WIDTH, 1.);
}

#[test]
fn left_panel_resize_hit_target_extends_past_the_divider() {
    assert!(LEFT_PANEL_DRAGBAR_OUTWARD_OFFSET > LEFT_PANEL_DIVIDER_WIDTH);
    assert!(
        LEFT_PANEL_DRAGBAR_OUTWARD_OFFSET < 5.,
        "offset should keep part of the 5px dragbar on the panel so the cursor still hits the line"
    );
}

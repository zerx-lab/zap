# Workspace Terminal Activity Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the existing repository workspace tab-count UI with a C3-style numeric badge, add a distinct selected-workspace glow, and animate the badge only when that workspace has a long-running terminal.

**Architecture:** Keep tab ownership in `RepositoryWorkspaceTabSets`, keep terminal running detection in `Workspace`, and keep presentation in `ProjectTreePanel`. `ProjectTreePanel` receives `tab_counts`, `active_workspace_id`, and `running_workspace_ids` from `Workspace::sync_project_tree`; it does not traverse terminal sessions.

**Tech Stack:** Rust, WarpUI elements (`Container`, `Border`, `DropShadow`, `Flex`, `Hoverable`), existing `BrailleSpinner`, existing `RunningSessionSummary`, existing repository workspace tab state.

---

## File Structure

- Modify `app/src/workspace/repository_workspace_tabs.rs`
  - Add a generic helper to map active/inactive repository workspace tabs into workspace IDs by predicate.
- Modify `app/src/workspace/repository_workspace_tabs_tests.rs`
  - Add unit coverage for active/inactive matching and unclassified tabs.
- Modify `app/src/project_organization/view/project_tree.rs`
  - Replace the old textual `"N tabs"` badge with a numeric activity badge.
  - Add running-workspace state input and per-workspace spinner state handles.
  - Add selected workspace frame/glow rendering separate from running badge animation.
- Modify `app/src/project_organization/view/project_tree_tests.rs`
  - Update tab-count label tests.
  - Add visual-state helper tests and render smoke coverage.
- Modify `app/src/workspace/view/left_panel.rs`
  - Add a forwarding setter for running repository workspace IDs.
- Modify `app/src/workspace/view.rs`
  - Compute running repository workspace IDs using all active and inactive repository workspace tabs.
  - Sync running IDs to `LeftPanelView`.
  - Refresh the project tree on `TerminalViewStateChanged`.

---

### Task 1: Add Repository Workspace Tab Predicate Helper

**Files:**
- Modify: `app/src/workspace/repository_workspace_tabs.rs`
- Modify: `app/src/workspace/repository_workspace_tabs_tests.rs`

- [ ] **Step 1: Write failing tests for matching active and inactive workspace tabs**

Add these tests to `app/src/workspace/repository_workspace_tabs_tests.rs`:

```rust
#[test]
fn workspace_ids_matching_includes_active_and_inactive_workspaces() {
    let workspace_a = RepositoryWorkspaceId(uuid::Uuid::from_u128(1));
    let workspace_b = RepositoryWorkspaceId(uuid::Uuid::from_u128(2));
    let workspace_c = RepositoryWorkspaceId(uuid::Uuid::from_u128(3));
    let mut sets = RepositoryWorkspaceTabSets::new(Some(workspace_a));
    sets.insert_inactive(
        Some(workspace_b),
        RepositoryWorkspaceTabState::new(vec![20_u64, 21], 0),
    );
    sets.insert_inactive(
        Some(workspace_c),
        RepositoryWorkspaceTabState::new(vec![30_u64], 0),
    );

    let active_tabs = vec![10_u64, 11];
    let matches = sets.workspace_ids_matching(&active_tabs, |tab| *tab == 11 || *tab == 20);

    assert!(matches.contains(&workspace_a));
    assert!(matches.contains(&workspace_b));
    assert!(!matches.contains(&workspace_c));
}

#[test]
fn workspace_ids_matching_ignores_unclassified_tabs() {
    let workspace_a = RepositoryWorkspaceId(uuid::Uuid::from_u128(1));
    let mut sets = RepositoryWorkspaceTabSets::new(Some(workspace_a));
    sets.insert_inactive(None, RepositoryWorkspaceTabState::new(vec![20_u64], 0));

    let active_tabs = vec![10_u64];
    let matches = sets.workspace_ids_matching(&active_tabs, |tab| *tab == 20);

    assert!(matches.is_empty());
}
```

- [ ] **Step 2: Run the focused tests and verify they fail**

Run:

```bash
cargo test -p warp --lib workspace::repository_workspace_tabs_tests::workspace_ids_matching
```

Expected: fails because `workspace_ids_matching` does not exist.

- [ ] **Step 3: Implement `workspace_ids_matching`**

Update the import and impl in `app/src/workspace/repository_workspace_tabs.rs`:

```rust
use std::collections::{HashMap, HashSet};
```

Add this method inside `impl<T> RepositoryWorkspaceTabSets<T>`:

```rust
    pub(crate) fn workspace_ids_matching(
        &self,
        active_tabs: &[T],
        mut matches_tab: impl FnMut(&T) -> bool,
    ) -> HashSet<RepositoryWorkspaceId> {
        let mut workspace_ids = HashSet::new();

        if let Some(workspace_id) = self.active_workspace_id {
            if active_tabs.iter().any(&mut matches_tab) {
                workspace_ids.insert(workspace_id);
            }
        }

        for (workspace_id, state) in &self.inactive {
            let Some(workspace_id) = workspace_id else {
                continue;
            };
            if state.tabs.iter().any(&mut matches_tab) {
                workspace_ids.insert(*workspace_id);
            }
        }

        workspace_ids
    }
```

- [ ] **Step 4: Run the focused tests and verify they pass**

Run:

```bash
cargo test -p warp --lib workspace::repository_workspace_tabs_tests::workspace_ids_matching
```

Expected: both tests pass.

- [ ] **Step 5: Commit Task 1**

```bash
git add app/src/workspace/repository_workspace_tabs.rs app/src/workspace/repository_workspace_tabs_tests.rs
git commit -m "feat: find repository workspaces matching tabs"
```

---

### Task 2: Add Project Tree Activity State Inputs

**Files:**
- Modify: `app/src/project_organization/view/project_tree.rs`
- Modify: `app/src/project_organization/view/project_tree_tests.rs`

- [ ] **Step 1: Write failing tests for badge labels and visual state**

Update the import list in `app/src/project_organization/view/project_tree_tests.rs`:

```rust
use super::{
    repository_add_workspace_position_id, resolved_project_organization_tab_layout,
    should_show_workspace_delete_button, synchronize_mouse_states, tab_count_badge_label,
    workspace_count_label, workspace_row_is_selected, ProjectTreeEvent, ProjectTreePanel,
    ProjectTreeState, RepositoryTreeNode, TabLayout, WorkspaceTreeNode, WorkspaceVisualState,
};
```

Replace the tab-count assertions in `project_tree_count_labels_use_correct_singular_and_plural_forms`:

```rust
#[test]
fn project_tree_count_labels_use_correct_singular_and_plural_forms() {
    assert_eq!(workspace_count_label(0), "0 workspaces");
    assert_eq!(workspace_count_label(1), "1 workspace");
    assert_eq!(workspace_count_label(2), "2 workspaces");
}

#[test]
fn tab_count_badge_label_is_numeric_and_caps_large_counts() {
    assert_eq!(tab_count_badge_label(0), "0");
    assert_eq!(tab_count_badge_label(1), "1");
    assert_eq!(tab_count_badge_label(99), "99");
    assert_eq!(tab_count_badge_label(100), "99+");
}
```

Add this visual-state test:

```rust
#[test]
fn workspace_visual_state_keeps_selection_and_running_separate() {
    let selected_static = WorkspaceVisualState::new(true, false);
    assert!(selected_static.should_render_selection_frame());
    assert!(!selected_static.should_render_running_spinner());

    let running_unselected = WorkspaceVisualState::new(false, true);
    assert!(!running_unselected.should_render_selection_frame());
    assert!(running_unselected.should_render_running_spinner());

    let selected_running = WorkspaceVisualState::new(true, true);
    assert!(selected_running.should_render_selection_frame());
    assert!(selected_running.should_render_running_spinner());
}
```

- [ ] **Step 2: Run the focused tests and verify they fail**

Run:

```bash
cargo test -p warp --lib project_organization::view::project_tree_tests
```

Expected: fails because `tab_count_badge_label` and `WorkspaceVisualState` do not exist.

- [ ] **Step 3: Add state fields and pure helpers**

Update imports in `app/src/project_organization/view/project_tree.rs`:

```rust
use pathfinder_geometry::vector::vec2f;
use warp_core::ui::color::coloru_with_opacity;
use warpui::{
    elements::{
        Border, ChildView, ConstrainedBox, Container, CornerRadius, CrossAxisAlignment, DropShadow,
        Element, Empty, Flex, Hoverable, MainAxisAlignment, MainAxisSize, MouseStateHandle,
        ParentElement, Radius, SavePosition, Shrinkable, Text,
    },
    platform::Cursor,
    text_layout::ClipConfig,
    ui_components::components::UiComponent,
    AppContext, Entity, ModelHandle, SingletonEntity, TypedActionView, View, ViewContext,
    ViewHandle,
};
```

Add the spinner import:

```rust
use crate::ui_components::{
    buttons::icon_button,
    icons,
    spinner::{BrailleSpinner, SpinnerStateHandle},
};
```

Replace `tab_count_label` with:

```rust
fn tab_count_badge_label(tab_count: usize) -> String {
    if tab_count > 99 {
        "99+".to_string()
    } else {
        tab_count.to_string()
    }
}
```

Add this helper type near `workspace_row_is_selected`:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct WorkspaceVisualState {
    is_selected: bool,
    has_running_terminal: bool,
}

impl WorkspaceVisualState {
    pub(crate) fn new(is_selected: bool, has_running_terminal: bool) -> Self {
        Self {
            is_selected,
            has_running_terminal,
        }
    }

    pub(crate) fn should_render_selection_frame(&self) -> bool {
        self.is_selected
    }

    pub(crate) fn should_render_running_spinner(&self) -> bool {
        self.has_running_terminal
    }
}
```

Add fields to `ProjectTreePanel`:

```rust
    running_workspace_ids: HashSet<RepositoryWorkspaceId>,
    workspace_spinner_states: HashMap<RepositoryWorkspaceId, SpinnerStateHandle>,
```

Initialize them in `ProjectTreePanel::new`:

```rust
            running_workspace_ids: HashSet::new(),
            workspace_spinner_states: HashMap::new(),
```

In `refresh_tree`, after `synchronize_mouse_states(&mut self.workspace_delete_mouse_states, &workspace_ids);`, add:

```rust
        self.running_workspace_ids
            .retain(|workspace_id| workspace_ids.contains(workspace_id));
        self.workspace_spinner_states
            .retain(|workspace_id, _| workspace_ids.contains(workspace_id));
        for workspace_id in &workspace_ids {
            self.workspace_spinner_states
                .entry(*workspace_id)
                .or_default();
        }
```

Add a setter to `impl ProjectTreePanel`:

```rust
    pub fn set_running_workspaces(
        &mut self,
        running_workspace_ids: HashSet<RepositoryWorkspaceId>,
        ctx: &mut ViewContext<Self>,
    ) {
        if self.running_workspace_ids == running_workspace_ids {
            return;
        }
        self.running_workspace_ids = running_workspace_ids;
        ctx.notify();
    }
```

- [ ] **Step 4: Run the focused tests and verify they pass**

Run:

```bash
cargo test -p warp --lib project_organization::view::project_tree_tests
```

Expected: both tests pass.

- [ ] **Step 5: Commit Task 2**

```bash
git add app/src/project_organization/view/project_tree.rs app/src/project_organization/view/project_tree_tests.rs
git commit -m "feat: add workspace activity visual state"
```

---

### Task 3: Replace Workspace Row Tab Count UI

**Files:**
- Modify: `app/src/project_organization/view/project_tree.rs`
- Modify: `app/src/project_organization/view/project_tree_tests.rs`

- [ ] **Step 1: Add render smoke coverage**

Add this test to `app/src/project_organization/view/project_tree_tests.rs` after `project_tree_renders_workspace_rows_with_finite_flex_constraints`:

```rust
#[test]
fn project_tree_renders_running_selected_workspace_activity_badge() {
    App::test((), |mut app| async move {
        app.add_singleton_model(|_| Appearance::mock());
        let tempdir = tempfile::tempdir().expect("temporary directory should be created");
        let repository_path = tempdir.path().join("dip-agent");
        let worktree_path = tempdir.path().join("feature-worktree");
        std::fs::create_dir(&repository_path).expect("repository directory should be created");
        std::fs::create_dir(&worktree_path).expect("worktree directory should be created");
        let repository_id = RepositoryId(uuid::Uuid::from_u128(1));
        let workspace_id = RepositoryWorkspaceId(uuid::Uuid::from_u128(2));
        let timestamp = chrono::DateTime::from_timestamp(0, 0)
            .expect("timestamp should be valid")
            .naive_utc();
        app.add_singleton_model(|ctx| {
            ProjectOrganizationModel::try_new(
                vec![PersistedRepository {
                    id: repository_id.to_string(),
                    display_name: "dip-agent".to_string(),
                    path: repository_path.to_string_lossy().to_string(),
                    remote_url: None,
                    source: "local".to_string(),
                    created_at: timestamp,
                    last_opened_at: timestamp,
                }],
                vec![PersistedRepositoryWorkspace {
                    id: workspace_id.to_string(),
                    repository_id: repository_id.to_string(),
                    display_name: "feature-workspace".to_string(),
                    branch: "feature/workspace".to_string(),
                    worktree_path: worktree_path.to_string_lossy().to_string(),
                    created_at: timestamp,
                    last_opened_at: timestamp,
                }],
                RepositoryPersistence::new(None),
                ctx,
            )
            .expect("project organization model should initialize")
        });

        let (window_id, host) =
            app.add_window(WindowStyle::NotStealFocus, ProjectTreeTestHost::new);
        let project_tree = host.read(&app, |host, _| host.project_tree.clone());
        project_tree.update(&mut app, |project_tree, ctx| {
            project_tree.set_tab_counts(HashMap::from([(workspace_id, 3)]), ctx);
            project_tree.set_active_workspace(Some(workspace_id), ctx);
            project_tree.set_running_workspaces(HashSet::from([workspace_id]), ctx);
        });
        let root_view_id = app
            .root_view_id(window_id)
            .expect("window should have a root view");
        let mut presenter = Presenter::new(window_id);

        app.update(|ctx| {
            presenter.invalidate(
                WindowInvalidation {
                    updated: [root_view_id, project_tree.id()].into_iter().collect(),
                    ..Default::default()
                },
                ctx,
            );
            presenter.build_scene(vec2f(360., 240.), 1., None, ctx);
        });
    });
}
```

- [ ] **Step 2: Run the render smoke test**

Run:

```bash
cargo test -p warp --lib project_organization::view::project_tree_tests::project_tree_renders_running_selected_workspace_activity_badge
```

Expected after Task 2: passes as smoke coverage for a selected workspace with running state before the visual replacement. Keep this test unchanged so Task 3 verifies the new badge and selected-frame rendering still lays out.

- [ ] **Step 3: Add badge and selected-frame rendering helpers**

Add this free helper near `tab_count_badge_label`:

```rust
fn apply_workspace_selection_frame(
    row_container: Container,
    visual_state: WorkspaceVisualState,
    selected_border_color: pathfinder_color::ColorU,
    selected_shadow_color: pathfinder_color::ColorU,
) -> Container {
    if !visual_state.should_render_selection_frame() {
        return row_container;
    }

    row_container
        .with_border(Border::all(1.).with_border_fill(selected_border_color))
        .with_drop_shadow(
            DropShadow::new_with_standard_offset_and_spread(selected_shadow_color)
                .with_offset(vec2f(0., 0.)),
        )
}
```

Add this method inside `impl ProjectTreePanel` before `render_workspace_row`:

```rust
    fn render_workspace_activity_badge(
        &self,
        tab_count: usize,
        visual_state: WorkspaceVisualState,
        workspace_id: RepositoryWorkspaceId,
        appearance: &Appearance,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();
        let metadata_color = theme.sub_text_color(theme.background());
        let running_color: pathfinder_color::ColorU = theme.terminal_colors().normal.green.into();
        let badge_background = if visual_state.should_render_running_spinner() {
            coloru_with_opacity(running_color, 14).into()
        } else {
            theme.surface_2()
        };
        let border_fill = if visual_state.should_render_running_spinner() {
            coloru_with_opacity(running_color, 42).into()
        } else {
            theme.surface_3()
        };

        let mut content = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_spacing(4.);

        if visual_state.should_render_running_spinner() {
            let spinner_state = self
                .workspace_spinner_states
                .get(&workspace_id)
                .expect("workspace spinner state should be initialized during tree refresh")
                .clone();
            content.add_child(
                ConstrainedBox::new(
                    Box::new(BrailleSpinner::new(
                        appearance.ui_font_family(),
                        appearance.ui_font_footnote(),
                        running_color,
                        spinner_state,
                    )),
                )
                .with_width(10.)
                .with_height(12.)
                .finish(),
            );
        }

        content.add_child(
            Text::new_inline(
                tab_count_badge_label(tab_count),
                appearance.ui_font_family(),
                appearance.ui_font_footnote(),
            )
            .with_color(metadata_color.into())
            .finish(),
        );

        let mut badge = Container::new(content.finish())
            .with_horizontal_padding(6.)
            .with_vertical_padding(2.)
            .with_background(badge_background)
            .with_border(Border::all(1.).with_border_fill(border_fill))
            .with_corner_radius(CornerRadius::with_all(Radius::Pixels(12.)));
        if visual_state.should_render_running_spinner() {
            badge = badge.with_drop_shadow(
                DropShadow::new_with_standard_offset_and_spread(coloru_with_opacity(
                    running_color,
                    30,
                ))
                .with_offset(vec2f(0., 0.)),
            );
        }
        badge.finish()
    }

```

- [ ] **Step 4: Replace old tab-count container usage in `render_workspace_row`**

In `render_workspace_row`, remove:

```rust
        let tab_count_background = if selected {
            selection_accent.with_opacity(20)
        } else {
            theme.surface_2()
        };
        let tab_count = Container::new(
            Text::new_inline(
                tab_count_label(workspace.tab_count),
                appearance.ui_font_family(),
                appearance.ui_font_footnote(),
            )
            .with_color(metadata_color.into())
            .finish(),
        )
        .with_horizontal_padding(6.)
        .with_vertical_padding(2.)
        .with_background(tab_count_background)
        .with_corner_radius(CornerRadius::with_all(Radius::Pixels(4.)))
        .finish();
```

Add:

```rust
        let visual_state = WorkspaceVisualState::new(
            selected,
            self.running_workspace_ids.contains(&workspace.workspace_id),
        );
        let tab_count = self.render_workspace_activity_badge(
            workspace.tab_count,
            visual_state,
            workspace_id,
            appearance,
        );
        let selected_color: pathfinder_color::ColorU =
            theme.terminal_colors().normal.blue.into();
        let selected_border_color = coloru_with_opacity(selected_color, 58);
        let selected_shadow_color = coloru_with_opacity(selected_color, 34);
```

Then, inside the `Hoverable` closure, after constructing `row_container`, replace the selected container styling block with:

```rust
                if selected {
                    row_container =
                        row_container.with_background(selection_accent.with_opacity(10));
                } else if mouse_state.is_hovered() {
                    row_container = row_container.with_background(theme.surface_overlay_2());
                } else {
                    row_container = row_container.with_background(theme.surface_overlay_1());
                }
                row_container = apply_workspace_selection_frame(
                    row_container,
                    visual_state,
                    selected_border_color,
                    selected_shadow_color,
                );
```

- [ ] **Step 5: Run focused project tree tests**

Run:

```bash
cargo test -p warp --lib project_organization::view::project_tree_tests
```

Expected: all project tree tests pass.

- [ ] **Step 6: Commit Task 3**

```bash
git add app/src/project_organization/view/project_tree.rs app/src/project_organization/view/project_tree_tests.rs
git commit -m "feat: render workspace activity badge"
```

---

### Task 4: Sync Running Repository Workspace IDs

**Files:**
- Modify: `app/src/workspace/view/left_panel.rs`
- Modify: `app/src/workspace/view.rs`

- [ ] **Step 1: Add `LeftPanelView` forwarding setter**

In `app/src/workspace/view/left_panel.rs`, add this method after `set_project_tree_active_workspace`:

```rust
    pub fn set_project_tree_running_workspaces(
        &mut self,
        running_workspace_ids: HashSet<RepositoryWorkspaceId>,
        ctx: &mut ViewContext<Self>,
    ) {
        self.project_tree_view.update(ctx, |tree, ctx| {
            tree.set_running_workspaces(running_workspace_ids, ctx);
        });
    }
```

Keep the existing `HashMap` and `HashSet` import at the top of `left_panel.rs`:

```rust
use std::collections::{HashMap, HashSet};
```

- [ ] **Step 2: Import `RunningSessionSummary`**

In `app/src/workspace/view.rs`, change:

```rust
use crate::session_management::{SessionNavigationData, SessionSource};
```

to:

```rust
use crate::session_management::{RunningSessionSummary, SessionNavigationData, SessionSource};
```

- [ ] **Step 3: Add helper for per-tab long-running detection**

Add this method near `all_repository_workspace_tabs` in `impl Workspace`:

```rust
    fn tab_has_long_running_terminal(&self, tab: &TabData, ctx: &AppContext) -> bool {
        let pane_group = tab.pane_group.as_ref(ctx);
        let sessions = pane_group
            .pane_sessions(tab.pane_group.id(), tab.pane_group.window_id(ctx), ctx)
            .collect_vec();
        !RunningSessionSummary::new(&sessions)
            .long_running_cmds
            .is_empty()
    }
```

Add this method after it:

```rust
    fn repository_workspace_ids_with_long_running_terminal(
        &self,
        ctx: &AppContext,
    ) -> HashSet<RepositoryWorkspaceId> {
        self.repository_workspace_tabs
            .workspace_ids_matching(&self.tabs, |tab| self.tab_has_long_running_terminal(tab, ctx))
    }
```

Ensure `HashSet` is imported in `workspace/view.rs`; the file already uses `HashMap`, so the import should become:

```rust
use std::collections::{HashMap, HashSet};
```

- [ ] **Step 4: Sync running IDs to project tree**

Update `sync_project_tree`:

```rust
    fn sync_project_tree(&mut self, ctx: &mut ViewContext<Self>) {
        if !FeatureFlag::RepositoryWorkspaces.is_enabled() {
            return;
        }

        let tab_counts = self.repository_workspace_tabs.tab_counts(&self.tabs);
        let active_workspace_id = self.active_repository_workspace_id();
        let running_workspace_ids = self.repository_workspace_ids_with_long_running_terminal(ctx);
        self.left_panel_view.update(ctx, |left_panel, ctx| {
            left_panel.set_project_tree_tab_counts(tab_counts, ctx);
            left_panel.set_project_tree_active_workspace(active_workspace_id, ctx);
            left_panel.set_project_tree_running_workspaces(running_workspace_ids, ctx);
        });
    }
```

- [ ] **Step 5: Refresh project tree on terminal state changes**

In the `pane_group::Event::TerminalViewStateChanged` match arm in `app/src/workspace/view.rs`, change:

```rust
            pane_group::Event::TerminalViewStateChanged => {
                self.update_active_session(ctx);
                ctx.notify();
            }
```

to:

```rust
            pane_group::Event::TerminalViewStateChanged => {
                self.update_active_session(ctx);
                self.sync_project_tree(ctx);
                ctx.notify();
            }
```

- [ ] **Step 6: Run focused tests**

Run:

```bash
cargo test -p warp --lib workspace::repository_workspace_tabs_tests project_organization::view::project_tree_tests
```

Expected: all focused tests pass.

- [ ] **Step 7: Commit Task 4**

```bash
git add app/src/workspace/view/left_panel.rs app/src/workspace/view.rs
git commit -m "feat: sync running workspace activity"
```

---

### Task 5: Final Verification

**Files:**
- Verify only; no planned source edits.

- [ ] **Step 1: Run focused unit tests**

Run:

```bash
cargo test -p warp --lib workspace::repository_workspace_tabs_tests project_organization::view::project_tree_tests
```

Expected: all tests pass.

- [ ] **Step 2: Run cargo check**

Run:

```bash
cargo check
```

Expected: completes successfully.

- [ ] **Step 3: Inspect diff for scope**

Run:

```bash
git diff --stat HEAD~4..HEAD
git diff HEAD~4..HEAD -- app/src/project_organization/view/project_tree.rs app/src/workspace/view.rs app/src/workspace/view/left_panel.rs app/src/workspace/repository_workspace_tabs.rs
```

Expected:
- Only repository workspace activity UI/state files changed.
- No terminal PTY output path changes.
- No string parsing or fallback logic for workspace identity.
- New badge number is `tab_count`, not long-running terminal count.

- [ ] **Step 4: Confirm no extra source edits were needed**

Run:

```bash
git status --short
```

Expected: no uncommitted source changes remain after the Task 1-4 commits.

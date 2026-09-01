# Workspace Selection State Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the selected repository workspace immediately identifiable in the project tree with a surface background, accent outline, and left accent stripe.

**Architecture:** Keep selection state in `ProjectTreeState::selected_workspace_id`. `ProjectTreePanel::render_workspace_row` will derive the selected visual treatment from that state and wrap the existing row element with existing WarpUI `Container`/`Border` primitives. The delete-button hover behavior and row click action remain unchanged.

**Tech Stack:** Rust, WarpUI elements, `Appearance` theme colors, Cargo unit tests.

---

### Task 1: Add a testable selection-style decision

**Files:**
- Modify: `app/src/project_organization/view/project_tree.rs:219-230`
- Modify: `app/src/project_organization/view/project_tree.rs:467-563`
- Test: `app/src/project_organization/view/project_tree_tests.rs:25-220`

- [ ] **Step 1: Write the failing test**

Add a small pure helper that maps the current workspace id and row workspace id to a boolean selected state, then test both matching and non-matching ids. Keep the existing state-selection tests unchanged.

```rust
fn workspace_row_is_selected(
    selected_workspace_id: Option<RepositoryWorkspaceId>,
    workspace_id: RepositoryWorkspaceId,
) -> bool {
    selected_workspace_id == Some(workspace_id)
}
```

```rust
#[test]
fn workspace_row_selection_matches_only_the_active_workspace() {
    let selected = RepositoryWorkspaceId(uuid::Uuid::from_u128(1));
    let other = RepositoryWorkspaceId(uuid::Uuid::from_u128(2));

    assert!(workspace_row_is_selected(Some(selected), selected));
    assert!(!workspace_row_is_selected(Some(selected), other));
    assert!(!workspace_row_is_selected(None, selected));
}
```

- [ ] **Step 2: Run the focused test to verify it fails**

Run:

```bash
cargo test -p warp workspace_row_selection_matches_only_the_active_workspace --lib
```

Expected: compilation failure because `workspace_row_is_selected` is not defined yet.

- [ ] **Step 3: Implement the helper and use it in row rendering**

Define the helper near the existing project-tree pure helpers and replace the inline comparison in `render_workspace_row`:

```rust
let selected = workspace_row_is_selected(
    self.state.selected_workspace_id(),
    workspace.workspace_id,
);
```

- [ ] **Step 4: Add the selected visual treatment**

Import `Border` from `warpui::elements`. Keep the existing content, delete button, placeholder, hover state, and click handlers. Apply the selected style to the row container only when `selected` is true:

```rust
let mut row_container = Container::new(row)
    .with_padding_left(36.)
    .with_vertical_padding(4.);

if selected {
    row_container = row_container
        .with_background(theme.surface_2())
        .with_border(Border::all(1.).with_border_fill(theme.accent()))
        .with_margin_top(-1.)
        .with_margin_bottom(-1.)
        .with_margin_left(-1.)
        .with_margin_right(-1.);
}
```

Add the left accent stripe as a stretched first child inside a selected-only flex wrapper. The stripe's negative left margin cancels its fixed width, so text and the row's available width do not shift:

```rust
let row = if selected {
    Flex::row()
        .with_main_axis_size(MainAxisSize::Max)
        .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
        .with_child(
            Container::new(
                ConstrainedBox::new(Empty::new().finish())
                    .with_width(3.)
                    .finish(),
            )
                .with_margin_left(-3.)
                .with_background(theme.accent())
                .finish(),
        )
        .with_child(Shrinkable::new(1.0, row_container.finish()).finish())
        .finish()
} else {
    row_container.finish()
};
```

Use `row` as the child returned by `Hoverable::new`; do not change the existing `delete`/`delete_placeholder` selection inside the hover closure.

- [ ] **Step 5: Run the focused tests to verify they pass**

Run:

```bash
cargo test -p warp project_tree --lib
```

Expected: all project-tree tests pass, including the new selection decision test and the existing delete-hover test.

- [ ] **Step 6: Run repository verification**

Run:

```bash
cargo check -p warp
git diff --check
```

Expected: both commands exit with status 0. Existing unrelated compiler warnings may remain.

- [ ] **Step 7: Commit the implementation**

```bash
git add app/src/project_organization/view/project_tree.rs app/src/project_organization/view/project_tree_tests.rs
git commit -m "fix: strengthen workspace selection state"
```

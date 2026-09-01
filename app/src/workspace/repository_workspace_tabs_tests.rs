use crate::project_organization::domain::RepositoryWorkspaceId;

use super::repository_workspace_tabs::{RepositoryWorkspaceTabSets, RepositoryWorkspaceTabState};

#[test]
fn switching_workspaces_swaps_tabs_without_dropping_inactive_state() {
    let workspace_a = RepositoryWorkspaceId(uuid::Uuid::from_u128(1));
    let workspace_b = RepositoryWorkspaceId(uuid::Uuid::from_u128(2));
    let mut active_tabs = vec![10_u64];
    let mut active_tab_index = 0;
    let mut sets = RepositoryWorkspaceTabSets::new(Some(workspace_a));
    sets.insert_inactive(
        Some(workspace_b),
        RepositoryWorkspaceTabState::new(vec![20_u64], 0),
    );

    sets.switch_to(Some(workspace_b), &mut active_tabs, &mut active_tab_index);
    assert_eq!(active_tabs, vec![20]);

    sets.switch_to(Some(workspace_a), &mut active_tabs, &mut active_tab_index);
    assert_eq!(active_tabs, vec![10]);
}

#[test]
fn switching_to_workspace_without_tabs_yields_empty_active_set() {
    let workspace_a = RepositoryWorkspaceId(uuid::Uuid::from_u128(1));
    let workspace_b = RepositoryWorkspaceId(uuid::Uuid::from_u128(2));
    let mut active_tabs = vec![10_u64];
    let mut active_tab_index = 0;
    let mut sets = RepositoryWorkspaceTabSets::new(Some(workspace_a));

    sets.switch_to(Some(workspace_b), &mut active_tabs, &mut active_tab_index);

    assert!(active_tabs.is_empty());
    assert_eq!(active_tab_index, 0);
    assert_eq!(sets.active_workspace_id(), Some(workspace_b));
}

#[test]
fn switching_workspaces_restores_each_workspace_active_tab_index() {
    let workspace_a = RepositoryWorkspaceId(uuid::Uuid::from_u128(1));
    let workspace_b = RepositoryWorkspaceId(uuid::Uuid::from_u128(2));
    let mut active_tabs = vec![10_u64, 11, 12];
    let mut active_tab_index = 2;
    let mut sets = RepositoryWorkspaceTabSets::new(Some(workspace_a));
    sets.insert_inactive(
        Some(workspace_b),
        RepositoryWorkspaceTabState::new(vec![20_u64], 0),
    );

    sets.switch_to(Some(workspace_b), &mut active_tabs, &mut active_tab_index);
    sets.switch_to(Some(workspace_a), &mut active_tabs, &mut active_tab_index);

    assert_eq!(active_tab_index, 2);
}

#[test]
fn tab_counts_include_active_and_inactive_workspaces() {
    let workspace_a = RepositoryWorkspaceId(uuid::Uuid::from_u128(1));
    let workspace_b = RepositoryWorkspaceId(uuid::Uuid::from_u128(2));
    let mut active_tabs = vec![10_u64, 11];
    let sets = {
        let mut sets = RepositoryWorkspaceTabSets::new(Some(workspace_a));
        sets.insert_inactive(
            Some(workspace_b),
            RepositoryWorkspaceTabState::new(vec![20_u64, 21, 22], 1),
        );
        sets.insert_inactive(None, RepositoryWorkspaceTabState::new(vec![30_u64], 0));
        sets
    };

    assert_eq!(sets.tab_counts(&active_tabs).get(&workspace_a), Some(&2));
    assert_eq!(sets.tab_counts(&active_tabs).get(&workspace_b), Some(&3));

    active_tabs.clear();
    assert_eq!(sets.tab_counts(&active_tabs).get(&workspace_a), None);
}

#[test]
fn map_tabs_lists_active_and_inactive_repository_workspaces_in_order() {
    let workspace_a = RepositoryWorkspaceId(uuid::Uuid::from_u128(1));
    let workspace_b = RepositoryWorkspaceId(uuid::Uuid::from_u128(2));
    let mut sets = RepositoryWorkspaceTabSets::new(Some(workspace_a));
    sets.insert_inactive(
        Some(workspace_b),
        RepositoryWorkspaceTabState::new(vec![20_u64, 21], 1),
    );
    sets.insert_inactive(None, RepositoryWorkspaceTabState::new(vec![30_u64], 0));

    let active_tabs = vec![10_u64, 11];
    let mapped = sets.map_tabs(&active_tabs, 1, |tab, index, is_active| {
        (*tab, index, is_active)
    });

    assert_eq!(
        mapped.get(&workspace_a),
        Some(&vec![(10, 0, false), (11, 1, true)])
    );
    assert_eq!(
        mapped.get(&workspace_b),
        Some(&vec![(20, 0, false), (21, 1, false)])
    );
    assert_eq!(mapped.len(), 2);
}

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

#[test]
fn taking_an_inactive_workspace_removes_only_its_tab_state() {
    let workspace_a = RepositoryWorkspaceId(uuid::Uuid::from_u128(1));
    let workspace_b = RepositoryWorkspaceId(uuid::Uuid::from_u128(2));
    let mut sets = RepositoryWorkspaceTabSets::new(Some(workspace_a));
    sets.insert_inactive(
        Some(workspace_b),
        RepositoryWorkspaceTabState::new(vec![20_u64], 0),
    );

    let removed = sets
        .take_inactive(Some(workspace_b))
        .expect("workspace state should be removed");

    assert_eq!(removed.tabs, vec![20]);
    assert!(sets.inactive_states().next().is_none());
    assert_eq!(sets.active_workspace_id(), Some(workspace_a));
}

#[test]
fn finds_inactive_workspace_containing_target_tab() {
    let workspace_a = RepositoryWorkspaceId(uuid::Uuid::from_u128(1));
    let workspace_b = RepositoryWorkspaceId(uuid::Uuid::from_u128(2));
    let mut sets = RepositoryWorkspaceTabSets::new(Some(workspace_a));
    sets.insert_inactive(
        Some(workspace_b),
        RepositoryWorkspaceTabState::new(vec![20_u64], 0),
    );

    assert_eq!(
        sets.find_inactive_workspace(|tab| *tab == 20),
        Some(Some(workspace_b))
    );
    assert_eq!(sets.find_inactive_workspace(|tab| *tab == 10), None);
}

#[test]
fn map_last_matching_keeps_later_tab_in_the_same_workspace() {
    let workspace_a = RepositoryWorkspaceId(uuid::Uuid::from_u128(1));
    let workspace_b = RepositoryWorkspaceId(uuid::Uuid::from_u128(2));
    let mut sets = RepositoryWorkspaceTabSets::new(Some(workspace_a));
    sets.insert_inactive(
        Some(workspace_b),
        RepositoryWorkspaceTabState::new(vec![20_u64, 21, 22], 0),
    );

    let active_tabs = vec![10_u64, 11, 12];
    let matches = sets.map_last_matching(&active_tabs, |tab| match tab {
        11 | 12 => Some(*tab),
        20 | 22 => Some(*tab),
        _ => None,
    });

    assert_eq!(matches.get(&workspace_a), Some(&12));
    assert_eq!(matches.get(&workspace_b), Some(&22));
}

#[test]
fn map_last_matching_ignores_unclassified_tabs() {
    let workspace_a = RepositoryWorkspaceId(uuid::Uuid::from_u128(1));
    let mut sets = RepositoryWorkspaceTabSets::new(Some(workspace_a));
    sets.insert_inactive(None, RepositoryWorkspaceTabState::new(vec![20_u64], 0));

    let matches = sets.map_last_matching(&[10_u64], |tab| (*tab == 20).then_some(*tab));
    assert!(matches.is_empty());
}

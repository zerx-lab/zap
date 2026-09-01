use std::path::PathBuf;

use super::migration::{classify_tab_worktree, WorkspaceHealth, WorktreeIdentity};

#[test]
fn linked_worktree_tab_with_one_identity_is_classified() {
    let identity = WorktreeIdentity {
        repository_path: PathBuf::from("/repository"),
        worktree_path: PathBuf::from("/repository-worktrees/feature-a"),
        branch: "feature/a".to_string(),
    };

    assert_eq!(
        classify_tab_worktree([Some(identity.clone()), Some(identity.clone())]),
        Some(identity)
    );
}

#[test]
fn mixed_worktree_tab_remains_unclassified() {
    let first = WorktreeIdentity {
        repository_path: PathBuf::from("/repository"),
        worktree_path: PathBuf::from("/worktree-a"),
        branch: "feature/a".to_string(),
    };
    let second = WorktreeIdentity {
        repository_path: PathBuf::from("/repository"),
        worktree_path: PathBuf::from("/worktree-b"),
        branch: "feature/b".to_string(),
    };

    assert_eq!(classify_tab_worktree([Some(first), Some(second)]), None);
}

#[test]
fn tab_without_recognized_worktree_remains_unclassified() {
    assert_eq!(classify_tab_worktree([None, None]), None);
}

#[test]
fn health_states_preserve_branch_mismatch_details() {
    assert_eq!(
        WorkspaceHealth::WorktreeBranchMismatch {
            actual: "feature/actual".to_string(),
        },
        WorkspaceHealth::WorktreeBranchMismatch {
            actual: "feature/actual".to_string(),
        }
    );
}

use super::startup_directory_with_repository_workspace_fallback;
use crate::terminal::session_settings::WorkingDirectoryMode;
use std::path::PathBuf;

#[test]
fn previous_dir_uses_worktree_when_no_prior_session() {
    assert_eq!(
        startup_directory_with_repository_workspace_fallback(
            None,
            WorkingDirectoryMode::PreviousDir,
            Some(PathBuf::from("/tmp/feature-worktree")),
        ),
        Some(PathBuf::from("/tmp/feature-worktree"))
    );
}

#[test]
fn previous_dir_replaces_cwd_outside_worktree_with_worktree() {
    assert_eq!(
        startup_directory_with_repository_workspace_fallback(
            Some(PathBuf::from("/Users/admin")),
            WorkingDirectoryMode::PreviousDir,
            Some(PathBuf::from("/tmp/feature-worktree")),
        ),
        Some(PathBuf::from("/tmp/feature-worktree"))
    );
}

#[test]
fn previous_dir_keeps_cwd_inside_worktree() {
    assert_eq!(
        startup_directory_with_repository_workspace_fallback(
            Some(PathBuf::from("/tmp/feature-worktree/src")),
            WorkingDirectoryMode::PreviousDir,
            Some(PathBuf::from("/tmp/feature-worktree")),
        ),
        Some(PathBuf::from("/tmp/feature-worktree/src"))
    );
}

#[test]
fn home_dir_uses_worktree_in_repository_workspace() {
    assert_eq!(
        startup_directory_with_repository_workspace_fallback(
            None,
            WorkingDirectoryMode::HomeDir,
            Some(PathBuf::from("/tmp/feature-worktree")),
        ),
        Some(PathBuf::from("/tmp/feature-worktree"))
    );
}

#[test]
fn custom_dir_falls_back_to_worktree_when_empty() {
    assert_eq!(
        startup_directory_with_repository_workspace_fallback(
            None,
            WorkingDirectoryMode::CustomDir,
            Some(PathBuf::from("/tmp/feature-worktree")),
        ),
        Some(PathBuf::from("/tmp/feature-worktree"))
    );
}

#[test]
fn custom_dir_keeps_explicit_path() {
    assert_eq!(
        startup_directory_with_repository_workspace_fallback(
            Some(PathBuf::from("/opt/custom")),
            WorkingDirectoryMode::CustomDir,
            Some(PathBuf::from("/tmp/feature-worktree")),
        ),
        Some(PathBuf::from("/opt/custom"))
    );
}

#[test]
fn previous_dir_without_worktree_stays_none() {
    assert_eq!(
        startup_directory_with_repository_workspace_fallback(
            None,
            WorkingDirectoryMode::PreviousDir,
            None,
        ),
        None
    );
}

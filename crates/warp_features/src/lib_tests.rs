use super::{FeatureFlag, DOGFOOD_FLAGS, PREVIEW_FLAGS, RELEASE_FLAGS};

#[test]
fn repository_workspaces_is_dogfood_only() {
    assert!(DOGFOOD_FLAGS.contains(&FeatureFlag::RepositoryWorkspaces));
    assert!(!PREVIEW_FLAGS.contains(&FeatureFlag::RepositoryWorkspaces));
    assert!(!RELEASE_FLAGS.contains(&FeatureFlag::RepositoryWorkspaces));
}

#[test]
fn cli_agent_session_resume_is_dogfood_only() {
    assert!(DOGFOOD_FLAGS.contains(&FeatureFlag::CliAgentSessionResume));
    assert!(!PREVIEW_FLAGS.contains(&FeatureFlag::CliAgentSessionResume));
    assert!(!RELEASE_FLAGS.contains(&FeatureFlag::CliAgentSessionResume));
}

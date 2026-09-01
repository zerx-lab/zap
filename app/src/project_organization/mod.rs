pub mod domain;
pub mod git;
pub mod migration;
pub mod model;
pub(crate) mod project_tree_tab;
pub mod view;
pub(crate) mod workspace_agent_activity;

#[cfg(test)]
#[path = "git_tests.rs"]
mod git_tests;

#[cfg(test)]
#[path = "migration_tests.rs"]
mod migration_tests;

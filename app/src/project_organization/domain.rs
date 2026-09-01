use std::{fmt, path::PathBuf, str::FromStr};

use chrono::NaiveDateTime;
use thiserror::Error;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct RepositoryId(pub Uuid);

impl fmt::Display for RepositoryId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl From<Uuid> for RepositoryId {
    fn from(value: Uuid) -> Self {
        Self(value)
    }
}

impl From<RepositoryId> for Uuid {
    fn from(value: RepositoryId) -> Self {
        value.0
    }
}

impl FromStr for RepositoryId {
    type Err = RepositoryIdParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(value)
            .map(Self)
            .map_err(|source| RepositoryIdParseError {
                value: value.to_string(),
                source,
            })
    }
}

impl TryFrom<String> for RepositoryId {
    type Error = RepositoryIdParseError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl TryFrom<&str> for RepositoryId {
    type Error = RepositoryIdParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

#[derive(Debug, Error)]
#[error("invalid repository ID `{value}`: {source}")]
pub struct RepositoryIdParseError {
    pub value: String,
    #[source]
    pub source: uuid::Error,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct RepositoryWorkspaceId(pub Uuid);

impl fmt::Display for RepositoryWorkspaceId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl From<Uuid> for RepositoryWorkspaceId {
    fn from(value: Uuid) -> Self {
        Self(value)
    }
}

impl From<RepositoryWorkspaceId> for Uuid {
    fn from(value: RepositoryWorkspaceId) -> Self {
        value.0
    }
}

impl FromStr for RepositoryWorkspaceId {
    type Err = RepositoryWorkspaceIdParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(value)
            .map(Self)
            .map_err(|source| RepositoryWorkspaceIdParseError {
                value: value.to_string(),
                source,
            })
    }
}

impl TryFrom<String> for RepositoryWorkspaceId {
    type Error = RepositoryWorkspaceIdParseError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl TryFrom<&str> for RepositoryWorkspaceId {
    type Error = RepositoryWorkspaceIdParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

#[derive(Debug, Error)]
#[error("invalid repository workspace ID `{value}`: {source}")]
pub struct RepositoryWorkspaceIdParseError {
    pub value: String,
    #[source]
    pub source: uuid::Error,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RepositorySource {
    Local,
    Cloned,
}

impl fmt::Display for RepositorySource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Local => formatter.write_str("local"),
            Self::Cloned => formatter.write_str("cloned"),
        }
    }
}

impl FromStr for RepositorySource {
    type Err = RepositorySourceParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "local" => Ok(Self::Local),
            "cloned" => Ok(Self::Cloned),
            value => Err(RepositorySourceParseError {
                value: value.to_string(),
            }),
        }
    }
}

impl TryFrom<String> for RepositorySource {
    type Error = RepositorySourceParseError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl TryFrom<&str> for RepositorySource {
    type Error = RepositorySourceParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

#[derive(Debug, Error)]
#[error("invalid repository source `{value}`; expected `local` or `cloned`")]
pub struct RepositorySourceParseError {
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Repository {
    pub id: RepositoryId,
    pub display_name: String,
    pub path: PathBuf,
    pub remote_url: Option<String>,
    pub source: RepositorySource,
    pub created_at: NaiveDateTime,
    pub last_opened_at: NaiveDateTime,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepositoryWorkspace {
    pub id: RepositoryWorkspaceId,
    pub repository_id: RepositoryId,
    pub display_name: String,
    pub branch: String,
    pub worktree_path: PathBuf,
    pub created_at: NaiveDateTime,
    pub last_opened_at: NaiveDateTime,
}

#[derive(Debug, Error)]
pub enum ProjectOrganizationError {
    #[error(
        "repository path `{canonical_path}` already belongs to repository {existing_repository_id}"
    )]
    RepositoryAlreadyExists {
        existing_repository_id: RepositoryId,
        canonical_path: PathBuf,
    },
    #[error(
        "canonical repository path `{canonical_path}` matches multiple repositories: {repository_ids:?}"
    )]
    AmbiguousRepositoryPath {
        canonical_path: PathBuf,
        repository_ids: Vec<RepositoryId>,
    },
    #[error(
        "branch `{branch}` already belongs to workspace {existing_workspace_id} in repository {repository_id}"
    )]
    WorkspaceBranchAlreadyExists {
        repository_id: RepositoryId,
        branch: String,
        existing_workspace_id: RepositoryWorkspaceId,
    },
    #[error(
        "worktree path `{canonical_path}` already belongs to workspace {existing_workspace_id}"
    )]
    WorkspacePathAlreadyExists {
        existing_workspace_id: RepositoryWorkspaceId,
        canonical_path: PathBuf,
    },
    #[error(
        "canonical worktree path `{canonical_path}` matches multiple workspaces: {workspace_ids:?}"
    )]
    AmbiguousWorkspacePath {
        canonical_path: PathBuf,
        workspace_ids: Vec<RepositoryWorkspaceId>,
    },
    #[error("repository {repository_id} still has workspaces")]
    RepositoryHasWorkspaces { repository_id: RepositoryId },
    #[error("repository {repository_id} does not exist")]
    RepositoryNotFound { repository_id: RepositoryId },
    #[error("repository workspace {workspace_id} does not exist")]
    WorkspaceNotFound { workspace_id: RepositoryWorkspaceId },
    #[error("repository ID {repository_id} already exists")]
    RepositoryIdAlreadyExists { repository_id: RepositoryId },
    #[error("repository workspace ID {workspace_id} already exists")]
    WorkspaceIdAlreadyExists { workspace_id: RepositoryWorkspaceId },
    #[error("persisted repository has invalid ID `{value}`: {source}")]
    InvalidPersistedRepositoryId {
        value: String,
        #[source]
        source: RepositoryIdParseError,
    },
    #[error("persisted repository workspace has invalid ID `{value}`: {source}")]
    InvalidPersistedWorkspaceId {
        value: String,
        #[source]
        source: RepositoryWorkspaceIdParseError,
    },
    #[error("persisted repository workspace has invalid repository ID `{value}`: {source}")]
    InvalidPersistedWorkspaceRepositoryId {
        value: String,
        #[source]
        source: RepositoryIdParseError,
    },
    #[error("persisted repository has invalid source `{value}`: {source}")]
    InvalidPersistedRepositorySource {
        value: String,
        #[source]
        source: RepositorySourceParseError,
    },
    #[error("path `{path}` cannot be canonicalized: {source}")]
    InvalidPath {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("path `{path}` is not valid UTF-8 and cannot be persisted")]
    InvalidPathEncoding { path: PathBuf },
    #[error("failed to persist {operation}: {details}")]
    Persistence {
        operation: &'static str,
        details: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProjectOrganizationEvent {
    RepositoryAdded { repository_id: RepositoryId },
    RepositoryUpdated { repository_id: RepositoryId },
    RepositoryRemoved { repository_id: RepositoryId },
    WorkspaceAdded { workspace_id: RepositoryWorkspaceId },
    WorkspaceUpdated { workspace_id: RepositoryWorkspaceId },
    WorkspaceRemoved { workspace_id: RepositoryWorkspaceId },
}

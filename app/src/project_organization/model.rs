use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use chrono::Utc;
use uuid::Uuid;
use warpui::{Entity, ModelContext, SingletonEntity};

use crate::persistence::{
    model::{Repository as PersistedRepository, RepositoryWorkspace as PersistedWorkspace},
    RepositoryPersistence, RepositoryPersistenceOperation,
};

use super::domain::{
    ProjectOrganizationError, ProjectOrganizationEvent, Repository, RepositoryId, RepositorySource,
    RepositoryWorkspace, RepositoryWorkspaceId,
};

pub struct ProjectOrganizationModel {
    repositories: HashMap<RepositoryId, Repository>,
    repository_ids_by_path: HashMap<PathBuf, RepositoryId>,
    workspaces: HashMap<RepositoryWorkspaceId, RepositoryWorkspace>,
    workspace_ids_by_repository_branch: HashMap<(RepositoryId, String), RepositoryWorkspaceId>,
    workspace_ids_by_path: HashMap<PathBuf, RepositoryWorkspaceId>,
    persistence: RepositoryPersistence,
}

enum CanonicalPathMatch<Id> {
    None,
    Unique(Id),
    Ambiguous(Vec<Id>),
}

impl Entity for ProjectOrganizationModel {
    type Event = ProjectOrganizationEvent;
}

impl SingletonEntity for ProjectOrganizationModel {}

impl ProjectOrganizationModel {
    pub fn try_new(
        persisted_repositories: Vec<PersistedRepository>,
        persisted_workspaces: Vec<PersistedWorkspace>,
        persistence: RepositoryPersistence,
        _ctx: &mut ModelContext<Self>,
    ) -> Result<Self, ProjectOrganizationError> {
        let mut model = Self {
            repositories: HashMap::new(),
            repository_ids_by_path: HashMap::new(),
            workspaces: HashMap::new(),
            workspace_ids_by_repository_branch: HashMap::new(),
            workspace_ids_by_path: HashMap::new(),
            persistence,
        };

        for repository in persisted_repositories {
            let repository = Self::repository_from_persisted(repository)?;
            model.insert_repository_checked(repository)?;
        }
        for workspace in persisted_workspaces {
            let workspace = Self::workspace_from_persisted(workspace)?;
            model.insert_workspace_checked(workspace)?;
        }

        Ok(model)
    }

    pub fn add_local_repository(
        &mut self,
        path: impl AsRef<Path>,
        ctx: &mut ModelContext<Self>,
    ) -> Result<RepositoryId, ProjectOrganizationError> {
        self.add_local_repository_with_optional_remote(path, None, ctx)
    }

    pub fn add_local_repository_with_remote(
        &mut self,
        path: impl AsRef<Path>,
        remote_url: String,
        ctx: &mut ModelContext<Self>,
    ) -> Result<RepositoryId, ProjectOrganizationError> {
        self.add_local_repository_with_optional_remote(path, Some(remote_url), ctx)
    }

    /// 添加本地 repository，并原子创建其主 worktree 对应的 local workspace。
    pub fn add_local_repository_with_initial_workspace(
        &mut self,
        path: impl AsRef<Path>,
        remote_url: Option<String>,
        primary_branch: impl Into<String>,
        ctx: &mut ModelContext<Self>,
    ) -> Result<(RepositoryId, RepositoryWorkspaceId), ProjectOrganizationError> {
        let canonical_path = Self::canonicalize(path.as_ref())?;
        let now = Utc::now().naive_utc();
        let repository = Repository {
            id: RepositoryId::from(Uuid::new_v4()),
            display_name: canonical_path
                .file_name()
                .and_then(|name| name.to_str())
                .filter(|name| !name.is_empty())
                .ok_or_else(|| ProjectOrganizationError::InvalidPathEncoding {
                    path: canonical_path.clone(),
                })?
                .to_string(),
            path: canonical_path.clone(),
            remote_url,
            source: RepositorySource::Local,
            created_at: now,
            last_opened_at: now,
        };
        self.validate_new_repository(&repository)?;

        let workspace = RepositoryWorkspace {
            id: RepositoryWorkspaceId::from(Uuid::new_v4()),
            repository_id: repository.id,
            display_name: "local".to_string(),
            branch: primary_branch.into(),
            worktree_path: canonical_path,
            created_at: repository.created_at,
            last_opened_at: repository.last_opened_at,
        };
        if self.workspaces.contains_key(&workspace.id) {
            return Err(ProjectOrganizationError::WorkspaceIdAlreadyExists {
                workspace_id: workspace.id,
            });
        }
        if let Some(existing_workspace_id) = self
            .workspace_ids_by_repository_branch
            .get(&(workspace.repository_id, workspace.branch.clone()))
        {
            return Err(ProjectOrganizationError::WorkspaceBranchAlreadyExists {
                repository_id: workspace.repository_id,
                branch: workspace.branch.clone(),
                existing_workspace_id: *existing_workspace_id,
            });
        }
        if let Some(existing_workspace_id) =
            self.workspace_ids_by_path.get(&workspace.worktree_path)
        {
            return Err(ProjectOrganizationError::WorkspacePathAlreadyExists {
                existing_workspace_id: *existing_workspace_id,
                canonical_path: workspace.worktree_path.clone(),
            });
        }

        self.persist(
            RepositoryPersistenceOperation::UpsertRepositoryWithWorkspace {
                repository: Self::persisted_repository(&repository)?,
                workspace: Self::persisted_workspace(&workspace)?,
            },
            "repository addition with initial workspace",
        )?;
        let repository_id = repository.id;
        let workspace_id = workspace.id;
        self.commit_repository(repository);
        self.commit_workspace(workspace);
        ctx.emit(ProjectOrganizationEvent::RepositoryAdded { repository_id });
        ctx.emit(ProjectOrganizationEvent::WorkspaceAdded { workspace_id });
        Ok((repository_id, workspace_id))
    }

    fn add_local_repository_with_optional_remote(
        &mut self,
        path: impl AsRef<Path>,
        remote_url: Option<String>,
        ctx: &mut ModelContext<Self>,
    ) -> Result<RepositoryId, ProjectOrganizationError> {
        let canonical_path = Self::canonicalize(path.as_ref())?;
        match self.repository_match_for_canonical_path(&canonical_path, None) {
            CanonicalPathMatch::None => {}
            CanonicalPathMatch::Unique(existing_repository_id) => {
                return Err(ProjectOrganizationError::RepositoryAlreadyExists {
                    existing_repository_id,
                    canonical_path,
                });
            }
            CanonicalPathMatch::Ambiguous(repository_ids) => {
                return Err(ProjectOrganizationError::AmbiguousRepositoryPath {
                    canonical_path,
                    repository_ids,
                });
            }
        }
        let display_name = canonical_path
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.is_empty())
            .ok_or_else(|| ProjectOrganizationError::InvalidPathEncoding {
                path: canonical_path.clone(),
            })?
            .to_string();
        let now = Utc::now().naive_utc();
        let repository = Repository {
            id: RepositoryId::from(Uuid::new_v4()),
            display_name,
            path: canonical_path,
            remote_url,
            source: RepositorySource::Local,
            created_at: now,
            last_opened_at: now,
        };
        let repository_id = repository.id;

        self.persist_repository(&repository, "repository addition")?;
        self.commit_repository(repository);
        ctx.emit(ProjectOrganizationEvent::RepositoryAdded { repository_id });
        Ok(repository_id)
    }

    pub fn touch_repository_path(
        &mut self,
        path: impl AsRef<Path>,
        ctx: &mut ModelContext<Self>,
    ) -> Result<RepositoryId, ProjectOrganizationError> {
        let canonical_path = Self::canonicalize(path.as_ref())?;
        let repository_id = match self.repository_match_for_canonical_path(&canonical_path, None) {
            CanonicalPathMatch::None => return self.add_local_repository(canonical_path, ctx),
            CanonicalPathMatch::Unique(repository_id) => repository_id,
            CanonicalPathMatch::Ambiguous(repository_ids) => {
                return Err(ProjectOrganizationError::AmbiguousRepositoryPath {
                    canonical_path,
                    repository_ids,
                });
            }
        };
        let mut repository = self
            .repositories
            .get(&repository_id)
            .expect("repository path index must reference an existing repository")
            .clone();
        let previous_path = repository.path.clone();
        repository.path = canonical_path;
        repository.last_opened_at = Utc::now().naive_utc();

        self.persist_repository(&repository, "repository access timestamp update")?;
        self.repository_ids_by_path.remove(&previous_path);
        self.repository_ids_by_path
            .insert(repository.path.clone(), repository_id);
        self.repositories.insert(repository_id, repository);
        ctx.emit(ProjectOrganizationEvent::RepositoryUpdated { repository_id });
        Ok(repository_id)
    }

    pub fn insert_repository(
        &mut self,
        mut repository: Repository,
        ctx: &mut ModelContext<Self>,
    ) -> Result<(), ProjectOrganizationError> {
        repository.path = Self::canonicalize(&repository.path)?;
        if self.repositories.contains_key(&repository.id) {
            return Err(ProjectOrganizationError::RepositoryIdAlreadyExists {
                repository_id: repository.id,
            });
        }
        match self.repository_match_for_canonical_path(&repository.path, None) {
            CanonicalPathMatch::None => {}
            CanonicalPathMatch::Unique(existing_repository_id) => {
                return Err(ProjectOrganizationError::RepositoryAlreadyExists {
                    existing_repository_id,
                    canonical_path: repository.path,
                });
            }
            CanonicalPathMatch::Ambiguous(repository_ids) => {
                return Err(ProjectOrganizationError::AmbiguousRepositoryPath {
                    canonical_path: repository.path,
                    repository_ids,
                });
            }
        }
        self.persist_repository(&repository, "repository insertion")?;
        let repository_id = repository.id;
        self.commit_repository(repository);
        ctx.emit(ProjectOrganizationEvent::RepositoryAdded { repository_id });
        Ok(())
    }

    pub fn update_repository(
        &mut self,
        mut repository: Repository,
        ctx: &mut ModelContext<Self>,
    ) -> Result<(), ProjectOrganizationError> {
        repository.path = Self::canonicalize(&repository.path)?;
        let previous = self.repositories.get(&repository.id).cloned().ok_or(
            ProjectOrganizationError::RepositoryNotFound {
                repository_id: repository.id,
            },
        )?;
        match self.repository_match_for_canonical_path(&repository.path, Some(repository.id)) {
            CanonicalPathMatch::None => {}
            CanonicalPathMatch::Unique(existing_repository_id) => {
                return Err(ProjectOrganizationError::RepositoryAlreadyExists {
                    existing_repository_id,
                    canonical_path: repository.path,
                });
            }
            CanonicalPathMatch::Ambiguous(repository_ids) => {
                return Err(ProjectOrganizationError::AmbiguousRepositoryPath {
                    canonical_path: repository.path,
                    repository_ids,
                });
            }
        }
        let previous_path = previous.path;
        let repository_id = repository.id;

        self.persist_repository(&repository, "repository update")?;
        self.repository_ids_by_path.remove(&previous_path);
        self.repository_ids_by_path
            .insert(repository.path.clone(), repository_id);
        self.repositories.insert(repository_id, repository);
        ctx.emit(ProjectOrganizationEvent::RepositoryUpdated { repository_id });
        Ok(())
    }

    pub fn rename_repository(
        &mut self,
        repository_id: RepositoryId,
        display_name: String,
        ctx: &mut ModelContext<Self>,
    ) -> Result<(), ProjectOrganizationError> {
        let mut repository = self
            .repositories
            .get(&repository_id)
            .cloned()
            .ok_or(ProjectOrganizationError::RepositoryNotFound { repository_id })?;
        repository.display_name = display_name;
        self.update_repository(repository, ctx)
    }

    pub fn remove_repository(
        &mut self,
        repository_id: RepositoryId,
        ctx: &mut ModelContext<Self>,
    ) -> Result<Repository, ProjectOrganizationError> {
        if self
            .workspace_ids_by_repository_branch
            .keys()
            .any(|(workspace_repository_id, _)| *workspace_repository_id == repository_id)
        {
            return Err(ProjectOrganizationError::RepositoryHasWorkspaces { repository_id });
        }
        let repository = self
            .repositories
            .get(&repository_id)
            .cloned()
            .ok_or(ProjectOrganizationError::RepositoryNotFound { repository_id })?;

        self.persist(
            RepositoryPersistenceOperation::DeleteRepository {
                repository_id: repository_id.to_string(),
            },
            "repository removal",
        )?;
        self.repositories.remove(&repository_id);
        self.repository_ids_by_path.remove(&repository.path);
        ctx.emit(ProjectOrganizationEvent::RepositoryRemoved { repository_id });
        Ok(repository)
    }

    pub fn insert_workspace(
        &mut self,
        mut workspace: RepositoryWorkspace,
        ctx: &mut ModelContext<Self>,
    ) -> Result<(), ProjectOrganizationError> {
        workspace.worktree_path = Self::canonicalize(&workspace.worktree_path)?;
        if self.workspaces.contains_key(&workspace.id) {
            return Err(ProjectOrganizationError::WorkspaceIdAlreadyExists {
                workspace_id: workspace.id,
            });
        }
        if !self.repositories.contains_key(&workspace.repository_id) {
            return Err(ProjectOrganizationError::RepositoryNotFound {
                repository_id: workspace.repository_id,
            });
        }
        let branch_key = (workspace.repository_id, workspace.branch.clone());
        if let Some(existing_workspace_id) =
            self.workspace_ids_by_repository_branch.get(&branch_key)
        {
            return Err(ProjectOrganizationError::WorkspaceBranchAlreadyExists {
                repository_id: workspace.repository_id,
                branch: workspace.branch,
                existing_workspace_id: *existing_workspace_id,
            });
        }
        match self.workspace_match_for_canonical_path(&workspace.worktree_path, None) {
            CanonicalPathMatch::None => {}
            CanonicalPathMatch::Unique(existing_workspace_id) => {
                return Err(ProjectOrganizationError::WorkspacePathAlreadyExists {
                    existing_workspace_id,
                    canonical_path: workspace.worktree_path,
                });
            }
            CanonicalPathMatch::Ambiguous(workspace_ids) => {
                return Err(ProjectOrganizationError::AmbiguousWorkspacePath {
                    canonical_path: workspace.worktree_path,
                    workspace_ids,
                });
            }
        }
        self.persist_workspace(&workspace, "repository workspace insertion")?;
        let workspace_id = workspace.id;
        self.commit_workspace(workspace);
        ctx.emit(ProjectOrganizationEvent::WorkspaceAdded { workspace_id });
        Ok(())
    }

    pub fn update_workspace(
        &mut self,
        mut workspace: RepositoryWorkspace,
        ctx: &mut ModelContext<Self>,
    ) -> Result<(), ProjectOrganizationError> {
        workspace.worktree_path = Self::canonicalize(&workspace.worktree_path)?;
        let previous = self.workspaces.get(&workspace.id).cloned().ok_or(
            ProjectOrganizationError::WorkspaceNotFound {
                workspace_id: workspace.id,
            },
        )?;
        if !self.repositories.contains_key(&workspace.repository_id) {
            return Err(ProjectOrganizationError::RepositoryNotFound {
                repository_id: workspace.repository_id,
            });
        }
        let branch_key = (workspace.repository_id, workspace.branch.clone());
        if let Some(existing_workspace_id) =
            self.workspace_ids_by_repository_branch.get(&branch_key)
        {
            if *existing_workspace_id != workspace.id {
                return Err(ProjectOrganizationError::WorkspaceBranchAlreadyExists {
                    repository_id: workspace.repository_id,
                    branch: workspace.branch,
                    existing_workspace_id: *existing_workspace_id,
                });
            }
        }
        match self.workspace_match_for_canonical_path(&workspace.worktree_path, Some(workspace.id))
        {
            CanonicalPathMatch::None => {}
            CanonicalPathMatch::Unique(existing_workspace_id) => {
                return Err(ProjectOrganizationError::WorkspacePathAlreadyExists {
                    existing_workspace_id,
                    canonical_path: workspace.worktree_path,
                });
            }
            CanonicalPathMatch::Ambiguous(workspace_ids) => {
                return Err(ProjectOrganizationError::AmbiguousWorkspacePath {
                    canonical_path: workspace.worktree_path,
                    workspace_ids,
                });
            }
        }
        let workspace_id = workspace.id;

        self.persist_workspace(&workspace, "repository workspace update")?;
        self.workspace_ids_by_repository_branch
            .remove(&(previous.repository_id, previous.branch));
        self.workspace_ids_by_path.remove(&previous.worktree_path);
        self.workspace_ids_by_repository_branch
            .insert(branch_key, workspace_id);
        self.workspace_ids_by_path
            .insert(workspace.worktree_path.clone(), workspace_id);
        self.workspaces.insert(workspace_id, workspace);
        ctx.emit(ProjectOrganizationEvent::WorkspaceUpdated { workspace_id });
        Ok(())
    }

    pub fn rename_workspace(
        &mut self,
        workspace_id: RepositoryWorkspaceId,
        display_name: String,
        ctx: &mut ModelContext<Self>,
    ) -> Result<(), ProjectOrganizationError> {
        let mut workspace = self
            .workspaces
            .get(&workspace_id)
            .cloned()
            .ok_or(ProjectOrganizationError::WorkspaceNotFound { workspace_id })?;
        workspace.display_name = display_name;
        self.update_workspace(workspace, ctx)
    }

    pub fn remove_workspace(
        &mut self,
        workspace_id: RepositoryWorkspaceId,
        ctx: &mut ModelContext<Self>,
    ) -> Result<RepositoryWorkspace, ProjectOrganizationError> {
        let workspace = self
            .workspaces
            .get(&workspace_id)
            .cloned()
            .ok_or(ProjectOrganizationError::WorkspaceNotFound { workspace_id })?;
        self.persist(
            RepositoryPersistenceOperation::DeleteRepositoryWorkspace {
                workspace_id: workspace_id.to_string(),
            },
            "repository workspace removal",
        )?;
        self.workspaces.remove(&workspace_id);
        self.workspace_ids_by_repository_branch
            .remove(&(workspace.repository_id, workspace.branch.clone()));
        self.workspace_ids_by_path.remove(&workspace.worktree_path);
        ctx.emit(ProjectOrganizationEvent::WorkspaceRemoved { workspace_id });
        Ok(workspace)
    }

    pub fn repository(&self, repository_id: RepositoryId) -> Option<&Repository> {
        self.repositories.get(&repository_id)
    }

    pub fn workspace(&self, workspace_id: RepositoryWorkspaceId) -> Option<&RepositoryWorkspace> {
        self.workspaces.get(&workspace_id)
    }

    pub fn repositories(&self) -> impl Iterator<Item = &Repository> {
        self.repositories.values()
    }

    pub fn workspaces(&self) -> impl Iterator<Item = &RepositoryWorkspace> {
        self.workspaces.values()
    }

    pub fn workspaces_for_repository(
        &self,
        repository_id: RepositoryId,
    ) -> impl Iterator<Item = &RepositoryWorkspace> {
        self.workspaces
            .values()
            .filter(move |workspace| workspace.repository_id == repository_id)
    }

    fn canonicalize(path: &Path) -> Result<PathBuf, ProjectOrganizationError> {
        dunce::canonicalize(path).map_err(|source| ProjectOrganizationError::InvalidPath {
            path: path.to_path_buf(),
            source,
        })
    }

    fn normalize_persisted_path(path: String) -> PathBuf {
        let path = PathBuf::from(path);
        dunce::canonicalize(&path).unwrap_or(path)
    }

    fn repository_match_for_canonical_path(
        &self,
        canonical_path: &Path,
        excluded_id: Option<RepositoryId>,
    ) -> CanonicalPathMatch<RepositoryId> {
        let mut matches = self
            .repositories
            .iter()
            .filter_map(|(repository_id, repository)| {
                if Some(*repository_id) == excluded_id {
                    return None;
                }
                let candidate = dunce::canonicalize(&repository.path).ok()?;
                (candidate == canonical_path).then_some(*repository_id)
            })
            .collect::<Vec<_>>();
        matches.sort_by_key(|id| id.0);
        match matches.as_slice() {
            [] => CanonicalPathMatch::None,
            [repository_id] => CanonicalPathMatch::Unique(*repository_id),
            _ => CanonicalPathMatch::Ambiguous(matches),
        }
    }

    fn workspace_match_for_canonical_path(
        &self,
        canonical_path: &Path,
        excluded_id: Option<RepositoryWorkspaceId>,
    ) -> CanonicalPathMatch<RepositoryWorkspaceId> {
        let mut matches = self
            .workspaces
            .iter()
            .filter_map(|(workspace_id, workspace)| {
                if Some(*workspace_id) == excluded_id {
                    return None;
                }
                let candidate = dunce::canonicalize(&workspace.worktree_path).ok()?;
                (candidate == canonical_path).then_some(*workspace_id)
            })
            .collect::<Vec<_>>();
        matches.sort_by_key(|id| id.0);
        match matches.as_slice() {
            [] => CanonicalPathMatch::None,
            [workspace_id] => CanonicalPathMatch::Unique(*workspace_id),
            _ => CanonicalPathMatch::Ambiguous(matches),
        }
    }

    fn validate_new_repository(
        &self,
        repository: &Repository,
    ) -> Result<(), ProjectOrganizationError> {
        if self.repositories.contains_key(&repository.id) {
            return Err(ProjectOrganizationError::RepositoryIdAlreadyExists {
                repository_id: repository.id,
            });
        }
        if let Some(existing_repository_id) = self.repository_ids_by_path.get(&repository.path) {
            return Err(ProjectOrganizationError::RepositoryAlreadyExists {
                existing_repository_id: *existing_repository_id,
                canonical_path: repository.path.clone(),
            });
        }
        Ok(())
    }

    fn validate_new_workspace(
        &self,
        workspace: &RepositoryWorkspace,
    ) -> Result<(), ProjectOrganizationError> {
        if self.workspaces.contains_key(&workspace.id) {
            return Err(ProjectOrganizationError::WorkspaceIdAlreadyExists {
                workspace_id: workspace.id,
            });
        }
        if !self.repositories.contains_key(&workspace.repository_id) {
            return Err(ProjectOrganizationError::RepositoryNotFound {
                repository_id: workspace.repository_id,
            });
        }
        let branch_key = (workspace.repository_id, workspace.branch.clone());
        if let Some(existing_workspace_id) =
            self.workspace_ids_by_repository_branch.get(&branch_key)
        {
            return Err(ProjectOrganizationError::WorkspaceBranchAlreadyExists {
                repository_id: workspace.repository_id,
                branch: workspace.branch.clone(),
                existing_workspace_id: *existing_workspace_id,
            });
        }
        if let Some(existing_workspace_id) =
            self.workspace_ids_by_path.get(&workspace.worktree_path)
        {
            return Err(ProjectOrganizationError::WorkspacePathAlreadyExists {
                existing_workspace_id: *existing_workspace_id,
                canonical_path: workspace.worktree_path.clone(),
            });
        }
        Ok(())
    }

    fn insert_repository_checked(
        &mut self,
        repository: Repository,
    ) -> Result<(), ProjectOrganizationError> {
        self.validate_new_repository(&repository)?;
        self.commit_repository(repository);
        Ok(())
    }

    fn commit_repository(&mut self, repository: Repository) {
        debug_assert!(!self.repositories.contains_key(&repository.id));
        self.repository_ids_by_path
            .insert(repository.path.clone(), repository.id);
        self.repositories.insert(repository.id, repository);
    }

    fn insert_workspace_checked(
        &mut self,
        workspace: RepositoryWorkspace,
    ) -> Result<(), ProjectOrganizationError> {
        self.validate_new_workspace(&workspace)?;
        self.commit_workspace(workspace);
        Ok(())
    }

    fn commit_workspace(&mut self, workspace: RepositoryWorkspace) {
        debug_assert!(!self.workspaces.contains_key(&workspace.id));
        self.workspace_ids_by_repository_branch.insert(
            (workspace.repository_id, workspace.branch.clone()),
            workspace.id,
        );
        self.workspace_ids_by_path
            .insert(workspace.worktree_path.clone(), workspace.id);
        self.workspaces.insert(workspace.id, workspace);
    }

    fn repository_from_persisted(
        repository: PersistedRepository,
    ) -> Result<Repository, ProjectOrganizationError> {
        let id = RepositoryId::try_from(repository.id.clone()).map_err(|source| {
            ProjectOrganizationError::InvalidPersistedRepositoryId {
                value: repository.id.clone(),
                source,
            }
        })?;
        let source = RepositorySource::try_from(repository.source.clone()).map_err(|source| {
            ProjectOrganizationError::InvalidPersistedRepositorySource {
                value: repository.source.clone(),
                source,
            }
        })?;
        let path = Self::normalize_persisted_path(repository.path);
        Ok(Repository {
            id,
            display_name: repository.display_name,
            path,
            remote_url: repository.remote_url,
            source,
            created_at: repository.created_at,
            last_opened_at: repository.last_opened_at,
        })
    }

    fn workspace_from_persisted(
        workspace: PersistedWorkspace,
    ) -> Result<RepositoryWorkspace, ProjectOrganizationError> {
        let id = RepositoryWorkspaceId::try_from(workspace.id.clone()).map_err(|source| {
            ProjectOrganizationError::InvalidPersistedWorkspaceId {
                value: workspace.id.clone(),
                source,
            }
        })?;
        let repository_id =
            RepositoryId::try_from(workspace.repository_id.clone()).map_err(|source| {
                ProjectOrganizationError::InvalidPersistedWorkspaceRepositoryId {
                    value: workspace.repository_id.clone(),
                    source,
                }
            })?;
        let worktree_path = Self::normalize_persisted_path(workspace.worktree_path);
        Ok(RepositoryWorkspace {
            id,
            repository_id,
            display_name: workspace.display_name,
            branch: workspace.branch,
            worktree_path,
            created_at: workspace.created_at,
            last_opened_at: workspace.last_opened_at,
        })
    }

    fn persisted_repository(
        repository: &Repository,
    ) -> Result<PersistedRepository, ProjectOrganizationError> {
        let path = repository.path.to_str().ok_or_else(|| {
            ProjectOrganizationError::InvalidPathEncoding {
                path: repository.path.clone(),
            }
        })?;
        Ok(PersistedRepository {
            id: repository.id.to_string(),
            display_name: repository.display_name.clone(),
            path: path.to_string(),
            remote_url: repository.remote_url.clone(),
            source: repository.source.to_string(),
            created_at: repository.created_at,
            last_opened_at: repository.last_opened_at,
        })
    }

    fn persisted_workspace(
        workspace: &RepositoryWorkspace,
    ) -> Result<PersistedWorkspace, ProjectOrganizationError> {
        let worktree_path = workspace.worktree_path.to_str().ok_or_else(|| {
            ProjectOrganizationError::InvalidPathEncoding {
                path: workspace.worktree_path.clone(),
            }
        })?;
        Ok(PersistedWorkspace {
            id: workspace.id.to_string(),
            repository_id: workspace.repository_id.to_string(),
            display_name: workspace.display_name.clone(),
            branch: workspace.branch.clone(),
            worktree_path: worktree_path.to_string(),
            created_at: workspace.created_at,
            last_opened_at: workspace.last_opened_at,
        })
    }

    fn persist_repository(
        &self,
        repository: &Repository,
        operation: &'static str,
    ) -> Result<(), ProjectOrganizationError> {
        self.persist(
            RepositoryPersistenceOperation::UpsertRepository {
                repository: Self::persisted_repository(repository)?,
            },
            operation,
        )
    }

    fn persist_workspace(
        &self,
        workspace: &RepositoryWorkspace,
        operation: &'static str,
    ) -> Result<(), ProjectOrganizationError> {
        self.persist(
            RepositoryPersistenceOperation::UpsertRepositoryWorkspace {
                workspace: Self::persisted_workspace(workspace)?,
            },
            operation,
        )
    }

    fn persist(
        &self,
        operation: RepositoryPersistenceOperation,
        operation_name: &'static str,
    ) -> Result<(), ProjectOrganizationError> {
        self.persistence
            .execute(operation)
            .map_err(|error| ProjectOrganizationError::Persistence {
                operation: operation_name,
                details: error.to_string(),
            })
    }
}

#[cfg(test)]
#[path = "model_tests.rs"]
mod model_tests;

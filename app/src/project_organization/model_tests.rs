use std::{
    path::Path,
    sync::{
        mpsc::{self, Receiver},
        Arc, Mutex,
    },
};

use chrono::{Duration, Utc};
use tempfile::TempDir;
use uuid::Uuid;
use warpui::{App, Entity, ModelHandle};

use crate::{
    persistence::{
        model::{Repository as PersistedRepository, RepositoryWorkspace as PersistedWorkspace},
        ModelEvent, RepositoryPersistence, RepositoryPersistenceError,
        RepositoryPersistenceOperation,
    },
    project_organization::{
        domain::{
            ProjectOrganizationError, ProjectOrganizationEvent, Repository, RepositoryId,
            RepositorySource, RepositoryWorkspace, RepositoryWorkspaceId,
        },
        model::ProjectOrganizationModel,
    },
};

struct PersistenceHarness {
    operations: Receiver<RepositoryPersistenceOperation>,
}

fn acknowledged_persistence(
    result: Result<(), RepositoryPersistenceError>,
) -> (RepositoryPersistence, PersistenceHarness) {
    let (event_sender, event_receiver) = mpsc::sync_channel(20);
    let (operation_sender, operation_receiver) = mpsc::sync_channel(20);
    std::thread::spawn(move || {
        while let Ok(ModelEvent::RepositoryPersistence(request)) = event_receiver.recv() {
            if operation_sender.send(request.operation).is_err() {
                return;
            }
            if request.response.send(result.clone()).is_err() {
                return;
            }
        }
    });
    (
        RepositoryPersistence::new(Some(event_sender)),
        PersistenceHarness {
            operations: operation_receiver,
        },
    )
}

fn create_model(
    app: &mut App,
    repositories: Vec<PersistedRepository>,
    workspaces: Vec<PersistedWorkspace>,
    persistence: RepositoryPersistence,
) -> ModelHandle<ProjectOrganizationModel> {
    app.add_model(|ctx| {
        ProjectOrganizationModel::try_new(repositories, workspaces, persistence, ctx)
            .expect("project organization model should initialize")
    })
}

fn create_acknowledged_model(
    app: &mut App,
    repositories: Vec<PersistedRepository>,
    workspaces: Vec<PersistedWorkspace>,
) -> (ModelHandle<ProjectOrganizationModel>, PersistenceHarness) {
    let (persistence, harness) = acknowledged_persistence(Ok(()));
    (
        create_model(app, repositories, workspaces, persistence),
        harness,
    )
}

struct ProjectOrganizationEventProbe;

impl Entity for ProjectOrganizationEventProbe {
    type Event = ();
}

fn capture_project_organization_events(
    app: &mut App,
    model: &ModelHandle<ProjectOrganizationModel>,
) -> (
    Arc<Mutex<Vec<ProjectOrganizationEvent>>>,
    ModelHandle<ProjectOrganizationEventProbe>,
) {
    let emitted_events = Arc::new(Mutex::new(Vec::new()));
    let captured_events = emitted_events.clone();
    let subscribed_model = model.clone();
    let probe = app.add_model(move |ctx| {
        ctx.subscribe_to_model(&subscribed_model, move |_, event, _| {
            captured_events.lock().unwrap().push(event.clone());
        });
        ProjectOrganizationEventProbe
    });
    (emitted_events, probe)
}

fn persisted_repository(id: RepositoryId, path: &Path) -> PersistedRepository {
    let created_at = Utc::now().naive_utc() - Duration::hours(1);
    PersistedRepository {
        id: id.to_string(),
        display_name: "repository".to_string(),
        path: path
            .to_str()
            .expect("temporary repository path should be valid UTF-8")
            .to_string(),
        remote_url: None,
        source: "local".to_string(),
        created_at,
        last_opened_at: created_at,
    }
}

fn persisted_workspace(
    id: RepositoryWorkspaceId,
    repository_id: RepositoryId,
    branch: &str,
    worktree_path: &Path,
) -> PersistedWorkspace {
    let created_at = Utc::now().naive_utc() - Duration::minutes(30);
    PersistedWorkspace {
        id: id.to_string(),
        repository_id: repository_id.to_string(),
        display_name: branch.to_string(),
        branch: branch.to_string(),
        worktree_path: worktree_path
            .to_str()
            .expect("temporary worktree path should be valid UTF-8")
            .to_string(),
        created_at,
        last_opened_at: created_at,
    }
}

fn initialization_error(
    app: &mut App,
    repositories: Vec<PersistedRepository>,
    workspaces: Vec<PersistedWorkspace>,
) -> ProjectOrganizationError {
    let (sender, receiver) = mpsc::sync_channel(1);
    app.add_model(move |ctx| {
        match ProjectOrganizationModel::try_new(
            repositories,
            workspaces,
            RepositoryPersistence::new(None),
            ctx,
        ) {
            Ok(_) => panic!("project organization initialization should fail"),
            Err(error) => {
                sender
                    .send(error)
                    .expect("initialization error should be captured");
                ProjectOrganizationModel::try_new(
                    vec![],
                    vec![],
                    RepositoryPersistence::new(None),
                    ctx,
                )
                .expect("empty project organization model should initialize")
            }
        }
    });
    receiver
        .recv()
        .expect("project organization initialization should return an error")
}

fn repository_workspace(
    id: RepositoryWorkspaceId,
    repository_id: RepositoryId,
    branch: &str,
    worktree_path: &Path,
) -> RepositoryWorkspace {
    let created_at = Utc::now().naive_utc() - Duration::minutes(30);
    RepositoryWorkspace {
        id,
        repository_id,
        display_name: branch.to_string(),
        branch: branch.to_string(),
        worktree_path: worktree_path.to_path_buf(),
        created_at,
        last_opened_at: created_at,
    }
}

fn repository(id: RepositoryId, path: &Path) -> Repository {
    let created_at = Utc::now().naive_utc() - Duration::hours(1);
    Repository {
        id,
        display_name: "repository".to_string(),
        path: path.to_path_buf(),
        remote_url: None,
        source: RepositorySource::Local,
        created_at,
        last_opened_at: created_at,
    }
}

fn assert_persistence_failure(error: ProjectOrganizationError, operation: &'static str) {
    match error {
        ProjectOrganizationError::Persistence {
            operation: actual_operation,
            details,
        } => {
            assert_eq!(actual_operation, operation);
            assert!(details.contains("injected failure"));
        }
        error => panic!("expected persistence error, got {error:?}"),
    }
}

#[test]
fn add_local_repository_rejects_duplicate_canonical_path() {
    App::test((), |mut app| async move {
        let tempdir = TempDir::new().expect("temporary directory should be created");
        let repository_path = tempdir.path().join("repository");
        std::fs::create_dir(&repository_path).expect("repository directory should be created");
        let alias_path = repository_path.join("..").join("repository");
        let canonical_path =
            dunce::canonicalize(&repository_path).expect("repository path should canonicalize");
        let (model, _events) = create_acknowledged_model(&mut app, vec![], vec![]);

        let repository_id = model
            .update(&mut app, |model, ctx| {
                model.add_local_repository(&repository_path, ctx)
            })
            .expect("first repository should be added");
        let error = model
            .update(&mut app, |model, ctx| {
                model.add_local_repository(&alias_path, ctx)
            })
            .expect_err("canonical duplicate should be rejected");

        assert!(matches!(
            error,
            ProjectOrganizationError::RepositoryAlreadyExists {
                existing_repository_id,
                canonical_path: existing_path,
            } if existing_repository_id == repository_id && existing_path == canonical_path
        ));
    });
}

#[test]
fn add_local_repository_with_remote_persists_remote_url() {
    App::test((), |mut app| async move {
        let tempdir = TempDir::new().unwrap();
        let repository_path = tempdir.path().join("repository");
        std::fs::create_dir(&repository_path).unwrap();
        let (model, _operations) = create_acknowledged_model(&mut app, vec![], vec![]);

        let repository_id = model
            .update(&mut app, |model, ctx| {
                model.add_local_repository_with_remote(
                    &repository_path,
                    "https://example.com/zap.git".to_string(),
                    ctx,
                )
            })
            .unwrap();

        assert_eq!(
            model.read(&app, |model, _| {
                model
                    .repository(repository_id)
                    .and_then(|repository| repository.remote_url.as_deref())
                    .map(str::to_string)
            }),
            Some("https://example.com/zap.git".to_string())
        );
    });
}

#[test]
fn add_repository_rejects_ambiguous_recovered_aliases() {
    App::test((), |mut app| async move {
        let tempdir = TempDir::new().unwrap();
        let target = tempdir.path().join("repository");
        let first_alias = tempdir.path().join("first").join("..").join("repository");
        let second_alias = tempdir.path().join("second").join("..").join("repository");
        let first_id = RepositoryId::from(Uuid::from_u128(1));
        let second_id = RepositoryId::from(Uuid::from_u128(2));
        let (model, _operations) = create_acknowledged_model(
            &mut app,
            vec![
                persisted_repository(first_id, &first_alias),
                persisted_repository(second_id, &second_alias),
            ],
            vec![],
        );
        std::fs::create_dir(tempdir.path().join("first")).unwrap();
        std::fs::create_dir(tempdir.path().join("second")).unwrap();
        std::fs::create_dir(&target).unwrap();

        let error = model
            .update(&mut app, |model, ctx| {
                model.add_local_repository(&target, ctx)
            })
            .unwrap_err();

        assert!(matches!(
            error,
            ProjectOrganizationError::AmbiguousRepositoryPath {
                canonical_path,
                repository_ids,
            } if canonical_path == dunce::canonicalize(&target).unwrap()
                && repository_ids == vec![first_id, second_id]
        ));
    });
}

#[test]
fn insert_repository_rejects_ambiguous_recovered_aliases() {
    App::test((), |mut app| async move {
        let tempdir = TempDir::new().unwrap();
        let target = tempdir.path().join("repository");
        let first_alias = tempdir.path().join("first").join("..").join("repository");
        let second_alias = tempdir.path().join("second").join("..").join("repository");
        let first_id = RepositoryId::from(Uuid::from_u128(1));
        let second_id = RepositoryId::from(Uuid::from_u128(2));
        let (model, _operations) = create_acknowledged_model(
            &mut app,
            vec![
                persisted_repository(first_id, &first_alias),
                persisted_repository(second_id, &second_alias),
            ],
            vec![],
        );
        std::fs::create_dir(tempdir.path().join("first")).unwrap();
        std::fs::create_dir(tempdir.path().join("second")).unwrap();
        std::fs::create_dir(&target).unwrap();

        let error = model
            .update(&mut app, |model, ctx| {
                model.insert_repository(
                    repository(RepositoryId::from(Uuid::from_u128(3)), &target),
                    ctx,
                )
            })
            .unwrap_err();

        assert!(matches!(
            error,
            ProjectOrganizationError::AmbiguousRepositoryPath {
                canonical_path,
                repository_ids,
            } if canonical_path == dunce::canonicalize(&target).unwrap()
                && repository_ids == vec![first_id, second_id]
        ));
    });
}

#[test]
fn update_repository_rejects_ambiguous_recovered_aliases_excluding_itself() {
    App::test((), |mut app| async move {
        let tempdir = TempDir::new().unwrap();
        let target = tempdir.path().join("repository");
        let first_alias = tempdir.path().join("first").join("..").join("repository");
        let second_alias = tempdir.path().join("second").join("..").join("repository");
        let updated_alias = tempdir.path().join("updated").join("..").join("repository");
        let first_id = RepositoryId::from(Uuid::from_u128(1));
        let second_id = RepositoryId::from(Uuid::from_u128(2));
        let updated_id = RepositoryId::from(Uuid::from_u128(3));
        let (model, _operations) = create_acknowledged_model(
            &mut app,
            vec![
                persisted_repository(first_id, &first_alias),
                persisted_repository(second_id, &second_alias),
                persisted_repository(updated_id, &updated_alias),
            ],
            vec![],
        );
        std::fs::create_dir(tempdir.path().join("first")).unwrap();
        std::fs::create_dir(tempdir.path().join("second")).unwrap();
        std::fs::create_dir(tempdir.path().join("updated")).unwrap();
        std::fs::create_dir(&target).unwrap();
        let mut updated = model.read(&app, |model, _| {
            model
                .repository(updated_id)
                .expect("updated repository should exist")
                .clone()
        });
        updated.path = target.clone();

        let error = model
            .update(&mut app, |model, ctx| model.update_repository(updated, ctx))
            .unwrap_err();

        assert!(matches!(
            error,
            ProjectOrganizationError::AmbiguousRepositoryPath {
                canonical_path,
                repository_ids,
            } if canonical_path == dunce::canonicalize(&target).unwrap()
                && repository_ids == vec![first_id, second_id]
        ));
    });
}

#[test]
fn add_local_repository_commits_memory_after_persistence_succeeds() {
    App::test((), |mut app| async move {
        let tempdir = TempDir::new().unwrap();
        let repository_path = tempdir.path().join("repository");
        std::fs::create_dir(&repository_path).unwrap();
        let canonical_path = dunce::canonicalize(&repository_path).unwrap();
        let phantom_id = RepositoryId::from(Uuid::from_u128(1));
        let (model, operations) = create_acknowledged_model(&mut app, vec![], vec![]);
        let (emitted_events, _event_probe) = capture_project_organization_events(&mut app, &model);
        model.update(&mut app, |model, _| {
            model
                .repository_ids_by_path
                .insert(canonical_path.clone(), phantom_id);
        });

        let result = model.update(&mut app, |model, ctx| {
            model.add_local_repository(&repository_path, ctx)
        });
        let operations = operations.operations.try_iter().collect::<Vec<_>>();

        assert_eq!(operations.len(), 1);
        let repository_id = result.expect("memory commit should be infallible after persistence");
        assert!(matches!(
            &operations[0],
            RepositoryPersistenceOperation::UpsertRepository { repository }
                if repository.id == repository_id.to_string()
        ));
        assert!(model.read(&app, |model, _| model.repository(repository_id).is_some()));
        assert_eq!(
            *emitted_events.lock().unwrap(),
            vec![ProjectOrganizationEvent::RepositoryAdded { repository_id }]
        );
    });
}

#[test]
fn adding_repository_with_initial_workspace_commits_both_rows_after_persistence() {
    App::test((), |mut app| async move {
        let tempdir = TempDir::new().unwrap();
        let repository_path = tempdir.path().join("repository");
        std::fs::create_dir(&repository_path).unwrap();
        let canonical_path = dunce::canonicalize(&repository_path).unwrap();
        let (model, operations) = create_acknowledged_model(&mut app, vec![], vec![]);
        let (emitted_events, _event_probe) = capture_project_organization_events(&mut app, &model);

        let (repository_id, workspace_id) = model
            .update(&mut app, |model, ctx| {
                model.add_local_repository_with_initial_workspace(
                    &repository_path,
                    None,
                    "main",
                    ctx,
                )
            })
            .expect("repository and local workspace should be added");
        let operations = operations.operations.try_iter().collect::<Vec<_>>();

        assert_eq!(operations.len(), 1);
        assert!(matches!(
            &operations[0],
            RepositoryPersistenceOperation::UpsertRepositoryWithWorkspace {
                repository,
                workspace,
            } if repository.id == repository_id.to_string()
                && workspace.id == workspace_id.to_string()
                && workspace.display_name == "local"
                && workspace.branch == "main"
                && workspace.worktree_path == canonical_path.to_string_lossy()
        ));
        model.read(&app, |model, _| {
            let repository = model
                .repository(repository_id)
                .expect("repository should exist");
            assert_eq!(repository.path, canonical_path);
            let workspace = model
                .workspace(workspace_id)
                .expect("workspace should exist");
            assert_eq!(workspace.display_name, "local");
            assert_eq!(workspace.branch, "main");
            assert_eq!(workspace.worktree_path, canonical_path);
        });
        assert_eq!(
            *emitted_events.lock().unwrap(),
            vec![
                ProjectOrganizationEvent::RepositoryAdded { repository_id },
                ProjectOrganizationEvent::WorkspaceAdded { workspace_id },
            ]
        );
    });
}

#[test]
fn repository_with_initial_workspace_does_not_change_memory_when_persistence_fails() {
    App::test((), |mut app| async move {
        let tempdir = TempDir::new().unwrap();
        let repository_path = tempdir.path().join("repository");
        std::fs::create_dir(&repository_path).unwrap();
        let canonical_path = dunce::canonicalize(&repository_path).unwrap();
        let (persistence, harness) =
            acknowledged_persistence(Err(RepositoryPersistenceError::Database {
                details: "injected failure".to_string(),
            }));
        let model = create_model(&mut app, vec![], vec![], persistence);
        let (emitted_events, _event_probe) = capture_project_organization_events(&mut app, &model);

        let error = model
            .update(&mut app, |model, ctx| {
                model.add_local_repository_with_initial_workspace(
                    &repository_path,
                    None,
                    "main",
                    ctx,
                )
            })
            .expect_err("persistence failure should be returned");

        assert_persistence_failure(error, "repository addition with initial workspace");
        model.read(&app, |model, _| {
            assert_eq!(model.repositories().count(), 0);
            assert_eq!(model.workspaces().count(), 0);
            assert!(!model.repository_ids_by_path.contains_key(&canonical_path));
        });
        assert!(emitted_events.lock().unwrap().is_empty());
        let operations = harness.operations.try_iter().collect::<Vec<_>>();
        assert_eq!(operations.len(), 1);
        assert!(matches!(
            &operations[0],
            RepositoryPersistenceOperation::UpsertRepositoryWithWorkspace {
                repository,
                workspace,
            } if repository.path == canonical_path.to_string_lossy()
                && workspace.worktree_path == canonical_path.to_string_lossy()
        ));
    });
}

#[test]
fn insert_repository_commits_memory_after_persistence_succeeds() {
    App::test((), |mut app| async move {
        let tempdir = TempDir::new().unwrap();
        let repository_path = tempdir.path().join("repository");
        std::fs::create_dir(&repository_path).unwrap();
        let canonical_path = dunce::canonicalize(&repository_path).unwrap();
        let repository_id = RepositoryId::from(Uuid::from_u128(1));
        let phantom_id = RepositoryId::from(Uuid::from_u128(2));
        let (model, operations) = create_acknowledged_model(&mut app, vec![], vec![]);
        model.update(&mut app, |model, _| {
            model
                .repository_ids_by_path
                .insert(canonical_path, phantom_id);
        });

        let result = model.update(&mut app, |model, ctx| {
            model.insert_repository(repository(repository_id, &repository_path), ctx)
        });
        let operations = operations.operations.try_iter().collect::<Vec<_>>();

        assert_eq!(operations.len(), 1);
        result.expect("memory commit should be infallible after persistence");
        assert!(matches!(
            &operations[0],
            RepositoryPersistenceOperation::UpsertRepository { repository }
                if repository.id == repository_id.to_string()
        ));
        assert!(model.read(&app, |model, _| model.repository(repository_id).is_some()));
    });
}

#[test]
fn repository_add_does_not_change_memory_when_persistence_fails() {
    App::test((), |mut app| async move {
        let tempdir = TempDir::new().unwrap();
        let repository_path = tempdir.path().join("repository");
        std::fs::create_dir(&repository_path).unwrap();
        let canonical_path = dunce::canonicalize(&repository_path).unwrap();
        let (persistence, harness) =
            acknowledged_persistence(Err(RepositoryPersistenceError::Database {
                details: "injected failure".to_string(),
            }));
        let model = create_model(&mut app, vec![], vec![], persistence);
        let (emitted_events, _event_probe) = capture_project_organization_events(&mut app, &model);

        let error = model
            .update(&mut app, |model, ctx| {
                model.add_local_repository(&repository_path, ctx)
            })
            .unwrap_err();

        assert_persistence_failure(error, "repository addition");
        model.read(&app, |model, _| {
            assert_eq!(model.repositories().count(), 0);
            assert!(!model.repository_ids_by_path.contains_key(&canonical_path));
        });
        assert!(emitted_events.lock().unwrap().is_empty());
        let operations = harness.operations.try_iter().collect::<Vec<_>>();
        assert_eq!(operations.len(), 1);
        assert!(matches!(
            &operations[0],
            RepositoryPersistenceOperation::UpsertRepository { repository }
                if repository.path == canonical_path.to_string_lossy()
        ));
    });
}

#[test]
fn repository_update_does_not_change_indexes_when_persistence_fails() {
    App::test((), |mut app| async move {
        let tempdir = TempDir::new().unwrap();
        let old_path = tempdir.path().join("old-repository");
        let new_path = tempdir.path().join("new-repository");
        std::fs::create_dir(&old_path).unwrap();
        std::fs::create_dir(&new_path).unwrap();
        let canonical_old_path = dunce::canonicalize(&old_path).unwrap();
        let canonical_new_path = dunce::canonicalize(&new_path).unwrap();
        let repository_id = RepositoryId::from(Uuid::new_v4());
        let persisted_repository = persisted_repository(repository_id, &old_path);
        let (persistence, harness) =
            acknowledged_persistence(Err(RepositoryPersistenceError::Database {
                details: "injected failure".to_string(),
            }));
        let model = create_model(&mut app, vec![persisted_repository], vec![], persistence);
        let (emitted_events, _event_probe) = capture_project_organization_events(&mut app, &model);
        let mut updated_repository = model.read(&app, |model, _| {
            model.repository(repository_id).unwrap().clone()
        });
        updated_repository.display_name = "updated".to_string();
        updated_repository.path = new_path;

        let error = model
            .update(&mut app, |model, ctx| {
                model.update_repository(updated_repository, ctx)
            })
            .unwrap_err();

        assert_persistence_failure(error, "repository update");
        model.read(&app, |model, _| {
            let repository = model.repository(repository_id).unwrap();
            assert_eq!(repository.display_name, "repository");
            assert_eq!(repository.path, canonical_old_path);
            assert_eq!(
                model.repository_ids_by_path.get(&canonical_old_path),
                Some(&repository_id)
            );
            assert!(!model
                .repository_ids_by_path
                .contains_key(&canonical_new_path));
        });
        assert!(emitted_events.lock().unwrap().is_empty());
        let operations = harness.operations.try_iter().collect::<Vec<_>>();
        assert_eq!(operations.len(), 1);
        assert!(matches!(
            &operations[0],
            RepositoryPersistenceOperation::UpsertRepository { repository }
                if repository.path == canonical_new_path.to_string_lossy()
        ));
    });
}

#[test]
fn repository_delete_does_not_change_indexes_when_persistence_fails() {
    App::test((), |mut app| async move {
        let tempdir = TempDir::new().unwrap();
        let repository_path = tempdir.path().join("repository");
        std::fs::create_dir(&repository_path).unwrap();
        let canonical_path = dunce::canonicalize(&repository_path).unwrap();
        let repository_id = RepositoryId::from(Uuid::new_v4());
        let (persistence, harness) =
            acknowledged_persistence(Err(RepositoryPersistenceError::Database {
                details: "injected failure".to_string(),
            }));
        let model = create_model(
            &mut app,
            vec![persisted_repository(repository_id, &repository_path)],
            vec![],
            persistence,
        );
        let (emitted_events, _event_probe) = capture_project_organization_events(&mut app, &model);

        let error = model
            .update(&mut app, |model, ctx| {
                model.remove_repository(repository_id, ctx)
            })
            .unwrap_err();

        assert_persistence_failure(error, "repository removal");
        model.read(&app, |model, _| {
            assert!(model.repository(repository_id).is_some());
            assert_eq!(
                model.repository_ids_by_path.get(&canonical_path),
                Some(&repository_id)
            );
        });
        assert!(emitted_events.lock().unwrap().is_empty());
        let operations = harness.operations.try_iter().collect::<Vec<_>>();
        assert_eq!(operations.len(), 1);
        assert!(matches!(
            &operations[0],
            RepositoryPersistenceOperation::DeleteRepository { repository_id: id }
                if id == &repository_id.to_string()
        ));
    });
}

#[test]
fn workspace_insert_does_not_change_memory_when_persistence_fails() {
    App::test((), |mut app| async move {
        let tempdir = TempDir::new().unwrap();
        let repository_path = tempdir.path().join("repository");
        let worktree_path = tempdir.path().join("worktree");
        std::fs::create_dir(&repository_path).unwrap();
        std::fs::create_dir(&worktree_path).unwrap();
        let canonical_worktree_path = dunce::canonicalize(&worktree_path).unwrap();
        let repository_id = RepositoryId::from(Uuid::new_v4());
        let workspace_id = RepositoryWorkspaceId::from(Uuid::new_v4());
        let (persistence, harness) =
            acknowledged_persistence(Err(RepositoryPersistenceError::Database {
                details: "injected failure".to_string(),
            }));
        let model = create_model(
            &mut app,
            vec![persisted_repository(repository_id, &repository_path)],
            vec![],
            persistence,
        );
        let (emitted_events, _event_probe) = capture_project_organization_events(&mut app, &model);

        let error = model
            .update(&mut app, |model, ctx| {
                model.insert_workspace(
                    repository_workspace(
                        workspace_id,
                        repository_id,
                        "feature/test",
                        &worktree_path,
                    ),
                    ctx,
                )
            })
            .unwrap_err();

        assert_persistence_failure(error, "repository workspace insertion");
        model.read(&app, |model, _| {
            assert!(model.workspace(workspace_id).is_none());
            assert!(!model
                .workspace_ids_by_repository_branch
                .contains_key(&(repository_id, "feature/test".to_string())));
            assert!(!model
                .workspace_ids_by_path
                .contains_key(&canonical_worktree_path));
        });
        assert!(emitted_events.lock().unwrap().is_empty());
        let operations = harness.operations.try_iter().collect::<Vec<_>>();
        assert_eq!(operations.len(), 1);
        assert!(matches!(
            &operations[0],
            RepositoryPersistenceOperation::UpsertRepositoryWorkspace { workspace }
                if workspace.id == workspace_id.to_string()
        ));
    });
}

#[test]
fn workspace_update_does_not_change_indexes_when_persistence_fails() {
    App::test((), |mut app| async move {
        let tempdir = TempDir::new().unwrap();
        let repository_path = tempdir.path().join("repository");
        let old_path = tempdir.path().join("old-worktree");
        let new_path = tempdir.path().join("new-worktree");
        for path in [&repository_path, &old_path, &new_path] {
            std::fs::create_dir(path).unwrap();
        }
        let canonical_old_path = dunce::canonicalize(&old_path).unwrap();
        let canonical_new_path = dunce::canonicalize(&new_path).unwrap();
        let repository_id = RepositoryId::from(Uuid::new_v4());
        let workspace_id = RepositoryWorkspaceId::from(Uuid::new_v4());
        let (persistence, harness) =
            acknowledged_persistence(Err(RepositoryPersistenceError::Database {
                details: "injected failure".to_string(),
            }));
        let model = create_model(
            &mut app,
            vec![persisted_repository(repository_id, &repository_path)],
            vec![persisted_workspace(
                workspace_id,
                repository_id,
                "feature/old",
                &old_path,
            )],
            persistence,
        );
        let (emitted_events, _event_probe) = capture_project_organization_events(&mut app, &model);
        let mut updated_workspace = model.read(&app, |model, _| {
            model.workspace(workspace_id).unwrap().clone()
        });
        updated_workspace.display_name = "updated".to_string();
        updated_workspace.branch = "feature/new".to_string();
        updated_workspace.worktree_path = new_path;

        let error = model
            .update(&mut app, |model, ctx| {
                model.update_workspace(updated_workspace, ctx)
            })
            .unwrap_err();

        assert_persistence_failure(error, "repository workspace update");
        model.read(&app, |model, _| {
            let workspace = model.workspace(workspace_id).unwrap();
            assert_eq!(workspace.display_name, "feature/old");
            assert_eq!(workspace.branch, "feature/old");
            assert_eq!(workspace.worktree_path, canonical_old_path);
            assert_eq!(
                model
                    .workspace_ids_by_repository_branch
                    .get(&(repository_id, "feature/old".to_string())),
                Some(&workspace_id)
            );
            assert!(!model
                .workspace_ids_by_repository_branch
                .contains_key(&(repository_id, "feature/new".to_string())));
            assert_eq!(
                model.workspace_ids_by_path.get(&canonical_old_path),
                Some(&workspace_id)
            );
            assert!(!model
                .workspace_ids_by_path
                .contains_key(&canonical_new_path));
        });
        assert!(emitted_events.lock().unwrap().is_empty());
        let operations = harness.operations.try_iter().collect::<Vec<_>>();
        assert_eq!(operations.len(), 1);
        assert!(matches!(
            &operations[0],
            RepositoryPersistenceOperation::UpsertRepositoryWorkspace { workspace }
                if workspace.branch == "feature/new"
                    && workspace.worktree_path == canonical_new_path.to_string_lossy()
        ));
    });
}

#[test]
fn workspace_delete_does_not_change_indexes_when_persistence_fails() {
    App::test((), |mut app| async move {
        let tempdir = TempDir::new().unwrap();
        let repository_path = tempdir.path().join("repository");
        let worktree_path = tempdir.path().join("worktree");
        std::fs::create_dir(&repository_path).unwrap();
        std::fs::create_dir(&worktree_path).unwrap();
        let canonical_worktree_path = dunce::canonicalize(&worktree_path).unwrap();
        let repository_id = RepositoryId::from(Uuid::new_v4());
        let workspace_id = RepositoryWorkspaceId::from(Uuid::new_v4());
        let (persistence, harness) =
            acknowledged_persistence(Err(RepositoryPersistenceError::Database {
                details: "injected failure".to_string(),
            }));
        let model = create_model(
            &mut app,
            vec![persisted_repository(repository_id, &repository_path)],
            vec![persisted_workspace(
                workspace_id,
                repository_id,
                "feature/test",
                &worktree_path,
            )],
            persistence,
        );
        let (emitted_events, _event_probe) = capture_project_organization_events(&mut app, &model);

        let error = model
            .update(&mut app, |model, ctx| {
                model.remove_workspace(workspace_id, ctx)
            })
            .unwrap_err();

        assert_persistence_failure(error, "repository workspace removal");
        model.read(&app, |model, _| {
            assert!(model.workspace(workspace_id).is_some());
            assert_eq!(
                model
                    .workspace_ids_by_repository_branch
                    .get(&(repository_id, "feature/test".to_string())),
                Some(&workspace_id)
            );
            assert_eq!(
                model.workspace_ids_by_path.get(&canonical_worktree_path),
                Some(&workspace_id)
            );
        });
        assert!(emitted_events.lock().unwrap().is_empty());
        let operations = harness.operations.try_iter().collect::<Vec<_>>();
        assert_eq!(operations.len(), 1);
        assert!(matches!(
            &operations[0],
            RepositoryPersistenceOperation::DeleteRepositoryWorkspace { workspace_id: id }
                if id == &workspace_id.to_string()
        ));
    });
}

#[test]
fn unavailable_persistence_does_not_change_memory() {
    App::test((), |mut app| async move {
        let tempdir = TempDir::new().unwrap();
        let repository_path = tempdir.path().join("repository");
        std::fs::create_dir(&repository_path).unwrap();
        let canonical_path = dunce::canonicalize(&repository_path).unwrap();
        let model = create_model(&mut app, vec![], vec![], RepositoryPersistence::new(None));
        let (emitted_events, _event_probe) = capture_project_organization_events(&mut app, &model);

        let error = model
            .update(&mut app, |model, ctx| {
                model.add_local_repository(&repository_path, ctx)
            })
            .unwrap_err();

        match error {
            ProjectOrganizationError::Persistence { operation, details } => {
                assert_eq!(operation, "repository addition");
                assert!(details.contains("unavailable"));
            }
            error => panic!("expected persistence error, got {error:?}"),
        }
        model.read(&app, |model, _| {
            assert_eq!(model.repositories().count(), 0);
            assert!(!model.repository_ids_by_path.contains_key(&canonical_path));
        });
        assert!(emitted_events.lock().unwrap().is_empty());
    });
}

#[test]
fn insert_workspace_rejects_duplicate_repository_branch() {
    App::test((), |mut app| async move {
        let tempdir = TempDir::new().expect("temporary directory should be created");
        let repository_path = tempdir.path().join("repository");
        let first_worktree = tempdir.path().join("first-worktree");
        let second_worktree = tempdir.path().join("second-worktree");
        for path in [&repository_path, &first_worktree, &second_worktree] {
            std::fs::create_dir(path).expect("test directory should be created");
        }
        let (model, _events) = create_acknowledged_model(&mut app, vec![], vec![]);
        let repository_id = model
            .update(&mut app, |model, ctx| {
                model.add_local_repository(&repository_path, ctx)
            })
            .expect("repository should be added");
        let first_workspace_id = RepositoryWorkspaceId::from(Uuid::new_v4());
        model
            .update(&mut app, |model, ctx| {
                model.insert_workspace(
                    repository_workspace(
                        first_workspace_id,
                        repository_id,
                        "feature/branch",
                        &first_worktree,
                    ),
                    ctx,
                )
            })
            .expect("first workspace should be inserted");

        let error = model
            .update(&mut app, |model, ctx| {
                model.insert_workspace(
                    repository_workspace(
                        RepositoryWorkspaceId::from(Uuid::new_v4()),
                        repository_id,
                        "feature/branch",
                        &second_worktree,
                    ),
                    ctx,
                )
            })
            .expect_err("duplicate branch should be rejected");

        assert!(matches!(
            error,
            ProjectOrganizationError::WorkspaceBranchAlreadyExists {
                repository_id: duplicate_repository_id,
                branch,
                existing_workspace_id,
            } if duplicate_repository_id == repository_id
                && branch == "feature/branch"
                && existing_workspace_id == first_workspace_id
        ));
    });
}

#[test]
fn remove_repository_is_blocked_while_workspace_exists() {
    App::test((), |mut app| async move {
        let tempdir = TempDir::new().expect("temporary directory should be created");
        let repository_path = tempdir.path().join("repository");
        let worktree_path = tempdir.path().join("worktree");
        std::fs::create_dir(&repository_path).expect("repository directory should be created");
        std::fs::create_dir(&worktree_path).expect("worktree directory should be created");
        let (model, _events) = create_acknowledged_model(&mut app, vec![], vec![]);
        let repository_id = model
            .update(&mut app, |model, ctx| {
                model.add_local_repository(&repository_path, ctx)
            })
            .expect("repository should be added");
        let workspace_id = RepositoryWorkspaceId::from(Uuid::new_v4());
        model
            .update(&mut app, |model, ctx| {
                model.insert_workspace(
                    repository_workspace(workspace_id, repository_id, "main", &worktree_path),
                    ctx,
                )
            })
            .expect("workspace should be inserted");

        let error = model
            .update(&mut app, |model, ctx| {
                model.remove_repository(repository_id, ctx)
            })
            .expect_err("repository with workspaces should not be removed");

        assert!(matches!(
            error,
            ProjectOrganizationError::RepositoryHasWorkspaces {
                repository_id: blocked_repository_id,
            } if blocked_repository_id == repository_id
        ));
    });
}

#[test]
fn remove_workspace_allows_reusing_branch_and_worktree_path() {
    App::test((), |mut app| async move {
        let tempdir = TempDir::new().expect("temporary directory should be created");
        let repository_path = tempdir.path().join("repository");
        let worktree_path = tempdir.path().join("worktree");
        for path in [&repository_path, &worktree_path] {
            std::fs::create_dir(path).expect("test directory should be created");
        }
        let (model, _events) = create_acknowledged_model(&mut app, vec![], vec![]);
        let repository_id = model
            .update(&mut app, |model, ctx| {
                model.add_local_repository(&repository_path, ctx)
            })
            .expect("repository should be added");
        let first_workspace_id = RepositoryWorkspaceId::from(Uuid::new_v4());
        model
            .update(&mut app, |model, ctx| {
                model.insert_workspace(
                    repository_workspace(
                        first_workspace_id,
                        repository_id,
                        "feature/reusable",
                        &worktree_path,
                    ),
                    ctx,
                )
            })
            .expect("workspace should be inserted");

        model
            .update(&mut app, |model, ctx| {
                model.remove_workspace(first_workspace_id, ctx)
            })
            .expect("workspace should be removed");
        let replacement_id = RepositoryWorkspaceId::from(Uuid::new_v4());
        model
            .update(&mut app, |model, ctx| {
                model.insert_workspace(
                    repository_workspace(
                        replacement_id,
                        repository_id,
                        "feature/reusable",
                        &worktree_path,
                    ),
                    ctx,
                )
            })
            .expect("removed workspace indexes should be reusable");

        assert!(model.read(&app, |model, _| model.workspace(replacement_id).is_some()));
    });
}

#[test]
fn remove_repository_allows_reusing_path() {
    App::test((), |mut app| async move {
        let tempdir = TempDir::new().expect("temporary directory should be created");
        let repository_path = tempdir.path().join("repository");
        std::fs::create_dir(&repository_path).expect("repository directory should be created");
        let (model, _events) = create_acknowledged_model(&mut app, vec![], vec![]);
        let first_repository_id = model
            .update(&mut app, |model, ctx| {
                model.add_local_repository(&repository_path, ctx)
            })
            .expect("repository should be added");

        model
            .update(&mut app, |model, ctx| {
                model.remove_repository(first_repository_id, ctx)
            })
            .expect("repository should be removed");
        let replacement_id = model
            .update(&mut app, |model, ctx| {
                model.add_local_repository(&repository_path, ctx)
            })
            .expect("removed repository path should be reusable");

        assert_ne!(replacement_id, first_repository_id);
    });
}

#[test]
fn update_repository_replaces_path_index_and_rejects_new_duplicate() {
    App::test((), |mut app| async move {
        let tempdir = TempDir::new().expect("temporary directory should be created");
        let old_path = tempdir.path().join("old-repository");
        let new_path = tempdir.path().join("new-repository");
        for path in [&old_path, &new_path] {
            std::fs::create_dir(path).expect("repository directory should be created");
        }
        let canonical_new_path =
            dunce::canonicalize(&new_path).expect("new repository path should canonicalize");
        let (model, _events) = create_acknowledged_model(&mut app, vec![], vec![]);
        let repository_id = model
            .update(&mut app, |model, ctx| {
                model.add_local_repository(&old_path, ctx)
            })
            .expect("repository should be added");
        let mut repository = model.read(&app, |model, _| {
            model
                .repository(repository_id)
                .expect("repository should exist")
                .clone()
        });
        repository.path = new_path.clone();

        model
            .update(&mut app, |model, ctx| {
                model.update_repository(repository, ctx)
            })
            .expect("repository path should be updated");
        model
            .update(&mut app, |model, ctx| {
                model.add_local_repository(&old_path, ctx)
            })
            .expect("old repository path index should be removed");
        let error = model
            .update(&mut app, |model, ctx| {
                model.add_local_repository(&new_path, ctx)
            })
            .expect_err("new repository path should remain indexed");

        assert!(matches!(
            error,
            ProjectOrganizationError::RepositoryAlreadyExists {
                existing_repository_id,
                canonical_path,
            } if existing_repository_id == repository_id && canonical_path == canonical_new_path
        ));
    });
}

#[test]
fn update_workspace_replaces_branch_and_path_indexes_and_rejects_new_duplicates() {
    App::test((), |mut app| async move {
        let tempdir = TempDir::new().expect("temporary directory should be created");
        let repository_path = tempdir.path().join("repository");
        let old_path = tempdir.path().join("old-worktree");
        let new_path = tempdir.path().join("new-worktree");
        let branch_conflict_path = tempdir.path().join("branch-conflict-worktree");
        for path in [
            &repository_path,
            &old_path,
            &new_path,
            &branch_conflict_path,
        ] {
            std::fs::create_dir(path).expect("test directory should be created");
        }
        let canonical_new_path =
            dunce::canonicalize(&new_path).expect("new worktree path should canonicalize");
        let (model, _events) = create_acknowledged_model(&mut app, vec![], vec![]);
        let repository_id = model
            .update(&mut app, |model, ctx| {
                model.add_local_repository(&repository_path, ctx)
            })
            .expect("repository should be added");
        let workspace_id = RepositoryWorkspaceId::from(Uuid::new_v4());
        model
            .update(&mut app, |model, ctx| {
                model.insert_workspace(
                    repository_workspace(workspace_id, repository_id, "feature/old", &old_path),
                    ctx,
                )
            })
            .expect("workspace should be inserted");
        let mut workspace = model.read(&app, |model, _| {
            model
                .workspace(workspace_id)
                .expect("workspace should exist")
                .clone()
        });
        workspace.branch = "feature/new".to_string();
        workspace.worktree_path = new_path.clone();

        model
            .update(&mut app, |model, ctx| {
                model.update_workspace(workspace, ctx)
            })
            .expect("workspace branch and path should be updated");
        model
            .update(&mut app, |model, ctx| {
                model.insert_workspace(
                    repository_workspace(
                        RepositoryWorkspaceId::from(Uuid::new_v4()),
                        repository_id,
                        "feature/old",
                        &old_path,
                    ),
                    ctx,
                )
            })
            .expect("old workspace indexes should be removed");
        let branch_error = model
            .update(&mut app, |model, ctx| {
                model.insert_workspace(
                    repository_workspace(
                        RepositoryWorkspaceId::from(Uuid::new_v4()),
                        repository_id,
                        "feature/new",
                        &branch_conflict_path,
                    ),
                    ctx,
                )
            })
            .expect_err("new branch should remain indexed");
        let path_error = model
            .update(&mut app, |model, ctx| {
                model.insert_workspace(
                    repository_workspace(
                        RepositoryWorkspaceId::from(Uuid::new_v4()),
                        repository_id,
                        "feature/path-conflict",
                        &new_path,
                    ),
                    ctx,
                )
            })
            .expect_err("new worktree path should remain indexed");

        assert!(matches!(
            branch_error,
            ProjectOrganizationError::WorkspaceBranchAlreadyExists {
                repository_id: duplicate_repository_id,
                branch,
                existing_workspace_id,
            } if duplicate_repository_id == repository_id
                && branch == "feature/new"
                && existing_workspace_id == workspace_id
        ));
        assert!(matches!(
            path_error,
            ProjectOrganizationError::WorkspacePathAlreadyExists {
                existing_workspace_id,
                canonical_path,
            } if existing_workspace_id == workspace_id && canonical_path == canonical_new_path
        ));
    });
}

#[test]
fn insert_repository_rejects_duplicate_id() {
    App::test((), |mut app| async move {
        let tempdir = TempDir::new().expect("temporary directory should be created");
        let first_path = tempdir.path().join("first-repository");
        let second_path = tempdir.path().join("second-repository");
        for path in [&first_path, &second_path] {
            std::fs::create_dir(path).expect("repository directory should be created");
        }
        let (model, _events) = create_acknowledged_model(&mut app, vec![], vec![]);
        let repository_id = model
            .update(&mut app, |model, ctx| {
                model.add_local_repository(&first_path, ctx)
            })
            .expect("repository should be added");
        let mut duplicate = model.read(&app, |model, _| {
            model
                .repository(repository_id)
                .expect("repository should exist")
                .clone()
        });
        duplicate.path = second_path;

        let error = model
            .update(&mut app, |model, ctx| {
                model.insert_repository(duplicate, ctx)
            })
            .expect_err("duplicate repository ID should be rejected");

        assert!(matches!(
            error,
            ProjectOrganizationError::RepositoryIdAlreadyExists {
                repository_id: duplicate_id,
            } if duplicate_id == repository_id
        ));
    });
}

#[test]
fn insert_workspace_rejects_duplicate_id() {
    App::test((), |mut app| async move {
        let tempdir = TempDir::new().expect("temporary directory should be created");
        let repository_path = tempdir.path().join("repository");
        let first_path = tempdir.path().join("first-worktree");
        let second_path = tempdir.path().join("second-worktree");
        for path in [&repository_path, &first_path, &second_path] {
            std::fs::create_dir(path).expect("test directory should be created");
        }
        let (model, _events) = create_acknowledged_model(&mut app, vec![], vec![]);
        let repository_id = model
            .update(&mut app, |model, ctx| {
                model.add_local_repository(&repository_path, ctx)
            })
            .expect("repository should be added");
        let workspace_id = RepositoryWorkspaceId::from(Uuid::new_v4());
        model
            .update(&mut app, |model, ctx| {
                model.insert_workspace(
                    repository_workspace(workspace_id, repository_id, "feature/first", &first_path),
                    ctx,
                )
            })
            .expect("workspace should be inserted");

        let error = model
            .update(&mut app, |model, ctx| {
                model.insert_workspace(
                    repository_workspace(
                        workspace_id,
                        repository_id,
                        "feature/second",
                        &second_path,
                    ),
                    ctx,
                )
            })
            .expect_err("duplicate workspace ID should be rejected");

        assert!(matches!(
            error,
            ProjectOrganizationError::WorkspaceIdAlreadyExists {
                workspace_id: duplicate_id,
            } if duplicate_id == workspace_id
        ));
    });
}

#[test]
fn insert_workspace_rejects_orphan_repository() {
    App::test((), |mut app| async move {
        let tempdir = TempDir::new().expect("temporary directory should be created");
        let worktree_path = tempdir.path().join("worktree");
        std::fs::create_dir(&worktree_path).expect("worktree directory should be created");
        let repository_id = RepositoryId::from(Uuid::new_v4());
        let (model, _events) = create_acknowledged_model(&mut app, vec![], vec![]);

        let error = model
            .update(&mut app, |model, ctx| {
                model.insert_workspace(
                    repository_workspace(
                        RepositoryWorkspaceId::from(Uuid::new_v4()),
                        repository_id,
                        "feature/orphan",
                        &worktree_path,
                    ),
                    ctx,
                )
            })
            .expect_err("orphan workspace should be rejected");

        assert!(matches!(
            error,
            ProjectOrganizationError::RepositoryNotFound {
                repository_id: missing_repository_id,
            } if missing_repository_id == repository_id
        ));
    });
}

#[test]
fn insert_workspace_commits_memory_after_persistence_succeeds() {
    App::test((), |mut app| async move {
        let tempdir = TempDir::new().unwrap();
        let repository_path = tempdir.path().join("repository");
        let worktree_path = tempdir.path().join("worktree");
        std::fs::create_dir(&repository_path).unwrap();
        std::fs::create_dir(&worktree_path).unwrap();
        let canonical_worktree_path = dunce::canonicalize(&worktree_path).unwrap();
        let workspace_id = RepositoryWorkspaceId::from(Uuid::from_u128(1));
        let phantom_id = RepositoryWorkspaceId::from(Uuid::from_u128(2));
        let (model, operations) = create_acknowledged_model(&mut app, vec![], vec![]);
        let repository_id = model
            .update(&mut app, |model, ctx| {
                model.add_local_repository(&repository_path, ctx)
            })
            .unwrap();
        operations.operations.recv().unwrap();
        model.update(&mut app, |model, _| {
            model
                .workspace_ids_by_path
                .insert(canonical_worktree_path, phantom_id);
        });

        let result = model.update(&mut app, |model, ctx| {
            model.insert_workspace(
                repository_workspace(workspace_id, repository_id, "feature/test", &worktree_path),
                ctx,
            )
        });
        let operations = operations.operations.try_iter().collect::<Vec<_>>();

        assert_eq!(operations.len(), 1);
        result.expect("memory commit should be infallible after persistence");
        assert!(matches!(
            &operations[0],
            RepositoryPersistenceOperation::UpsertRepositoryWorkspace { workspace }
                if workspace.id == workspace_id.to_string()
        ));
        assert!(model.read(&app, |model, _| model.workspace(workspace_id).is_some()));
    });
}

#[test]
fn rename_repository_changes_only_display_name() {
    App::test((), |mut app| async move {
        let tempdir = TempDir::new().expect("temporary directory should be created");
        let repository_path = tempdir.path().join("repository");
        std::fs::create_dir(&repository_path).expect("repository directory should be created");
        let canonical_path =
            dunce::canonicalize(&repository_path).expect("repository path should canonicalize");
        let (model, _events) = create_acknowledged_model(&mut app, vec![], vec![]);
        let repository_id = model
            .update(&mut app, |model, ctx| {
                model.add_local_repository(&repository_path, ctx)
            })
            .expect("repository should be added");

        model
            .update(&mut app, |model, ctx| {
                model.rename_repository(repository_id, "Renamed repository".to_string(), ctx)
            })
            .expect("repository should be renamed");
        let repository = model.read(&app, |model, _| {
            model
                .repository(repository_id)
                .expect("repository should exist")
                .clone()
        });

        assert_eq!(repository.display_name, "Renamed repository");
        assert_eq!(repository.path, canonical_path);
        assert_eq!(repository.source, RepositorySource::Local);
    });
}

#[test]
fn rename_workspace_changes_only_display_name() {
    App::test((), |mut app| async move {
        let tempdir = TempDir::new().expect("temporary directory should be created");
        let repository_path = tempdir.path().join("repository");
        let worktree_path = tempdir.path().join("worktree");
        std::fs::create_dir(&repository_path).expect("repository directory should be created");
        std::fs::create_dir(&worktree_path).expect("worktree directory should be created");
        let canonical_worktree_path =
            dunce::canonicalize(&worktree_path).expect("worktree path should canonicalize");
        let (model, _events) = create_acknowledged_model(&mut app, vec![], vec![]);
        let repository_id = model
            .update(&mut app, |model, ctx| {
                model.add_local_repository(&repository_path, ctx)
            })
            .expect("repository should be added");
        let workspace_id = RepositoryWorkspaceId::from(Uuid::new_v4());
        model
            .update(&mut app, |model, ctx| {
                model.insert_workspace(
                    repository_workspace(
                        workspace_id,
                        repository_id,
                        "feature/branch",
                        &worktree_path,
                    ),
                    ctx,
                )
            })
            .expect("workspace should be inserted");

        model
            .update(&mut app, |model, ctx| {
                model.rename_workspace(workspace_id, "Renamed workspace".to_string(), ctx)
            })
            .expect("workspace should be renamed");
        let workspace = model.read(&app, |model, _| {
            model
                .workspace(workspace_id)
                .expect("workspace should exist")
                .clone()
        });

        assert_eq!(workspace.display_name, "Renamed workspace");
        assert_eq!(workspace.branch, "feature/branch");
        assert_eq!(workspace.worktree_path, canonical_worktree_path);
    });
}

#[test]
fn touch_repository_path_adds_repository_and_persistence_event() {
    App::test((), |mut app| async move {
        let tempdir = TempDir::new().expect("temporary directory should be created");
        let repository_path = tempdir.path().join("repository");
        std::fs::create_dir(&repository_path).expect("repository directory should be created");
        let canonical_path =
            dunce::canonicalize(&repository_path).expect("repository path should canonicalize");
        let (model, operations) = create_acknowledged_model(&mut app, vec![], vec![]);

        let repository_id = model
            .update(&mut app, |model, ctx| {
                model.touch_repository_path(&repository_path, ctx)
            })
            .expect("repository path should be touched");
        let operation = operations
            .operations
            .recv()
            .expect("persistence operation should be sent");

        assert!(matches!(
            operation,
            RepositoryPersistenceOperation::UpsertRepository { repository }
                if repository.id == repository_id.to_string()
                    && repository.path == canonical_path.to_string_lossy()
        ));
    });
}

#[test]
fn touch_repository_path_updates_existing_timestamp_and_persistence_event() {
    App::test((), |mut app| async move {
        let tempdir = TempDir::new().expect("temporary directory should be created");
        let repository_path = tempdir.path().join("repository");
        std::fs::create_dir(&repository_path).expect("repository directory should be created");
        let repository_id = RepositoryId::from(Uuid::new_v4());
        let persisted_repository = persisted_repository(repository_id, &repository_path);
        let previous_last_opened_at = persisted_repository.last_opened_at;
        let (model, operations) =
            create_acknowledged_model(&mut app, vec![persisted_repository], vec![]);

        let touched_id = model
            .update(&mut app, |model, ctx| {
                model.touch_repository_path(&repository_path, ctx)
            })
            .expect("repository path should be touched");
        let operation = operations
            .operations
            .recv()
            .expect("persistence operation should be sent");

        assert_eq!(touched_id, repository_id);
        assert!(matches!(
            operation,
            RepositoryPersistenceOperation::UpsertRepository { repository }
                if repository.id == repository_id.to_string()
                    && repository.last_opened_at > previous_last_opened_at
        ));
    });
}

#[test]
fn touch_repository_rejects_ambiguous_recovered_aliases() {
    App::test((), |mut app| async move {
        let tempdir = TempDir::new().unwrap();
        let target = tempdir.path().join("repository");
        let first_alias = tempdir.path().join("first").join("..").join("repository");
        let second_alias = tempdir.path().join("second").join("..").join("repository");
        let first_id = RepositoryId::from(Uuid::from_u128(1));
        let second_id = RepositoryId::from(Uuid::from_u128(2));
        let (model, _operations) = create_acknowledged_model(
            &mut app,
            vec![
                persisted_repository(first_id, &first_alias),
                persisted_repository(second_id, &second_alias),
            ],
            vec![],
        );
        std::fs::create_dir(tempdir.path().join("first")).unwrap();
        std::fs::create_dir(tempdir.path().join("second")).unwrap();
        std::fs::create_dir(&target).unwrap();

        let error = model
            .update(&mut app, |model, ctx| {
                model.touch_repository_path(&target, ctx)
            })
            .unwrap_err();

        assert!(matches!(
            error,
            ProjectOrganizationError::AmbiguousRepositoryPath {
                canonical_path,
                repository_ids,
            } if canonical_path == dunce::canonicalize(&target).unwrap()
                && repository_ids == vec![first_id, second_id]
        ));
    });
}

#[test]
fn touch_repository_migrates_unique_recovered_alias_to_canonical_path() {
    App::test((), |mut app| async move {
        let tempdir = TempDir::new().unwrap();
        let target = tempdir.path().join("repository");
        let alias = tempdir.path().join("alias").join("..").join("repository");
        let repository_id = RepositoryId::from(Uuid::from_u128(1));
        let (model, operations) = create_acknowledged_model(
            &mut app,
            vec![persisted_repository(repository_id, &alias)],
            vec![],
        );
        std::fs::create_dir(tempdir.path().join("alias")).unwrap();
        std::fs::create_dir(&target).unwrap();
        let canonical_path = dunce::canonicalize(&target).unwrap();

        let touched_id = model
            .update(&mut app, |model, ctx| {
                model.touch_repository_path(&target, ctx)
            })
            .unwrap();
        let duplicate_error = model
            .update(&mut app, |model, ctx| {
                model.add_local_repository(&canonical_path, ctx)
            })
            .unwrap_err();
        let stored_path = model.read(&app, |model, _| {
            model
                .repository(repository_id)
                .expect("repository should exist")
                .path
                .clone()
        });

        assert_eq!(touched_id, repository_id);
        assert_eq!(stored_path, canonical_path);
        assert_eq!(model.read(&app, |model, _| model.repositories().count()), 1);
        assert_eq!(operations.operations.try_iter().count(), 1);
        assert!(matches!(
            duplicate_error,
            ProjectOrganizationError::RepositoryAlreadyExists {
                existing_repository_id,
                canonical_path: duplicate_path,
            } if existing_repository_id == repository_id && duplicate_path == canonical_path
        ));
    });
}

#[test]
fn insert_workspace_rejects_ambiguous_recovered_aliases() {
    App::test((), |mut app| async move {
        let tempdir = TempDir::new().unwrap();
        let repository_path = tempdir.path().join("repository");
        std::fs::create_dir(&repository_path).unwrap();
        let target = tempdir.path().join("worktree");
        let first_alias = tempdir.path().join("first").join("..").join("worktree");
        let second_alias = tempdir.path().join("second").join("..").join("worktree");
        let repository_id = RepositoryId::from(Uuid::from_u128(1));
        let first_id = RepositoryWorkspaceId::from(Uuid::from_u128(2));
        let second_id = RepositoryWorkspaceId::from(Uuid::from_u128(3));
        let (model, _operations) = create_acknowledged_model(
            &mut app,
            vec![persisted_repository(repository_id, &repository_path)],
            vec![
                persisted_workspace(first_id, repository_id, "main", &first_alias),
                persisted_workspace(second_id, repository_id, "feature/second", &second_alias),
            ],
        );
        std::fs::create_dir(tempdir.path().join("first")).unwrap();
        std::fs::create_dir(tempdir.path().join("second")).unwrap();
        std::fs::create_dir(&target).unwrap();

        let error = model
            .update(&mut app, |model, ctx| {
                model.insert_workspace(
                    repository_workspace(
                        RepositoryWorkspaceId::from(Uuid::from_u128(4)),
                        repository_id,
                        "feature/new",
                        &target,
                    ),
                    ctx,
                )
            })
            .unwrap_err();

        assert!(matches!(
            error,
            ProjectOrganizationError::AmbiguousWorkspacePath {
                canonical_path,
                workspace_ids,
            } if canonical_path == dunce::canonicalize(&target).unwrap()
                && workspace_ids == vec![first_id, second_id]
        ));
    });
}

#[test]
fn update_workspace_rejects_ambiguous_recovered_aliases_excluding_itself() {
    App::test((), |mut app| async move {
        let tempdir = TempDir::new().unwrap();
        let repository_path = tempdir.path().join("repository");
        std::fs::create_dir(&repository_path).unwrap();
        let target = tempdir.path().join("worktree");
        let first_alias = tempdir.path().join("first").join("..").join("worktree");
        let second_alias = tempdir.path().join("second").join("..").join("worktree");
        let updated_alias = tempdir.path().join("updated").join("..").join("worktree");
        let repository_id = RepositoryId::from(Uuid::from_u128(1));
        let first_id = RepositoryWorkspaceId::from(Uuid::from_u128(2));
        let second_id = RepositoryWorkspaceId::from(Uuid::from_u128(3));
        let updated_id = RepositoryWorkspaceId::from(Uuid::from_u128(4));
        let (model, _operations) = create_acknowledged_model(
            &mut app,
            vec![persisted_repository(repository_id, &repository_path)],
            vec![
                persisted_workspace(first_id, repository_id, "main", &first_alias),
                persisted_workspace(second_id, repository_id, "feature/second", &second_alias),
                persisted_workspace(updated_id, repository_id, "feature/updated", &updated_alias),
            ],
        );
        std::fs::create_dir(tempdir.path().join("first")).unwrap();
        std::fs::create_dir(tempdir.path().join("second")).unwrap();
        std::fs::create_dir(tempdir.path().join("updated")).unwrap();
        std::fs::create_dir(&target).unwrap();
        let mut workspace = model.read(&app, |model, _| {
            model
                .workspace(updated_id)
                .expect("updated workspace should exist")
                .clone()
        });
        workspace.worktree_path = target.clone();

        let error = model
            .update(&mut app, |model, ctx| {
                model.update_workspace(workspace, ctx)
            })
            .unwrap_err();

        assert!(matches!(
            error,
            ProjectOrganizationError::AmbiguousWorkspacePath {
                canonical_path,
                workspace_ids,
            } if canonical_path == dunce::canonicalize(&target).unwrap()
                && workspace_ids == vec![first_id, second_id]
        ));
    });
}

#[test]
fn persisted_repository_alias_is_normalized_and_rejected_as_duplicate() {
    App::test((), |mut app| async move {
        let tempdir = TempDir::new().expect("temporary directory should be created");
        let repository_path = tempdir.path().join("repository");
        std::fs::create_dir(&repository_path).expect("repository directory should be created");
        let alias_path = repository_path.join("..").join("repository");
        let canonical_path =
            dunce::canonicalize(&repository_path).expect("repository path should canonicalize");
        let repository_id = RepositoryId::from(Uuid::new_v4());
        let repository = persisted_repository(repository_id, &alias_path);

        let (model, _events) = create_acknowledged_model(&mut app, vec![repository], vec![]);
        let loaded_path = model.read(&app, |model, _| {
            model
                .repository(repository_id)
                .expect("persisted repository should be retained")
                .path
                .clone()
        });
        let error = model
            .update(&mut app, |model, ctx| {
                model.add_local_repository(&canonical_path, ctx)
            })
            .expect_err("canonical duplicate should be rejected");

        assert_eq!(loaded_path, canonical_path);
        assert!(matches!(
            error,
            ProjectOrganizationError::RepositoryAlreadyExists {
                existing_repository_id,
                canonical_path: existing_path,
            } if existing_repository_id == repository_id && existing_path == canonical_path
        ));
    });
}

#[test]
fn persisted_workspace_alias_is_normalized_and_rejected_as_duplicate() {
    App::test((), |mut app| async move {
        let tempdir = TempDir::new().expect("temporary directory should be created");
        let repository_path = tempdir.path().join("repository");
        let worktree_path = tempdir.path().join("worktree");
        for path in [&repository_path, &worktree_path] {
            std::fs::create_dir(path).expect("test directory should be created");
        }
        let alias_path = worktree_path.join("..").join("worktree");
        let canonical_path =
            dunce::canonicalize(&worktree_path).expect("worktree path should canonicalize");
        let repository_id = RepositoryId::from(Uuid::new_v4());
        let workspace_id = RepositoryWorkspaceId::from(Uuid::new_v4());
        let repository = persisted_repository(repository_id, &repository_path);
        let workspace = persisted_workspace(workspace_id, repository_id, "main", &alias_path);

        let (model, _events) =
            create_acknowledged_model(&mut app, vec![repository], vec![workspace]);
        let loaded_path = model.read(&app, |model, _| {
            model
                .workspace(workspace_id)
                .expect("persisted workspace should be retained")
                .worktree_path
                .clone()
        });
        let error = model
            .update(&mut app, |model, ctx| {
                model.insert_workspace(
                    repository_workspace(
                        RepositoryWorkspaceId::from(Uuid::new_v4()),
                        repository_id,
                        "feature/duplicate-path",
                        &canonical_path,
                    ),
                    ctx,
                )
            })
            .expect_err("canonical worktree duplicate should be rejected");

        assert_eq!(loaded_path, canonical_path);
        assert!(matches!(
            error,
            ProjectOrganizationError::WorkspacePathAlreadyExists {
                existing_workspace_id,
                canonical_path: existing_path,
            } if existing_workspace_id == workspace_id && existing_path == canonical_path
        ));
    });
}

#[test]
fn persisted_repository_alias_duplicate_fails_initialization() {
    App::test((), |mut app| async move {
        let tempdir = TempDir::new().expect("temporary directory should be created");
        let repository_path = tempdir.path().join("repository");
        std::fs::create_dir(&repository_path).expect("repository directory should be created");
        let alias_path = repository_path.join("..").join("repository");
        let canonical_path =
            dunce::canonicalize(&repository_path).expect("repository path should canonicalize");
        let first_id = RepositoryId::from(Uuid::new_v4());
        let second_id = RepositoryId::from(Uuid::new_v4());

        let error = initialization_error(
            &mut app,
            vec![
                persisted_repository(first_id, &alias_path),
                persisted_repository(second_id, &repository_path),
            ],
            vec![],
        );

        assert!(matches!(
            error,
            ProjectOrganizationError::RepositoryAlreadyExists {
                existing_repository_id,
                canonical_path: existing_path,
            } if existing_repository_id == first_id && existing_path == canonical_path
        ));
    });
}

#[test]
fn persisted_workspace_alias_duplicate_fails_initialization() {
    App::test((), |mut app| async move {
        let tempdir = TempDir::new().expect("temporary directory should be created");
        let repository_path = tempdir.path().join("repository");
        let worktree_path = tempdir.path().join("worktree");
        for path in [&repository_path, &worktree_path] {
            std::fs::create_dir(path).expect("test directory should be created");
        }
        let alias_path = worktree_path.join("..").join("worktree");
        let canonical_path =
            dunce::canonicalize(&worktree_path).expect("worktree path should canonicalize");
        let repository_id = RepositoryId::from(Uuid::new_v4());
        let first_id = RepositoryWorkspaceId::from(Uuid::new_v4());
        let second_id = RepositoryWorkspaceId::from(Uuid::new_v4());
        let repository = persisted_repository(repository_id, &repository_path);

        let error = initialization_error(
            &mut app,
            vec![repository],
            vec![
                persisted_workspace(first_id, repository_id, "main", &alias_path),
                persisted_workspace(
                    second_id,
                    repository_id,
                    "feature/duplicate-path",
                    &worktree_path,
                ),
            ],
        );

        assert!(matches!(
            error,
            ProjectOrganizationError::WorkspacePathAlreadyExists {
                existing_workspace_id,
                canonical_path: existing_path,
            } if existing_workspace_id == first_id && existing_path == canonical_path
        ));
    });
}

#[test]
fn persisted_repository_with_missing_path_is_loaded_unchanged() {
    App::test((), |mut app| async move {
        let tempdir = TempDir::new().expect("temporary directory should be created");
        let missing_path = tempdir.path().join("missing-repository");
        let repository_id = RepositoryId::from(Uuid::new_v4());
        let repository = persisted_repository(repository_id, &missing_path);

        let (model, _events) = create_acknowledged_model(&mut app, vec![repository], vec![]);
        let loaded = model.read(&app, |model, _| {
            model
                .repository(repository_id)
                .expect("persisted repository should be retained")
                .clone()
        });

        assert_eq!(loaded.id, repository_id);
        assert_eq!(loaded.source, RepositorySource::Local);
        assert_eq!(loaded.path, missing_path);
    });
}

#[test]
fn persisted_workspace_with_missing_path_is_loaded_unchanged() {
    App::test((), |mut app| async move {
        let tempdir = TempDir::new().expect("temporary directory should be created");
        let repository_path = tempdir.path().join("repository");
        std::fs::create_dir(&repository_path).expect("repository directory should be created");
        let missing_worktree_path = tempdir.path().join("missing-worktree");
        let repository_id = RepositoryId::from(Uuid::new_v4());
        let workspace_id = RepositoryWorkspaceId::from(Uuid::new_v4());
        let repository = persisted_repository(repository_id, &repository_path);
        let workspace = persisted_workspace(
            workspace_id,
            repository_id,
            "feature/missing-worktree",
            &missing_worktree_path,
        );

        let (model, _events) =
            create_acknowledged_model(&mut app, vec![repository], vec![workspace]);
        let loaded = model.read(&app, |model, _| {
            model
                .workspace(workspace_id)
                .expect("persisted workspace should be retained")
                .clone()
        });

        assert_eq!(loaded.repository_id, repository_id);
        assert_eq!(loaded.worktree_path, missing_worktree_path);
    });
}

#[test]
fn persisted_repository_with_invalid_uuid_fails_initialization() {
    App::test((), |mut app| async move {
        let tempdir = TempDir::new().expect("temporary directory should be created");
        let mut repository = persisted_repository(
            RepositoryId::from(Uuid::new_v4()),
            &tempdir.path().join("repository"),
        );
        repository.id = "not-a-repository-uuid".to_string();

        let error = initialization_error(&mut app, vec![repository], vec![]);

        assert!(matches!(
            error,
            ProjectOrganizationError::InvalidPersistedRepositoryId { value, .. }
                if value == "not-a-repository-uuid"
        ));
    });
}

#[test]
fn persisted_repository_with_invalid_source_fails_initialization() {
    App::test((), |mut app| async move {
        let tempdir = TempDir::new().expect("temporary directory should be created");
        let mut repository = persisted_repository(
            RepositoryId::from(Uuid::new_v4()),
            &tempdir.path().join("repository"),
        );
        repository.source = "remote".to_string();

        let error = initialization_error(&mut app, vec![repository], vec![]);

        assert!(matches!(
            error,
            ProjectOrganizationError::InvalidPersistedRepositorySource { value, .. }
                if value == "remote"
        ));
    });
}

#[test]
fn persisted_workspace_with_invalid_uuid_fails_initialization() {
    App::test((), |mut app| async move {
        let tempdir = TempDir::new().expect("temporary directory should be created");
        let repository_id = RepositoryId::from(Uuid::new_v4());
        let repository_path = tempdir.path().join("repository");
        std::fs::create_dir(&repository_path).expect("repository directory should be created");
        let repository = persisted_repository(repository_id, &repository_path);
        let mut workspace = persisted_workspace(
            RepositoryWorkspaceId::from(Uuid::new_v4()),
            repository_id,
            "main",
            &tempdir.path().join("worktree"),
        );
        workspace.id = "not-a-workspace-uuid".to_string();

        let error = initialization_error(&mut app, vec![repository], vec![workspace]);

        assert!(matches!(
            error,
            ProjectOrganizationError::InvalidPersistedWorkspaceId { value, .. }
                if value == "not-a-workspace-uuid"
        ));
    });
}

#[test]
fn persisted_workspace_with_invalid_repository_id_fails_initialization() {
    App::test((), |mut app| async move {
        let tempdir = TempDir::new().expect("temporary directory should be created");
        let repository_id = RepositoryId::from(Uuid::new_v4());
        let repository_path = tempdir.path().join("repository");
        std::fs::create_dir(&repository_path).expect("repository directory should be created");
        let repository = persisted_repository(repository_id, &repository_path);
        let mut workspace = persisted_workspace(
            RepositoryWorkspaceId::from(Uuid::new_v4()),
            repository_id,
            "main",
            &tempdir.path().join("worktree"),
        );
        workspace.repository_id = "not-a-repository-uuid".to_string();

        let error = initialization_error(&mut app, vec![repository], vec![workspace]);

        assert!(matches!(
            error,
            ProjectOrganizationError::InvalidPersistedWorkspaceRepositoryId { value, .. }
                if value == "not-a-repository-uuid"
        ));
    });
}

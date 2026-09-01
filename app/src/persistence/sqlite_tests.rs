#[cfg(target_os = "macos")]
use std::fs;
use std::{path::PathBuf, sync::Arc};

use diesel::connection::SimpleConnection;
use diesel::migration::{MigrationSource, MigrationVersion};
use diesel::prelude::*;
use diesel::result::{DatabaseErrorKind, Error as DieselError};
use diesel::sql_types::{BigInt, Nullable, Text, Timestamp};
use diesel::sqlite::{Sqlite, SqliteConnection};
use diesel_migrations::MigrationHarness;
use warp_core::features::FeatureFlag;

use crate::{
    app_state::{
        AppState, CodePaneSnapShot, CodePaneTabSnapshot, LeafContents, LeafSnapshot,
        PaneNodeSnapshot, TabSnapshot, TerminalPaneSnapshot, WindowSnapshot,
    },
    cloud_object::{Owner, StoredObjectPermissions},
    code::editor_management::CodeSource,
    notebooks::{NotebookObject, NotebookObjectModel},
    persistence::{
        model::{ObjectPermissions, Repository, RepositoryWorkspace},
        BlockCompleted, ModelEvent, RepositoryPersistence, RepositoryPersistenceError,
        RepositoryPersistenceOperation, RepositoryPersistenceRequest,
    },
    project_organization::domain::RepositoryWorkspaceId,
    server::ids::ClientId,
    server_time::ServerTimestamp,
    tab::SelectedTabColor,
    terminal::cli_agent_resume::CliAgentResumeSnapshot,
    terminal::model::block::SerializedBlock,
    terminal::CLIAgent,
    terminal::ShellLaunchData,
};

use super::{
    decode_path, deduplicate_events, delete_repository, delete_repository_workspace, encode_path,
    execute_repository_persistence_operation, get_all_repositories, get_all_repository_workspaces,
    read_sqlite_data, save_app_state, save_repository, save_repository_workspace, setup_database,
    start_writer, start_writer_with_state, WriterState,
};

// Diesel canonicalizes the directory version `2026-07-11-000000` by removing hyphens.
const REPOSITORY_WORKSPACES_MIGRATION_VERSION: &str = "20260711000000";
const DROP_LEGACY_PROJECTS_MIGRATION_VERSION: &str = "20260711010000";

#[derive(QueryableByName)]
struct SqliteTableCount {
    #[diesel(sql_type = BigInt)]
    count: i64,
}

#[derive(QueryableByName)]
struct LegacyProjectRow {
    #[diesel(sql_type = Text)]
    path: String,
    #[diesel(sql_type = Timestamp)]
    added_ts: chrono::NaiveDateTime,
    #[diesel(sql_type = Nullable<Timestamp>)]
    last_opened_ts: Option<chrono::NaiveDateTime>,
}

fn sqlite_table_count(conn: &mut SqliteConnection, table_name: &str) -> i64 {
    diesel::sql_query(
        "SELECT COUNT(*) AS count FROM sqlite_master WHERE type = 'table' AND name = ?",
    )
    .bind::<Text, _>(table_name)
    .get_result::<SqliteTableCount>(conn)
    .expect("sqlite table metadata should load")
    .count
}

fn revert_drop_legacy_projects_migration(conn: &mut SqliteConnection) {
    let target_version = MigrationVersion::from(DROP_LEGACY_PROJECTS_MIGRATION_VERSION);
    let migrations =
        <diesel_migrations::EmbeddedMigrations as MigrationSource<Sqlite>>::migrations(
            &persistence::MIGRATIONS,
        )
        .expect("embedded migrations should load");
    let migration = migrations
        .iter()
        .find(|migration| migration.name().version() == target_version)
        .expect("drop legacy projects migration should be embedded");
    conn.revert_migration(migration.as_ref())
        .expect("drop legacy projects migration should revert");
}

fn revert_repository_workspaces_migration_and_later(conn: &mut SqliteConnection) {
    let target_version = MigrationVersion::from(REPOSITORY_WORKSPACES_MIGRATION_VERSION);
    let applied_versions = conn
        .applied_migrations()
        .expect("applied migrations should load");
    let migrations =
        <diesel_migrations::EmbeddedMigrations as MigrationSource<Sqlite>>::migrations(
            &persistence::MIGRATIONS,
        )
        .expect("embedded migrations should load");
    let versions_to_revert = applied_versions
        .into_iter()
        .take_while(|version| version >= &target_version)
        .collect::<Vec<_>>();

    assert!(
        versions_to_revert
            .iter()
            .any(|version| version == &target_version),
        "repository workspaces migration should be applied"
    );
    for version in versions_to_revert {
        let migration = migrations
            .iter()
            .find(|migration| migration.name().version() == version)
            .unwrap_or_else(|| panic!("migration {version} should be embedded"));
        conn.revert_migration(migration.as_ref())
            .unwrap_or_else(|error| panic!("migration {version} should revert: {error}"));
    }
}

#[test]
fn all_migrations_remove_legacy_projects_table() {
    let tempdir = tempfile::tempdir().expect("tempdir should be created");
    let database_path = tempdir.path().join("warp.sqlite");
    let mut conn = setup_database(&database_path).expect("database should initialize");

    assert_eq!(sqlite_table_count(&mut conn, "projects"), 0);
}

#[test]
fn reverting_drop_legacy_projects_backfills_from_repositories() {
    let tempdir = tempfile::tempdir().expect("tempdir should be created");
    let database_path = tempdir.path().join("warp.sqlite");
    let mut conn = setup_database(&database_path).expect("database should initialize");
    let repository = repository_row(
        "123e4567-e89b-12d3-a456-426614174099",
        "/tmp/zap-project-backfill",
    );
    save_repository(&mut conn, repository.clone()).expect("repository should save");

    revert_drop_legacy_projects_migration(&mut conn);

    let project =
        diesel::sql_query("SELECT path, added_ts, last_opened_ts FROM projects WHERE path = ?")
            .bind::<Text, _>(&repository.path)
            .get_result::<LegacyProjectRow>(&mut conn)
            .expect("legacy project should be backfilled");
    assert_eq!(project.path, repository.path);
    assert_eq!(project.added_ts, repository.created_at);
    assert_eq!(project.last_opened_ts, Some(repository.last_opened_at));

    conn.run_pending_migrations(persistence::MIGRATIONS)
        .expect("drop legacy projects migration should run again");
    assert_eq!(sqlite_table_count(&mut conn, "projects"), 0);
}

#[test]
fn test_deduplicate_snapshots() {
    let local_notebook = NotebookObject::new_local(
        NotebookObjectModel {
            title: "Hello".to_string(),
            data: "World".to_string(),
            ai_document_id: None,
            conversation_id: None,
        },
        Owner::mock_current_user(),
        None,
        ClientId::new(),
    );
    let completed_block_1 = BlockCompleted {
        pane_id: vec![1, 2, 3],
        block: Arc::new(SerializedBlock::default()),
        is_local: true,
    };
    let completed_block_2 = BlockCompleted {
        pane_id: vec![4, 5, 6],
        block: Arc::new(SerializedBlock::default()),
        is_local: true,
    };
    let snapshot_1 = AppState {
        active_window_index: Some(1),
        block_lists: Default::default(),
        windows: Default::default(),
        running_mcp_servers: Default::default(),
    };
    let snapshot_2 = AppState {
        active_window_index: Some(2),
        block_lists: Default::default(),
        windows: Default::default(),
        running_mcp_servers: Default::default(),
    };
    let snapshot_3 = AppState {
        active_window_index: Some(3),
        block_lists: Default::default(),
        windows: Default::default(),
        running_mcp_servers: Default::default(),
    };

    let original_events = vec![
        ModelEvent::UpsertNotebook {
            notebook: local_notebook.clone(),
        },
        ModelEvent::Snapshot(snapshot_1.clone()),
        ModelEvent::SaveBlock(completed_block_1.clone()),
        ModelEvent::Snapshot(snapshot_2.clone()),
        ModelEvent::SaveBlock(completed_block_2.clone()),
        ModelEvent::Snapshot(snapshot_3.clone()),
        ModelEvent::UpsertNotebook {
            notebook: local_notebook.clone(),
        },
    ];

    let filtered_events = deduplicate_events(original_events);
    assert_eq!(filtered_events.len(), 5);

    assert!(matches!(
        &filtered_events[0],
        &ModelEvent::UpsertNotebook { .. }
    ));
    // The first snapshot should have been filtered out.
    assert!(matches!(&filtered_events[1], &ModelEvent::SaveBlock(_)));
    // The second snapshot should have been filtered out.
    assert!(matches!(&filtered_events[2], &ModelEvent::SaveBlock(_)));
    // The third snapshot should be preserved.
    match &filtered_events[3] {
        ModelEvent::Snapshot(snapshot) => assert_eq!(snapshot, &snapshot_3),
        other => panic!("Expected ModelEvent::Snapshot, got {other:?}"),
    }
    assert!(matches!(
        &filtered_events[4],
        &ModelEvent::UpsertNotebook { .. }
    ));
}

#[test]
fn test_deduplicate_no_snapshots() {
    let original_events = vec![ModelEvent::SaveBlock(BlockCompleted {
        pane_id: vec![1, 2, 3],
        block: Default::default(),
        is_local: true,
    })];
    let filtered_events = deduplicate_events(original_events);
    assert_eq!(filtered_events.len(), 1);
    assert!(matches!(&filtered_events[0], &ModelEvent::SaveBlock(_)));
}

fn test_terminal_window_snapshot(vertical_tabs_panel_open: bool) -> WindowSnapshot {
    WindowSnapshot {
        tabs: vec![TabSnapshot {
            repository_workspace_id: None,
            custom_title: None,
            root: PaneNodeSnapshot::Leaf(LeafSnapshot {
                is_focused: true,
                custom_vertical_tabs_title: None,
                contents: LeafContents::Terminal(TerminalPaneSnapshot {
                    uuid: vec![u8::from(vertical_tabs_panel_open) + 1],
                    cwd: Some("/tmp".to_string()),
                    shell_launch_data: Some(ShellLaunchData::Executable {
                        executable_path: PathBuf::from("/bin/zsh"),
                        shell_type: crate::terminal::shell::ShellType::Zsh,
                    }),
                    is_active: true,
                    is_read_only: false,
                    input_config: None,
                    llm_model_override: None,
                    active_profile_id: None,
                    conversation_ids_to_restore: vec![],
                    active_conversation_id: None,
                    cli_agent_resume: None,
                }),
            }),
            default_directory_color: None,
            selected_color: SelectedTabColor::default(),
            left_panel: None,
            right_panel: None,
        }],
        active_tab_index: 0,
        active_repository_workspace_id: None,
        repository_workspace_states: Vec::new(),
        bounds: None,
        fullscreen_state: Default::default(),
        quake_mode: false,
        universal_search_width: None,
        warp_ai_width: None,
        voltron_width: None,
        warp_drive_index_width: None,
        left_panel_open: false,
        vertical_tabs_panel_open,
        left_panel_width: None,
        right_panel_width: None,
        agent_management_filters: None,
        theme_override: None,
    }
}

#[test]
fn test_sqlite_round_trips_vertical_tabs_panel_open() {
    let tempdir = tempfile::tempdir().expect("tempdir should be created");
    let database_path = tempdir.path().join("warp.sqlite");
    let mut conn = setup_database(&database_path).expect("database should initialize");

    let app_state = AppState {
        windows: vec![
            test_terminal_window_snapshot(false),
            test_terminal_window_snapshot(true),
        ],
        active_window_index: Some(1),
        block_lists: Default::default(),
        running_mcp_servers: Default::default(),
    };

    save_app_state(&mut conn, &app_state).expect("app state should save");

    let restored = read_sqlite_data(&mut conn, None)
        .expect("app state should load")
        .app_state;

    assert_eq!(restored.active_window_index, Some(1));
    assert_eq!(
        restored
            .windows
            .iter()
            .map(|window| window.vertical_tabs_panel_open)
            .collect::<Vec<_>>(),
        vec![false, true]
    );
}

#[test]
fn repository_workspace_state_round_trips() {
    let tempdir = tempfile::tempdir().expect("tempdir should be created");
    let database_path = tempdir.path().join("warp.sqlite");
    let mut conn = setup_database(&database_path).expect("database should initialize");
    let repository = repository_row(
        "123e4567-e89b-12d3-a456-426614174100",
        "/tmp/repository-workspace-state",
    );
    let workspace = repository_workspace_row(
        "123e4567-e89b-12d3-a456-426614174101",
        &repository.id,
        "feature/workspace-state",
        "/tmp/repository-workspace-state-feature",
    );
    let workspace_id = RepositoryWorkspaceId(
        workspace
            .id
            .parse()
            .expect("workspace fixture should have a valid UUID"),
    );
    save_repository(&mut conn, repository).expect("repository should save");
    save_repository_workspace(&mut conn, workspace).expect("workspace should save");

    let mut window = test_terminal_window_snapshot(false);
    window.active_repository_workspace_id = Some(workspace_id);
    window.repository_workspace_states =
        vec![crate::app_state::RepositoryWorkspaceWindowStateSnapshot {
            repository_workspace_id: workspace_id,
            active_tab_index: 0,
        }];
    window.tabs[0].repository_workspace_id = Some(workspace_id);
    let state = AppState {
        windows: vec![window],
        active_window_index: Some(0),
        block_lists: Default::default(),
        running_mcp_servers: Default::default(),
    };

    save_app_state(&mut conn, &state).expect("app state should save");
    let restored = read_sqlite_data(&mut conn, None)
        .expect("app state should load")
        .app_state;

    assert_eq!(restored.windows, state.windows);
    assert_eq!(restored.active_window_index, state.active_window_index);
}

#[test]
fn empty_repository_workspace_window_round_trips() {
    let tempdir = tempfile::tempdir().expect("tempdir should be created");
    let database_path = tempdir.path().join("warp.sqlite");
    let mut conn = setup_database(&database_path).expect("database should initialize");
    let repository = repository_row(
        "123e4567-e89b-12d3-a456-426614174200",
        "/tmp/empty-repository-workspace",
    );
    let workspace = repository_workspace_row(
        "123e4567-e89b-12d3-a456-426614174201",
        &repository.id,
        "feature/empty-workspace",
        "/tmp/empty-repository-workspace-feature",
    );
    let workspace_id = RepositoryWorkspaceId(
        workspace
            .id
            .parse()
            .expect("workspace fixture should have a valid UUID"),
    );
    save_repository(&mut conn, repository).expect("repository should save");
    save_repository_workspace(&mut conn, workspace).expect("workspace should save");

    let mut window = test_terminal_window_snapshot(false);
    window.tabs.clear();
    window.active_tab_index = 0;
    window.active_repository_workspace_id = Some(workspace_id);
    window.repository_workspace_states = Vec::new();
    let state = AppState {
        windows: vec![window],
        active_window_index: Some(0),
        block_lists: Default::default(),
        running_mcp_servers: Default::default(),
    };

    save_app_state(&mut conn, &state).expect("empty workspace window should save");
    let restored = read_sqlite_data(&mut conn, None)
        .expect("empty workspace window should load")
        .app_state;

    assert_eq!(restored.windows, state.windows);
    assert!(restored.windows[0].tabs.is_empty());
    assert_eq!(
        restored.windows[0].active_repository_workspace_id,
        Some(workspace_id)
    );
}

#[test]
fn repository_rows_round_trip() {
    let tempdir = tempfile::tempdir().expect("tempdir should be created");
    let database_path = tempdir.path().join("warp.sqlite");
    let mut conn = setup_database(&database_path).expect("database should initialize");
    let created_at = chrono::DateTime::from_timestamp(1_700_000_000, 0)
        .expect("timestamp should be valid")
        .naive_utc();
    let last_opened_at = chrono::DateTime::from_timestamp(1_700_000_100, 0)
        .expect("timestamp should be valid")
        .naive_utc();
    let repository = Repository {
        id: "123e4567-e89b-12d3-a456-426614174000".to_string(),
        display_name: "zap".to_string(),
        path: "/tmp/zap".to_string(),
        remote_url: None,
        source: "local".to_string(),
        created_at,
        last_opened_at,
    };

    save_repository(&mut conn, repository.clone()).expect("repository should save");

    assert_eq!(
        get_all_repositories(&mut conn).expect("repositories should load"),
        vec![repository]
    );
}

#[test]
fn legacy_project_migration_normalizes_repository_display_name() {
    let tempdir = tempfile::tempdir().expect("tempdir should be created");
    let repository_path = tempdir.path().join("repository");
    let database_path = tempdir.path().join("warp.sqlite");
    let mut conn = setup_database(&database_path).expect("database should initialize");
    revert_repository_workspaces_migration_and_later(&mut conn);
    diesel::sql_query("INSERT INTO projects (path, added_ts, last_opened_ts) VALUES (?, ?, NULL)")
        .bind::<Text, _>(
            repository_path
                .to_str()
                .expect("repository path should be valid UTF-8"),
        )
        .bind::<Timestamp, _>(
            chrono::DateTime::from_timestamp(1_700_000_000, 0)
                .expect("timestamp should be valid")
                .naive_utc(),
        )
        .execute(&mut conn)
        .expect("legacy project should insert");
    conn.run_pending_migrations(persistence::MIGRATIONS)
        .expect("repository workspace migration should run");

    let persisted_data = read_sqlite_data(&mut conn, None).expect("database should load");

    assert_eq!(persisted_data.repositories[0].display_name, "repository");
    assert_eq!(
        crate::persistence::schema::repositories::table
            .select(crate::persistence::schema::repositories::display_name)
            .first::<String>(&mut conn)
            .expect("repository display name should load"),
        "repository"
    );
}

#[test]
fn malformed_repository_row_returns_error() {
    let tempdir = tempfile::tempdir().expect("tempdir should be created");
    let database_path = tempdir.path().join("warp.sqlite");
    let mut conn = setup_database(&database_path).expect("database should initialize");
    conn.batch_execute(
        "INSERT INTO repositories (
            id, display_name, path, remote_url, source, created_at, last_opened_at
         ) VALUES (
            '123e4567-e89b-12d3-a456-426614174001', 'malformed', '/tmp/malformed', NULL,
            'local', 'not-a-timestamp', '2026-07-11 01:02:03'
         );",
    )
    .expect("malformed repository row should insert");

    assert!(get_all_repositories(&mut conn).is_err());
}

#[test]
fn malformed_repository_workspace_row_returns_error() {
    let tempdir = tempfile::tempdir().expect("tempdir should be created");
    let database_path = tempdir.path().join("warp.sqlite");
    let mut conn = setup_database(&database_path).expect("database should initialize");
    let now = chrono::Utc::now().naive_utc();
    let repository = Repository {
        id: "123e4567-e89b-12d3-a456-426614174002".to_string(),
        display_name: "zap".to_string(),
        path: "/tmp/zap-malformed-workspace".to_string(),
        remote_url: None,
        source: "local".to_string(),
        created_at: now,
        last_opened_at: now,
    };
    save_repository(&mut conn, repository).expect("repository should save");
    conn.batch_execute(
        "INSERT INTO repository_workspaces (
            id, repository_id, display_name, branch, worktree_path, created_at, last_opened_at
         ) VALUES (
            '123e4567-e89b-12d3-a456-426614174003',
            '123e4567-e89b-12d3-a456-426614174002',
            'main', 'main', '/tmp/zap-malformed-workspace-main',
            'not-a-timestamp', '2026-07-11 01:02:03'
         );",
    )
    .expect("malformed repository workspace row should insert");

    assert!(get_all_repository_workspaces(&mut conn).is_err());
}

fn repository_row(id: &str, path: &str) -> Repository {
    let now = chrono::DateTime::from_timestamp(1_700_000_000, 0)
        .expect("timestamp should be valid")
        .naive_utc();
    Repository {
        id: id.to_string(),
        display_name: "zap".to_string(),
        path: path.to_string(),
        remote_url: None,
        source: "local".to_string(),
        created_at: now,
        last_opened_at: now,
    }
}

#[test]
fn repository_persistence_acknowledges_committed_upsert() {
    let tempdir = tempfile::tempdir().expect("tempdir should be created");
    let database_path = tempdir.path().join("warp.sqlite");
    let conn = setup_database(&database_path).expect("database should initialize");
    let handles = start_writer(conn, database_path.clone()).expect("writer should start");
    let persistence = RepositoryPersistence::new(Some(handles.sender.clone()));
    let repository = repository_row(
        "123e4567-e89b-12d3-a456-426614174100",
        "/tmp/ack-repository",
    );

    persistence
        .execute(RepositoryPersistenceOperation::UpsertRepository {
            repository: repository.clone(),
        })
        .expect("repository upsert should be acknowledged");

    let mut read_conn = setup_database(&database_path).expect("read connection should initialize");
    assert_eq!(
        get_all_repositories(&mut read_conn).expect("repositories should load"),
        vec![repository]
    );
    handles
        .sender
        .send(ModelEvent::Terminate)
        .expect("writer should receive termination");
    handles.handle.join().expect("writer should terminate");
}

#[test]
fn repository_persistence_returns_database_error() {
    let tempdir = tempfile::tempdir().expect("tempdir should be created");
    let database_path = tempdir.path().join("warp.sqlite");
    let conn = setup_database(&database_path).expect("database should initialize");
    let handles = start_writer(conn, database_path.clone()).expect("writer should start");
    let persistence = RepositoryPersistence::new(Some(handles.sender.clone()));
    let first = repository_row(
        "123e4567-e89b-12d3-a456-426614174101",
        "/tmp/ack-duplicate-path",
    );
    let second = repository_row(
        "123e4567-e89b-12d3-a456-426614174102",
        "/tmp/ack-duplicate-path",
    );
    persistence
        .execute(RepositoryPersistenceOperation::UpsertRepository { repository: first })
        .expect("first repository should persist");

    let error = persistence
        .execute(RepositoryPersistenceOperation::UpsertRepository { repository: second })
        .expect_err("duplicate path should fail");

    match error {
        RepositoryPersistenceError::Database { details } => {
            assert!(details.contains("error upserting repository"));
            assert!(details.contains("UNIQUE constraint failed: repositories.path"));
        }
        other => panic!("expected database error, got {other:?}"),
    }
    handles
        .sender
        .send(ModelEvent::Terminate)
        .expect("writer should receive termination");
    handles.handle.join().expect("writer should terminate");
}

#[test]
fn repository_persistence_reports_database_error() {
    let tempdir = tempfile::tempdir().expect("tempdir should be created");
    let database_path = tempdir.path().join("warp.sqlite");
    let mut conn = setup_database(&database_path).expect("database should initialize");
    let first = repository_row(
        "123e4567-e89b-12d3-a456-426614174111",
        "/tmp/ack-reported-error",
    );
    let second = repository_row(
        "123e4567-e89b-12d3-a456-426614174112",
        "/tmp/ack-reported-error",
    );
    save_repository(&mut conn, first).expect("first repository should persist");
    let mut reported_error = None;

    let error = execute_repository_persistence_operation(
        RepositoryPersistenceOperation::UpsertRepository { repository: second },
        &mut conn,
        &database_path,
        |error_kind, error, reported_path| {
            reported_error = Some((
                error_kind.to_string(),
                format!("{error:#}"),
                reported_path.to_path_buf(),
            ));
        },
    )
    .expect_err("duplicate path should fail");

    let RepositoryPersistenceError::Database { details } = error else {
        panic!("expected database error, got {error:?}");
    };
    assert!(details.contains("error upserting repository"));
    assert!(details.contains("UNIQUE constraint failed: repositories.path"));
    let (error_kind, reported_details, reported_path) =
        reported_error.expect("database error should be reported");
    assert_eq!(error_kind, "Repository persistence");
    assert_eq!(reported_details, details);
    assert_eq!(reported_path, database_path);
}

#[test]
fn repository_persistence_fails_while_writer_is_paused() {
    let tempdir = tempfile::tempdir().expect("tempdir should be created");
    let database_path = tempdir.path().join("warp.sqlite");
    let conn = setup_database(&database_path).expect("database should initialize");
    let handles = start_writer_with_state(conn, database_path.clone(), WriterState::Paused)
        .expect("paused writer should start");
    let persistence = RepositoryPersistence::new(Some(handles.sender.clone()));
    let repository = repository_row(
        "123e4567-e89b-12d3-a456-426614174103",
        "/tmp/ack-paused-repository",
    );

    assert_eq!(
        persistence.execute(RepositoryPersistenceOperation::UpsertRepository { repository }),
        Err(RepositoryPersistenceError::Paused)
    );

    let mut read_conn = setup_database(&database_path).expect("read connection should initialize");
    assert!(get_all_repositories(&mut read_conn)
        .expect("repositories should load")
        .is_empty());
    handles
        .sender
        .send(ModelEvent::Terminate)
        .expect("writer should receive termination");
    handles.handle.join().expect("writer should terminate");
}

#[test]
fn repository_persistence_returns_unavailable_without_sender() {
    let persistence = RepositoryPersistence::new(None);
    let repository = repository_row(
        "123e4567-e89b-12d3-a456-426614174104",
        "/tmp/ack-unavailable-repository",
    );

    assert_eq!(
        persistence.execute(RepositoryPersistenceOperation::UpsertRepository { repository }),
        Err(RepositoryPersistenceError::Unavailable)
    );
}

#[test]
fn repository_persistence_returns_request_disconnected() {
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    drop(receiver);
    let persistence = RepositoryPersistence::new(Some(sender));
    let repository = repository_row(
        "123e4567-e89b-12d3-a456-426614174105",
        "/tmp/ack-request-disconnected",
    );

    let error = persistence
        .execute(RepositoryPersistenceOperation::UpsertRepository { repository })
        .expect_err("disconnected request channel should fail");

    assert!(matches!(
        error,
        RepositoryPersistenceError::RequestDisconnected { .. }
    ));
}

#[test]
fn repository_persistence_returns_response_disconnected() {
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    let receiver_thread = std::thread::spawn(move || {
        let event = receiver.recv().expect("request should be received");
        let ModelEvent::RepositoryPersistence(request) = event else {
            panic!("expected repository persistence request, got {event:?}");
        };
        drop(request.response);
    });
    let persistence = RepositoryPersistence::new(Some(sender));
    let repository = repository_row(
        "123e4567-e89b-12d3-a456-426614174106",
        "/tmp/ack-response-disconnected",
    );

    let error = persistence
        .execute(RepositoryPersistenceOperation::UpsertRepository { repository })
        .expect_err("disconnected response channel should fail");

    assert!(matches!(
        error,
        RepositoryPersistenceError::ResponseDisconnected { .. }
    ));
    receiver_thread.join().expect("receiver should terminate");
}

#[test]
fn repository_persistence_writer_continues_after_response_disconnect() {
    let tempdir = tempfile::tempdir().expect("tempdir should be created");
    let database_path = tempdir.path().join("warp.sqlite");
    let conn = setup_database(&database_path).expect("database should initialize");
    let handles = start_writer(conn, database_path.clone()).expect("writer should start");
    let persistence = RepositoryPersistence::new(Some(handles.sender.clone()));
    let first = repository_row(
        "123e4567-e89b-12d3-a456-426614174107",
        "/tmp/ack-dropped-response",
    );
    let second = repository_row(
        "123e4567-e89b-12d3-a456-426614174108",
        "/tmp/ack-after-dropped-response",
    );
    let (response, receiver) = std::sync::mpsc::sync_channel(1);
    drop(receiver);
    handles
        .sender
        .send(ModelEvent::RepositoryPersistence(
            RepositoryPersistenceRequest {
                operation: RepositoryPersistenceOperation::UpsertRepository {
                    repository: first.clone(),
                },
                response,
            },
        ))
        .expect("writer should receive request with disconnected response");

    persistence
        .execute(RepositoryPersistenceOperation::UpsertRepository {
            repository: second.clone(),
        })
        .expect("writer should continue serving requests");

    let mut read_conn = setup_database(&database_path).expect("read connection should initialize");
    let mut repositories = get_all_repositories(&mut read_conn).expect("repositories should load");
    repositories.sort_by(|left, right| left.id.cmp(&right.id));
    assert_eq!(repositories, vec![first, second]);
    handles
        .sender
        .send(ModelEvent::Terminate)
        .expect("writer should receive termination");
    handles.handle.join().expect("writer should terminate");
}

#[test]
fn repository_persistence_operation_matrix_commits_before_acknowledgement() {
    let tempdir = tempfile::tempdir().expect("tempdir should be created");
    let database_path = tempdir.path().join("warp.sqlite");
    let conn = setup_database(&database_path).expect("database should initialize");
    let handles = start_writer(conn, database_path.clone()).expect("writer should start");
    let persistence = RepositoryPersistence::new(Some(handles.sender.clone()));
    let repository = repository_row(
        "123e4567-e89b-12d3-a456-426614174109",
        "/tmp/ack-operation-matrix",
    );
    let workspace = repository_workspace_row(
        "123e4567-e89b-12d3-a456-426614174110",
        &repository.id,
        "main",
        "/tmp/ack-operation-matrix-main",
    );
    let mut read_conn = setup_database(&database_path).expect("read connection should initialize");

    persistence
        .execute(RepositoryPersistenceOperation::UpsertRepository {
            repository: repository.clone(),
        })
        .expect("repository upsert should be acknowledged");
    assert_eq!(
        get_all_repositories(&mut read_conn).expect("repositories should load"),
        vec![repository.clone()]
    );

    persistence
        .execute(RepositoryPersistenceOperation::UpsertRepositoryWorkspace {
            workspace: workspace.clone(),
        })
        .expect("workspace upsert should be acknowledged");
    assert_eq!(
        get_all_repository_workspaces(&mut read_conn).expect("workspaces should load"),
        vec![workspace.clone()]
    );

    persistence
        .execute(RepositoryPersistenceOperation::DeleteRepositoryWorkspace {
            workspace_id: workspace.id,
        })
        .expect("workspace delete should be acknowledged");
    assert!(get_all_repository_workspaces(&mut read_conn)
        .expect("workspaces should load")
        .is_empty());

    persistence
        .execute(RepositoryPersistenceOperation::DeleteRepository {
            repository_id: repository.id,
        })
        .expect("repository delete should be acknowledged");
    assert!(get_all_repositories(&mut read_conn)
        .expect("repositories should load")
        .is_empty());
    handles
        .sender
        .send(ModelEvent::Terminate)
        .expect("writer should receive termination");
    handles.handle.join().expect("writer should terminate");
}

#[test]
fn repository_and_initial_workspace_are_persisted_as_one_transaction() {
    let tempdir = tempfile::tempdir().expect("tempdir should be created");
    let database_path = tempdir.path().join("warp.sqlite");
    let conn = setup_database(&database_path).expect("database should initialize");
    let handles = start_writer(conn, database_path.clone()).expect("writer should start");
    let persistence = RepositoryPersistence::new(Some(handles.sender.clone()));
    let repository = repository_row(
        "123e4567-e89b-12d3-a456-426614174113",
        "/tmp/atomic-repository",
    );
    let workspace = repository_workspace_row(
        "123e4567-e89b-12d3-a456-426614174114",
        &repository.id,
        "main",
        "/tmp/atomic-repository",
    );

    persistence
        .execute(
            RepositoryPersistenceOperation::UpsertRepositoryWithWorkspace {
                repository: repository.clone(),
                workspace: workspace.clone(),
            },
        )
        .expect("repository and workspace upsert should be acknowledged");

    let mut read_conn = setup_database(&database_path).expect("read connection should initialize");
    assert_eq!(
        get_all_repositories(&mut read_conn).expect("repositories should load"),
        vec![repository]
    );
    assert_eq!(
        get_all_repository_workspaces(&mut read_conn).expect("workspaces should load"),
        vec![workspace]
    );
    handles
        .sender
        .send(ModelEvent::Terminate)
        .expect("writer should receive termination");
    handles.handle.join().expect("writer should terminate");
}

#[test]
fn failed_initial_workspace_upsert_rolls_back_the_new_repository() {
    let tempdir = tempfile::tempdir().expect("tempdir should be created");
    let database_path = tempdir.path().join("warp.sqlite");
    let mut conn = setup_database(&database_path).expect("database should initialize");
    let existing_repository = repository_row(
        "123e4567-e89b-12d3-a456-426614174115",
        "/tmp/atomic-existing-repository",
    );
    let conflicting_workspace = repository_workspace_row(
        "123e4567-e89b-12d3-a456-426614174116",
        &existing_repository.id,
        "main",
        "/tmp/atomic-conflicting-worktree",
    );
    save_repository(&mut conn, existing_repository.clone()).expect("repository should save");
    save_repository_workspace(&mut conn, conflicting_workspace.clone())
        .expect("workspace should save");
    drop(conn);

    let conn = setup_database(&database_path).expect("database should initialize");
    let handles = start_writer(conn, database_path.clone()).expect("writer should start");
    let persistence = RepositoryPersistence::new(Some(handles.sender.clone()));
    let new_repository = repository_row(
        "123e4567-e89b-12d3-a456-426614174117",
        "/tmp/atomic-new-repository",
    );
    let new_workspace = repository_workspace_row(
        "123e4567-e89b-12d3-a456-426614174118",
        &new_repository.id,
        "main",
        "/tmp/atomic-conflicting-worktree",
    );

    assert!(matches!(
        persistence.execute(
            RepositoryPersistenceOperation::UpsertRepositoryWithWorkspace {
                repository: new_repository.clone(),
                workspace: new_workspace,
            }
        ),
        Err(RepositoryPersistenceError::Database { .. })
    ));

    let mut read_conn = setup_database(&database_path).expect("read connection should initialize");
    assert_eq!(
        get_all_repositories(&mut read_conn).expect("repositories should load"),
        vec![existing_repository]
    );
    assert_eq!(
        get_all_repository_workspaces(&mut read_conn).expect("workspaces should load"),
        vec![conflicting_workspace]
    );
    handles
        .sender
        .send(ModelEvent::Terminate)
        .expect("writer should receive termination");
    handles.handle.join().expect("writer should terminate");
}

fn repository_workspace_row(
    id: &str,
    repository_id: &str,
    branch: &str,
    worktree_path: &str,
) -> RepositoryWorkspace {
    let now = chrono::DateTime::from_timestamp(1_700_000_000, 0)
        .expect("timestamp should be valid")
        .naive_utc();
    RepositoryWorkspace {
        id: id.to_string(),
        repository_id: repository_id.to_string(),
        display_name: branch.to_string(),
        branch: branch.to_string(),
        worktree_path: worktree_path.to_string(),
        created_at: now,
        last_opened_at: now,
    }
}

fn save_test_window_and_tab(conn: &mut SqliteConnection) -> (i32, i32) {
    let app_state = AppState {
        windows: vec![test_terminal_window_snapshot(false)],
        active_window_index: None,
        block_lists: Default::default(),
        running_mcp_servers: Default::default(),
    };
    save_app_state(conn, &app_state).expect("app state should save");

    let window_id = crate::persistence::schema::windows::table
        .select(crate::persistence::schema::windows::id)
        .first(conn)
        .expect("window should load");
    let tab_id = crate::persistence::schema::tabs::table
        .select(crate::persistence::schema::tabs::id)
        .first(conn)
        .expect("tab should load");
    (window_id, tab_id)
}

fn insert_repository_workspace_window_state(
    conn: &mut SqliteConnection,
    window_id: i32,
    repository_workspace_id: &str,
) {
    use crate::persistence::schema::repository_workspace_window_states::dsl;

    diesel::insert_into(dsl::repository_workspace_window_states)
        .values((
            dsl::window_id.eq(window_id),
            dsl::repository_workspace_id.eq(repository_workspace_id),
            dsl::active_tab_index.eq(0),
        ))
        .execute(conn)
        .expect("repository workspace window state should insert");
}

fn assert_unique_violation(error: &anyhow::Error) {
    assert!(matches!(
        error.downcast_ref::<DieselError>(),
        Some(DieselError::DatabaseError(
            DatabaseErrorKind::UniqueViolation,
            _
        ))
    ));
}

#[test]
fn repository_workspace_rows_support_crud_and_repository_restrict() {
    let tempdir = tempfile::tempdir().expect("tempdir should be created");
    let database_path = tempdir.path().join("warp.sqlite");
    let mut conn = setup_database(&database_path).expect("database should initialize");
    let created_at = chrono::DateTime::from_timestamp(1_700_000_000, 0)
        .expect("timestamp should be valid")
        .naive_utc();
    let repository = Repository {
        id: "123e4567-e89b-12d3-a456-426614174004".to_string(),
        display_name: "zap".to_string(),
        path: "/tmp/zap-workspace-crud".to_string(),
        remote_url: None,
        source: "local".to_string(),
        created_at,
        last_opened_at: created_at,
    };
    save_repository(&mut conn, repository.clone()).expect("repository should save");
    let mut workspace = RepositoryWorkspace {
        id: "123e4567-e89b-12d3-a456-426614174005".to_string(),
        repository_id: repository.id.clone(),
        display_name: "main".to_string(),
        branch: "main".to_string(),
        worktree_path: "/tmp/zap-workspace-crud-main".to_string(),
        created_at,
        last_opened_at: created_at,
    };

    save_repository_workspace(&mut conn, workspace.clone()).expect("workspace should save");
    assert_eq!(
        get_all_repository_workspaces(&mut conn).expect("workspaces should load"),
        vec![workspace.clone()]
    );

    workspace.display_name = "Main workspace".to_string();
    workspace.last_opened_at = chrono::DateTime::from_timestamp(1_700_000_100, 0)
        .expect("timestamp should be valid")
        .naive_utc();
    save_repository_workspace(&mut conn, workspace.clone()).expect("workspace should update");
    assert_eq!(
        get_all_repository_workspaces(&mut conn).expect("workspaces should load"),
        vec![workspace.clone()]
    );
    assert!(delete_repository(&mut conn, &repository.id).is_err());

    delete_repository_workspace(&mut conn, &workspace.id).expect("workspace should delete");
    assert!(get_all_repository_workspaces(&mut conn)
        .expect("workspaces should load")
        .is_empty());
    delete_repository(&mut conn, &repository.id).expect("repository should delete");
}

#[test]
fn repository_workspace_window_state_cascades_when_window_is_deleted() {
    use crate::persistence::schema::repository_workspace_window_states::dsl as window_states;

    let tempdir = tempfile::tempdir().expect("tempdir should be created");
    let database_path = tempdir.path().join("warp.sqlite");
    let mut conn = setup_database(&database_path).expect("database should initialize");
    let repository = repository_row(
        "123e4567-e89b-12d3-a456-426614174010",
        "/tmp/zap-window-cascade",
    );
    let workspace = repository_workspace_row(
        "123e4567-e89b-12d3-a456-426614174011",
        &repository.id,
        "main",
        "/tmp/zap-window-cascade-main",
    );
    save_repository(&mut conn, repository).expect("repository should save");
    save_repository_workspace(&mut conn, workspace.clone()).expect("workspace should save");
    let (window_id, _) = save_test_window_and_tab(&mut conn);
    insert_repository_workspace_window_state(&mut conn, window_id, &workspace.id);

    save_app_state(
        &mut conn,
        &AppState {
            windows: vec![],
            active_window_index: None,
            block_lists: Default::default(),
            running_mcp_servers: Default::default(),
        },
    )
    .expect("empty app state should save");

    assert_eq!(
        window_states::repository_workspace_window_states
            .count()
            .get_result::<i64>(&mut conn)
            .expect("window state count should load"),
        0
    );
}

#[test]
fn repository_workspace_window_state_cascades_when_workspace_is_deleted() {
    use crate::persistence::schema::repository_workspace_window_states::dsl as window_states;

    let tempdir = tempfile::tempdir().expect("tempdir should be created");
    let database_path = tempdir.path().join("warp.sqlite");
    let mut conn = setup_database(&database_path).expect("database should initialize");
    let repository = repository_row(
        "123e4567-e89b-12d3-a456-426614174012",
        "/tmp/zap-workspace-cascade",
    );
    let workspace = repository_workspace_row(
        "123e4567-e89b-12d3-a456-426614174013",
        &repository.id,
        "main",
        "/tmp/zap-workspace-cascade-main",
    );
    save_repository(&mut conn, repository).expect("repository should save");
    save_repository_workspace(&mut conn, workspace.clone()).expect("workspace should save");
    let (window_id, _) = save_test_window_and_tab(&mut conn);
    insert_repository_workspace_window_state(&mut conn, window_id, &workspace.id);

    delete_repository_workspace(&mut conn, &workspace.id).expect("workspace should delete");

    assert_eq!(
        window_states::repository_workspace_window_states
            .count()
            .get_result::<i64>(&mut conn)
            .expect("window state count should load"),
        0
    );
}

#[test]
fn repository_workspace_delete_nulls_tab_and_window_workspace_ids() {
    let tempdir = tempfile::tempdir().expect("tempdir should be created");
    let database_path = tempdir.path().join("warp.sqlite");
    let mut conn = setup_database(&database_path).expect("database should initialize");
    let repository = repository_row("123e4567-e89b-12d3-a456-426614174014", "/tmp/zap-set-null");
    let workspace = repository_workspace_row(
        "123e4567-e89b-12d3-a456-426614174015",
        &repository.id,
        "main",
        "/tmp/zap-set-null-main",
    );
    save_repository(&mut conn, repository).expect("repository should save");
    save_repository_workspace(&mut conn, workspace.clone()).expect("workspace should save");
    let (window_id, tab_id) = save_test_window_and_tab(&mut conn);
    diesel::update(
        crate::persistence::schema::tabs::table
            .filter(crate::persistence::schema::tabs::id.eq(tab_id)),
    )
    .set(crate::persistence::schema::tabs::repository_workspace_id.eq(Some(workspace.id.clone())))
    .execute(&mut conn)
    .expect("tab workspace should update");
    diesel::update(
        crate::persistence::schema::windows::table
            .filter(crate::persistence::schema::windows::id.eq(window_id)),
    )
    .set(
        crate::persistence::schema::windows::active_repository_workspace_id
            .eq(Some(workspace.id.clone())),
    )
    .execute(&mut conn)
    .expect("window workspace should update");

    delete_repository_workspace(&mut conn, &workspace.id).expect("workspace should delete");

    assert_eq!(
        crate::persistence::schema::tabs::table
            .filter(crate::persistence::schema::tabs::id.eq(tab_id))
            .select(crate::persistence::schema::tabs::repository_workspace_id)
            .first::<Option<String>>(&mut conn)
            .expect("tab workspace should load"),
        None
    );
    assert_eq!(
        crate::persistence::schema::windows::table
            .filter(crate::persistence::schema::windows::id.eq(window_id))
            .select(crate::persistence::schema::windows::active_repository_workspace_id)
            .first::<Option<String>>(&mut conn)
            .expect("window workspace should load"),
        None
    );
}

#[test]
fn repository_source_check_rejects_invalid_source() {
    let tempdir = tempfile::tempdir().expect("tempdir should be created");
    let database_path = tempdir.path().join("warp.sqlite");
    let mut conn = setup_database(&database_path).expect("database should initialize");
    let mut repository = repository_row(
        "123e4567-e89b-12d3-a456-426614174016",
        "/tmp/zap-invalid-source",
    );
    repository.source = "remote".to_string();

    let error = save_repository(&mut conn, repository).expect_err("invalid source should fail");

    assert!(matches!(
        error,
        DieselError::DatabaseError(DatabaseErrorKind::CheckViolation, _)
    ));
}

#[test]
fn repository_path_must_be_unique() {
    let tempdir = tempfile::tempdir().expect("tempdir should be created");
    let database_path = tempdir.path().join("warp.sqlite");
    let mut conn = setup_database(&database_path).expect("database should initialize");
    let first = repository_row(
        "123e4567-e89b-12d3-a456-426614174017",
        "/tmp/zap-unique-path",
    );
    let second = repository_row(
        "123e4567-e89b-12d3-a456-426614174018",
        "/tmp/zap-unique-path",
    );
    save_repository(&mut conn, first).expect("first repository should save");

    let error = save_repository(&mut conn, second).expect_err("duplicate path should fail");

    assert!(matches!(
        error,
        DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _)
    ));
}

#[test]
fn repository_workspace_worktree_path_must_be_unique() {
    let tempdir = tempfile::tempdir().expect("tempdir should be created");
    let database_path = tempdir.path().join("warp.sqlite");
    let mut conn = setup_database(&database_path).expect("database should initialize");
    let repository = repository_row(
        "123e4567-e89b-12d3-a456-426614174019",
        "/tmp/zap-unique-worktree",
    );
    let first = repository_workspace_row(
        "123e4567-e89b-12d3-a456-426614174020",
        &repository.id,
        "main",
        "/tmp/zap-shared-worktree-path",
    );
    let second = repository_workspace_row(
        "123e4567-e89b-12d3-a456-426614174021",
        &repository.id,
        "preview",
        "/tmp/zap-shared-worktree-path",
    );
    save_repository(&mut conn, repository).expect("repository should save");
    save_repository_workspace(&mut conn, first).expect("first workspace should save");

    let error =
        save_repository_workspace(&mut conn, second).expect_err("duplicate path should fail");

    assert_unique_violation(&error);
}

#[test]
fn repository_workspace_branch_must_be_unique_within_repository() {
    let tempdir = tempfile::tempdir().expect("tempdir should be created");
    let database_path = tempdir.path().join("warp.sqlite");
    let mut conn = setup_database(&database_path).expect("database should initialize");
    let repository = repository_row(
        "123e4567-e89b-12d3-a456-426614174022",
        "/tmp/zap-unique-branch",
    );
    let first = repository_workspace_row(
        "123e4567-e89b-12d3-a456-426614174023",
        &repository.id,
        "main",
        "/tmp/zap-unique-branch-main",
    );
    let second = repository_workspace_row(
        "123e4567-e89b-12d3-a456-426614174024",
        &repository.id,
        "main",
        "/tmp/zap-unique-branch-main-copy",
    );
    save_repository(&mut conn, repository).expect("repository should save");
    save_repository_workspace(&mut conn, first).expect("first workspace should save");

    let error =
        save_repository_workspace(&mut conn, second).expect_err("duplicate branch should fail");

    assert_unique_violation(&error);
}

#[test]
fn test_sqlite_round_trips_custom_vertical_tabs_title() {
    let tempdir = tempfile::tempdir().expect("tempdir should be created");
    let database_path = tempdir.path().join("warp.sqlite");
    let mut conn = setup_database(&database_path).expect("database should initialize");

    let app_state = AppState {
        windows: vec![WindowSnapshot {
            tabs: vec![TabSnapshot {
                repository_workspace_id: None,
                custom_title: None,
                root: PaneNodeSnapshot::Leaf(LeafSnapshot {
                    is_focused: true,
                    custom_vertical_tabs_title: Some("Production API".to_string()),
                    contents: LeafContents::Terminal(TerminalPaneSnapshot {
                        uuid: vec![42],
                        cwd: Some("/tmp".to_string()),
                        shell_launch_data: Some(ShellLaunchData::Executable {
                            executable_path: PathBuf::from("/bin/zsh"),
                            shell_type: crate::terminal::shell::ShellType::Zsh,
                        }),
                        is_active: true,
                        is_read_only: false,
                        input_config: None,
                        llm_model_override: None,
                        active_profile_id: None,
                        conversation_ids_to_restore: vec![],
                        active_conversation_id: None,
                        cli_agent_resume: None,
                    }),
                }),
                default_directory_color: None,
                selected_color: SelectedTabColor::default(),
                left_panel: None,
                right_panel: None,
            }],
            active_tab_index: 0,
            active_repository_workspace_id: None,
            repository_workspace_states: Vec::new(),
            bounds: None,
            fullscreen_state: Default::default(),
            quake_mode: false,
            universal_search_width: None,
            warp_ai_width: None,
            voltron_width: None,
            warp_drive_index_width: None,
            left_panel_open: false,
            vertical_tabs_panel_open: false,
            left_panel_width: None,
            right_panel_width: None,
            agent_management_filters: None,
            theme_override: None,
        }],
        active_window_index: Some(0),
        block_lists: Default::default(),
        running_mcp_servers: Default::default(),
    };

    save_app_state(&mut conn, &app_state).expect("app state should save");

    let restored = read_sqlite_data(&mut conn, None)
        .expect("app state should load")
        .app_state;

    let PaneNodeSnapshot::Leaf(LeafSnapshot {
        custom_vertical_tabs_title,
        ..
    }) = &restored.windows[0].tabs[0].root
    else {
        panic!("Expected terminal pane leaf");
    };
    assert_eq!(
        custom_vertical_tabs_title.as_deref(),
        Some("Production API")
    );
}

#[test]
fn test_sqlite_round_trips_cli_agent_resume() {
    let tempdir = tempfile::tempdir().expect("tempdir should be created");
    let database_path = tempdir.path().join("warp.sqlite");
    let mut conn = setup_database(&database_path).expect("database should initialize");

    let resume = CliAgentResumeSnapshot {
        agent: CLIAgent::Claude,
        session_id: Some("abc-123".to_owned()),
        original_command: Some("claude --dangerously-skip-permissions".to_owned()),
    };
    let app_state = AppState {
        windows: vec![WindowSnapshot {
            tabs: vec![TabSnapshot {
                repository_workspace_id: None,
                custom_title: None,
                root: PaneNodeSnapshot::Leaf(LeafSnapshot {
                    is_focused: true,
                    custom_vertical_tabs_title: None,
                    contents: LeafContents::Terminal(TerminalPaneSnapshot {
                        uuid: vec![7],
                        cwd: Some("/tmp/proj".to_string()),
                        shell_launch_data: None,
                        is_active: true,
                        is_read_only: false,
                        input_config: None,
                        llm_model_override: None,
                        active_profile_id: None,
                        conversation_ids_to_restore: vec![],
                        active_conversation_id: None,
                        cli_agent_resume: Some(resume.clone()),
                    }),
                }),
                default_directory_color: None,
                selected_color: SelectedTabColor::default(),
                left_panel: None,
                right_panel: None,
            }],
            active_tab_index: 0,
            active_repository_workspace_id: None,
            repository_workspace_states: Vec::new(),
            bounds: None,
            fullscreen_state: Default::default(),
            quake_mode: false,
            universal_search_width: None,
            warp_ai_width: None,
            voltron_width: None,
            warp_drive_index_width: None,
            left_panel_open: false,
            vertical_tabs_panel_open: false,
            left_panel_width: None,
            right_panel_width: None,
            agent_management_filters: None,
            theme_override: None,
        }],
        active_window_index: Some(0),
        block_lists: Default::default(),
        running_mcp_servers: Default::default(),
    };

    save_app_state(&mut conn, &app_state).expect("app state should save");

    let restored = read_sqlite_data(&mut conn, None)
        .expect("app state should load")
        .app_state;

    let PaneNodeSnapshot::Leaf(LeafSnapshot {
        contents: LeafContents::Terminal(terminal),
        ..
    }) = &restored.windows[0].tabs[0].root
    else {
        panic!("Expected terminal pane leaf");
    };
    assert_eq!(terminal.cli_agent_resume.as_ref(), Some(&resume));
    assert_eq!(
        terminal
            .cli_agent_resume
            .as_ref()
            .and_then(|snapshot| snapshot.resume_command())
            .as_deref(),
        Some("claude --dangerously-skip-permissions --resume abc-123"),
    );
}

#[test]
fn test_sqlite_round_trips_code_pane_with_multiple_tabs() {
    let tempdir = tempfile::tempdir().expect("tempdir should be created");
    let database_path = tempdir.path().join("warp.sqlite");
    let mut conn = setup_database(&database_path).expect("database should initialize");

    let app_state = AppState {
        windows: vec![WindowSnapshot {
            tabs: vec![TabSnapshot {
                repository_workspace_id: None,
                custom_title: None,
                root: PaneNodeSnapshot::Leaf(LeafSnapshot {
                    is_focused: true,
                    custom_vertical_tabs_title: None,
                    contents: LeafContents::Code(CodePaneSnapShot::Local {
                        tabs: vec![
                            CodePaneTabSnapshot {
                                path: Some(PathBuf::from("/tmp/main.rs")),
                            },
                            CodePaneTabSnapshot {
                                path: Some(PathBuf::from("/tmp/lib.rs")),
                            },
                            CodePaneTabSnapshot { path: None },
                        ],
                        active_tab_index: 1,
                        source: Some(CodeSource::FileTree {
                            path: PathBuf::from("/tmp/main.rs"),
                        }),
                    }),
                }),
                default_directory_color: None,
                selected_color: SelectedTabColor::default(),
                left_panel: None,
                right_panel: None,
            }],
            active_tab_index: 0,
            active_repository_workspace_id: None,
            repository_workspace_states: Vec::new(),
            bounds: None,
            fullscreen_state: Default::default(),
            quake_mode: false,
            universal_search_width: None,
            warp_ai_width: None,
            voltron_width: None,
            warp_drive_index_width: None,
            left_panel_open: false,
            vertical_tabs_panel_open: false,
            left_panel_width: None,
            right_panel_width: None,
            agent_management_filters: None,
            theme_override: None,
        }],
        active_window_index: Some(0),
        block_lists: Default::default(),
        running_mcp_servers: Default::default(),
    };

    save_app_state(&mut conn, &app_state).expect("app state should save");

    let restored = read_sqlite_data(&mut conn, None)
        .expect("app state should load")
        .app_state;

    assert_eq!(restored.windows.len(), 1);
    let restored_tab = &restored.windows[0].tabs[0];
    let PaneNodeSnapshot::Leaf(LeafSnapshot {
        contents:
            LeafContents::Code(CodePaneSnapShot::Local {
                tabs,
                active_tab_index,
                source,
            }),
        ..
    }) = &restored_tab.root
    else {
        panic!("Expected code pane leaf");
    };

    assert_eq!(tabs.len(), 3);
    assert_eq!(*active_tab_index, 1);
    assert_eq!(tabs[0].path, Some(PathBuf::from("/tmp/main.rs")));
    assert_eq!(tabs[1].path, Some(PathBuf::from("/tmp/lib.rs")));
    assert_eq!(tabs[2].path, None);
    assert!(matches!(source, Some(CodeSource::FileTree { .. })));
}

fn assert_encode_then_decode_preserves_original_path(original_path: PathBuf) {
    let bytes = encode_path(original_path.clone());
    let decoded_path = decode_path(bytes);
    assert_eq!(original_path, decoded_path);
}

/// Test that a local path can be encoded and decoded. We use this when persisting a local
/// file path for notebooks in sqlite. We need this test because Windows `OsString`s are
/// often arbitrary sequences of 16-bit values, unlike Unix which uses sequences of 8-bit
/// values (bytes). Since `diesel::sql_types::Binary` deals with sequences of bytes (`u8`)
/// we need to perform special casting on `OsString`s on Windows.
#[test]
fn test_path_encode_decode() {
    // Empty path
    assert_encode_then_decode_preserves_original_path(PathBuf::new());

    // Windows-style paths
    assert_encode_then_decode_preserves_original_path(PathBuf::from(r"C:\windows\system32.dll"));
    assert_encode_then_decode_preserves_original_path(PathBuf::from("c:temp"));
    assert_encode_then_decode_preserves_original_path(PathBuf::from(r"\temp"));
    assert_encode_then_decode_preserves_original_path(PathBuf::from(r"\temp\emoji\🙈.txt"));
    assert_encode_then_decode_preserves_original_path(PathBuf::from(r"\temp\ñoñàscii\temp.txt"));
    assert_encode_then_decode_preserves_original_path(PathBuf::from(r"\temp\hindi\हिन्दी"));
    assert_encode_then_decode_preserves_original_path(PathBuf::from(r"\temp\cjk\狗没有耐心"));

    // Unix-style paths
    assert_encode_then_decode_preserves_original_path(PathBuf::from(
        "/home/persistence/example.sql",
    ));
    assert_encode_then_decode_preserves_original_path(PathBuf::from("./database/log.txt"));
    assert_encode_then_decode_preserves_original_path(PathBuf::from("/temp/emoji/🙈.txt"));
    assert_encode_then_decode_preserves_original_path(PathBuf::from("/temp/ñoñàscii/temp.txt"));
    assert_encode_then_decode_preserves_original_path(PathBuf::from("/temp/hindi/हिन्दी"));
    assert_encode_then_decode_preserves_original_path(PathBuf::from("/temp/cjk/狗没有耐心"));
}

#[cfg(target_os = "macos")]
#[test]
fn test_migrate_zap_app_group_sqlite_copies_newer_legacy_files() {
    use super::migrate_zap_app_group_sqlite_if_needed;

    let tempdir = tempfile::tempdir().expect("tempdir should be created");
    let legacy_dir = tempdir.path().join("legacy");
    let state_dir = tempdir.path().join("state");
    let target_db = state_dir.join("warp.sqlite");
    fs::create_dir_all(&legacy_dir).expect("legacy dir should be created");
    fs::create_dir_all(&state_dir).expect("state dir should be created");

    fs::write(&target_db, "old-target").expect("target db should be written");
    std::thread::sleep(std::time::Duration::from_secs(1));

    let legacy_db = legacy_dir.join("warp.sqlite");
    fs::write(&legacy_db, "legacy-db").expect("legacy db should be written");
    fs::write(legacy_db.with_extension("sqlite-wal"), "legacy-wal")
        .expect("legacy wal should be written");
    fs::write(legacy_db.with_extension("sqlite-shm"), "legacy-shm")
        .expect("legacy shm should be written");

    migrate_zap_app_group_sqlite_if_needed(&target_db, &legacy_dir)
        .expect("migration should succeed");

    assert_eq!(fs::read_to_string(&target_db).unwrap(), "legacy-db");
    assert_eq!(
        fs::read_to_string(target_db.with_extension("sqlite-wal")).unwrap(),
        "legacy-wal"
    );
    assert_eq!(
        fs::read_to_string(target_db.with_extension("sqlite-shm")).unwrap(),
        "legacy-shm"
    );
    assert!(state_dir.join(".zap-app-group-sqlite-migrated").exists());
}

#[cfg(target_os = "macos")]
#[test]
fn test_migrate_zap_app_group_sqlite_copies_when_legacy_wal_is_newer() {
    use super::migrate_zap_app_group_sqlite_if_needed;

    let tempdir = tempfile::tempdir().expect("tempdir should be created");
    let legacy_dir = tempdir.path().join("legacy");
    let state_dir = tempdir.path().join("state");
    let legacy_db = legacy_dir.join("warp.sqlite");
    let target_db = state_dir.join("warp.sqlite");
    fs::create_dir_all(&legacy_dir).expect("legacy dir should be created");
    fs::create_dir_all(&state_dir).expect("state dir should be created");

    fs::write(&legacy_db, "legacy-db").expect("legacy db should be written");
    std::thread::sleep(std::time::Duration::from_secs(1));
    fs::write(&target_db, "target-db").expect("target db should be written");
    std::thread::sleep(std::time::Duration::from_secs(1));
    fs::write(legacy_db.with_extension("sqlite-wal"), "legacy-wal")
        .expect("legacy wal should be written");

    migrate_zap_app_group_sqlite_if_needed(&target_db, &legacy_dir)
        .expect("migration should succeed");

    assert_eq!(fs::read_to_string(&target_db).unwrap(), "legacy-db");
    assert_eq!(
        fs::read_to_string(target_db.with_extension("sqlite-wal")).unwrap(),
        "legacy-wal"
    );
}

#[cfg(target_os = "macos")]
#[test]
fn test_migrate_zap_app_group_sqlite_marker_skips_copy() {
    use super::migrate_zap_app_group_sqlite_if_needed;

    let tempdir = tempfile::tempdir().expect("tempdir should be created");
    let legacy_dir = tempdir.path().join("legacy");
    let state_dir = tempdir.path().join("state");
    let target_db = state_dir.join("warp.sqlite");
    fs::create_dir_all(&legacy_dir).expect("legacy dir should be created");
    fs::create_dir_all(&state_dir).expect("state dir should be created");

    fs::write(legacy_dir.join("warp.sqlite"), "legacy-db").expect("legacy db should be written");
    fs::write(&target_db, "target-db").expect("target db should be written");
    fs::write(
        state_dir.join(".zap-app-group-sqlite-migrated"),
        "migrated\n",
    )
    .expect("marker should be written");

    migrate_zap_app_group_sqlite_if_needed(&target_db, &legacy_dir)
        .expect("migration should succeed");

    assert_eq!(fs::read_to_string(&target_db).unwrap(), "target-db");
}

#[test]
fn test_deserialize_corrupted_guests() {
    let _ = FeatureFlag::SharedWithMe.override_enabled(true);
    // Use a hardcoded timestamp to ensure this test works on systems with more-than-microsecond
    // precision.
    let permissions_ts_micros = 123456;
    let permissions_ts =
        ServerTimestamp::from_unix_timestamp_micros(permissions_ts_micros).unwrap();

    let db_permissions = ObjectPermissions {
        id: 42,
        object_metadata_id: 10,
        subject_type: "TEAM".to_string(),
        subject_id: Some("7".to_string()),
        subject_uid: "team_uid12345678912345".to_string(),
        permissions_last_updated_at: Some(permissions_ts_micros),
        // This is not a valid set of encoded object guests.
        object_guests: Some(vec![1, 2, 3]),
        anyone_with_link_access_level: None,
        anyone_with_link_source: None,
    };

    // The overall permissions should successfully convert, minus the object guests.
    let cloud_permissions = super::to_cloud_object_permissions(&db_permissions, None);
    assert_eq!(
        cloud_permissions,
        Some(StoredObjectPermissions {
            owner: Owner::Team {
                team_uid: crate::server::ids::ServerId::from_string_lossy("team_uid12345678912345"),
            },
            permissions_last_updated_ts: Some(permissions_ts),
            anyone_with_link: None,
            guests: vec![],
        })
    );
}

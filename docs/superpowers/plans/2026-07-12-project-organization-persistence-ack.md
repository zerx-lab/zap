# Project Organization Persistence Acknowledgement Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 消除 repository/workspace 路径恢复后的唯一性漏洞，并让领域 CRUD 仅在 SQLite writer 确认提交后更新内存和发送 UI event。

**Architecture:** `ProjectOrganizationModel` 使用统一 canonical path resolver 区分零匹配、唯一匹配和歧义。repository/workspace persistence 使用现有 SQLite writer 上的领域专用 request/response 事件；writer 返回 paused、database 或 channel 错误，模型只在 acknowledgement 成功后提交内存状态。

**Tech Stack:** Rust 2021、WarpUI Entity/ModelContext、Diesel 2.3 + SQLite、`std::sync::mpsc`、现有 persistence writer 和 Cargo tests。

---

## Execution Preconditions

- Work from `/Users/admin/project/opensource/zap/.worktrees/repository-workspaces` on `feat/repository-workspaces`.
- Read `docs/superpowers/specs/2026-07-12-project-organization-persistence-ack-design.md` before editing.
- Use `superpowers:test-driven-development`, `rust-unit-tests`, and `superpowers:verification-before-completion`.
- Do not run full-workspace `cargo fmt`; format only explicitly changed Rust files and revert any unrelated formatting immediately.
- Existing baseline warnings are out of scope.

### Task 1: Add deterministic canonical path resolution

**Files:**
- Modify: `app/src/project_organization/domain.rs`
- Modify: `app/src/project_organization/model.rs`
- Modify: `app/src/project_organization/model_tests.rs`

- [ ] **Step 1: Write failing recovered-alias ambiguity tests**

Add tests that load two distinct missing aliases, create their parent directories and shared target after model initialization, then exercise the canonical path.

```rust
#[test]
fn add_repository_rejects_ambiguous_recovered_aliases() {
    App::test((), |mut app| async move {
        let tempdir = TempDir::new().unwrap();
        let target = tempdir.path().join("repository");
        let first_alias = tempdir.path().join("first").join("..").join("repository");
        let second_alias = tempdir.path().join("second").join("..").join("repository");
        let first_id = RepositoryId::from(Uuid::from_u128(1));
        let second_id = RepositoryId::from(Uuid::from_u128(2));
        let (model, _operations) = create_model(
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
            .update(&mut app, |model, ctx| model.add_local_repository(&target, ctx))
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
```

Add the equivalent tests for:

- `touch_repository_path` with two recovered repository aliases.
- `insert_workspace` with two recovered worktree aliases.
- `update_workspace` excluding its own ID while still rejecting two other matches.

Use fixed UUIDs and assert sorted ID vectors so error output is deterministic.

- [ ] **Step 2: Run the ambiguity tests and verify RED**

Run:

```bash
cargo test -p warp --lib project_organization::model::model_tests::add_repository_rejects_ambiguous_recovered_aliases
cargo test -p warp --lib project_organization::model::model_tests::touch_repository_rejects_ambiguous_recovered_aliases
cargo test -p warp --lib project_organization::model::model_tests::insert_workspace_rejects_ambiguous_recovered_aliases
```

Expected: FAIL because `add_local_repository` creates a duplicate and `touch_repository_path` selects a `HashMap` entry rather than returning ambiguity.

- [ ] **Step 3: Add structured ambiguity errors**

Add to `ProjectOrganizationError`:

```rust
#[error("canonical repository path `{canonical_path}` matches multiple repositories: {repository_ids:?}")]
AmbiguousRepositoryPath {
    canonical_path: PathBuf,
    repository_ids: Vec<RepositoryId>,
},

#[error("canonical worktree path `{canonical_path}` matches multiple workspaces: {workspace_ids:?}")]
AmbiguousWorkspacePath {
    canonical_path: PathBuf,
    workspace_ids: Vec<RepositoryWorkspaceId>,
},
```

- [ ] **Step 4: Implement one canonical resolver per domain collection**

Add a private result type in `model.rs`:

```rust
enum CanonicalPathMatch<Id> {
    None,
    Unique(Id),
    Ambiguous(Vec<Id>),
}
```

Implement deterministic resolvers. Repository IDs must be sorted by their UUID value before returning ambiguity.

```rust
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
```

Implement the workspace equivalent using `worktree_path` and `RepositoryWorkspaceId`.

- [ ] **Step 5: Route strict mutators through the resolver**

After strict canonicalization, use the resolver in:

- `add_local_repository`
- `touch_repository_path`
- `insert_repository`
- `update_repository` with `excluded_id = Some(repository.id)`
- `insert_workspace`
- `update_workspace` with `excluded_id = Some(workspace.id)`

Required behavior:

```rust
match self.repository_match_for_canonical_path(&canonical_path, None) {
    CanonicalPathMatch::None => { /* insert */ }
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
```

`touch_repository_path` uses the unique ID, migrates its stored path/index to the canonical path, and updates `last_opened_at`. Delete the existing `HashMap::find_map` implementation.

- [ ] **Step 6: Add index cleanup and update regression tests**

Add focused tests:

- successful workspace delete allows reusing its old branch and worktree path.
- successful repository delete allows reusing its old path.
- repository path update removes the old index and rejects the new duplicate.
- workspace branch/path update removes both old indexes and rejects both new duplicates.
- duplicate repository/workspace IDs and orphan workspace insert remain fail-fast.

- [ ] **Step 7: Run Task 1 tests and check**

Run:

```bash
cargo test -p warp --lib project_organization::model::model_tests
cargo test -p warp --lib workspace::view::tests::repository_open_preflight_rejects_missing_path
cargo check -p warp
git diff --check
```

Expected: all tests pass; only baseline warnings remain.

- [ ] **Step 8: Commit Task 1**

```bash
git add app/src/project_organization/domain.rs \
        app/src/project_organization/model.rs \
        app/src/project_organization/model_tests.rs
git commit -m "fix: resolve repository path ambiguity"
```

### Task 2: Add acknowledged repository persistence requests

**Files:**
- Modify: `app/src/persistence/mod.rs`
- Modify: `app/src/persistence/sqlite.rs`
- Modify: `app/src/persistence/sqlite_tests.rs`

- [ ] **Step 1: Write failing persistence acknowledgement tests**

Add tests in `sqlite_tests.rs` for a real temporary SQLite writer:

```rust
#[test]
fn repository_persistence_acknowledges_committed_upsert() {
    let tempdir = tempfile::tempdir().unwrap();
    let database_path = tempdir.path().join("warp.sqlite");
    let conn = setup_database(&database_path).unwrap();
    let handles = start_writer(conn, database_path.clone()).unwrap();
    let persistence = RepositoryPersistence::new(Some(handles.sender.clone()));
    let repository = repository_row(
        "123e4567-e89b-12d3-a456-426614174100",
        "/tmp/ack-repository",
    );

    persistence
        .execute(RepositoryPersistenceOperation::UpsertRepository {
            repository: repository.clone(),
        })
        .unwrap();

    let mut read_conn = setup_database(&database_path).unwrap();
    assert_eq!(get_all_repositories(&mut read_conn).unwrap(), vec![repository]);
    handles.sender.send(ModelEvent::Terminate).unwrap();
    handles.handle.join().unwrap();
}
```

Add tests for:

- unique constraint failure is returned as `RepositoryPersistenceError::Database`.
- paused writer returns `RepositoryPersistenceError::Paused` and does not write.
- missing sender returns `RepositoryPersistenceError::Unavailable`.
- disconnected request channel returns `RequestDisconnected`.
- response channel disconnect returns `ResponseDisconnected` using a controlled receiver thread that drops the request responder.

- [ ] **Step 2: Run acknowledgement tests and verify RED**

Run:

```bash
cargo test -p warp --lib persistence::sqlite::tests::repository_persistence_acknowledges_committed_upsert
cargo test -p warp --lib persistence::sqlite::tests::repository_persistence_returns_database_error
cargo test -p warp --lib persistence::sqlite::tests::repository_persistence_fails_while_writer_is_paused
```

Expected: FAIL because `RepositoryPersistence`, operation/request types, response errors, and writer acknowledgement do not exist.

- [ ] **Step 3: Define operation, request, client, and errors**

In `app/src/persistence/mod.rs`, add:

```rust
#[derive(Debug)]
pub enum RepositoryPersistenceOperation {
    UpsertRepository { repository: model::Repository },
    DeleteRepository { repository_id: String },
    UpsertRepositoryWorkspace { workspace: model::RepositoryWorkspace },
    DeleteRepositoryWorkspace { workspace_id: String },
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum RepositoryPersistenceError {
    #[error("repository persistence is unavailable")]
    Unavailable,
    #[error("SQLite writer is paused")]
    Paused,
    #[error("repository persistence request channel disconnected: {details}")]
    RequestDisconnected { details: String },
    #[error("repository persistence response channel disconnected: {details}")]
    ResponseDisconnected { details: String },
    #[error("repository persistence database operation failed: {details}")]
    Database { details: String },
}

#[derive(Debug)]
pub struct RepositoryPersistenceRequest {
    pub operation: RepositoryPersistenceOperation,
    pub response: SyncSender<Result<(), RepositoryPersistenceError>>,
}

#[derive(Clone)]
pub struct RepositoryPersistence {
    sender: Option<SyncSender<ModelEvent>>,
}
```

Implement `RepositoryPersistence::new` and `execute`:

```rust
pub fn execute(
    &self,
    operation: RepositoryPersistenceOperation,
) -> Result<(), RepositoryPersistenceError> {
    let sender = self
        .sender
        .as_ref()
        .ok_or(RepositoryPersistenceError::Unavailable)?;
    let (response, receiver) = std::sync::mpsc::sync_channel(1);
    sender
        .send(ModelEvent::RepositoryPersistence(RepositoryPersistenceRequest {
            operation,
            response,
        }))
        .map_err(|error| RepositoryPersistenceError::RequestDisconnected {
            details: error.to_string(),
        })?;
    receiver
        .recv()
        .map_err(|error| RepositoryPersistenceError::ResponseDisconnected {
            details: error.to_string(),
        })?
}
```

Add the acknowledged request variant alongside the four existing repository/workspace CRUD variants:

```rust
RepositoryPersistence(RepositoryPersistenceRequest),
```

Do not remove the old variants in this task. `ProjectOrganizationModel` still uses them until Task 3, so temporary coexistence is required for this intermediate commit to compile. Task 3 removes the old variants immediately after switching the model.

- [ ] **Step 4: Add writer-side operation dispatch**

In `sqlite.rs`, add:

```rust
fn handle_repository_persistence_operation(
    operation: RepositoryPersistenceOperation,
    connection: &mut SqliteConnection,
) -> anyhow::Result<()> {
    match operation {
        RepositoryPersistenceOperation::UpsertRepository { repository } => {
            save_repository(connection, repository).context("error upserting repository")
        }
        RepositoryPersistenceOperation::DeleteRepository { repository_id } => {
            delete_repository(connection, &repository_id).context("error deleting repository")
        }
        RepositoryPersistenceOperation::UpsertRepositoryWorkspace { workspace } => {
            save_repository_workspace(connection, workspace)
                .context("error upserting repository workspace")
        }
        RepositoryPersistenceOperation::DeleteRepositoryWorkspace { workspace_id } => {
            delete_repository_workspace(connection, &workspace_id)
                .context("error deleting repository workspace")
        }
    }
}
```

Handle `ModelEvent::RepositoryPersistence(request)` in the writer loop before the generic paused-event branch:

```rust
ModelEvent::RepositoryPersistence(request) => {
    let result = if paused {
        Err(RepositoryPersistenceError::Paused)
    } else {
        handle_repository_persistence_operation(request.operation, &mut current_conn)
            .map_err(|error| RepositoryPersistenceError::Database {
                details: format!("{error:#}"),
            })
    };
    if request.response.send(result).is_err() {
        log::error!("Repository persistence requester disconnected before acknowledgement");
    }
}
```

Keep the old repository CRUD arms in `handle_model_event` for the Task 2 intermediate commit. Add `RepositoryPersistence` to its control-flow panic arm because acknowledged requests must be handled by the writer loop. Task 3 removes the old arms after all production callers move to acknowledgement.

- [ ] **Step 5: Run writer acknowledgement tests**

Run:

```bash
cargo test -p warp --lib persistence::sqlite::tests::repository_persistence
cargo test -p warp --lib persistence::sqlite::tests
cargo check -p warp
git diff --check
```

Expected: acknowledgement tests and the complete SQLite suite pass.

- [ ] **Step 6: Commit Task 2**

```bash
git add app/src/persistence/mod.rs \
        app/src/persistence/sqlite.rs \
        app/src/persistence/sqlite_tests.rs
git commit -m "feat: acknowledge repository persistence"
```

### Task 3: Require acknowledgement before model state changes

**Files:**
- Modify: `app/src/lib.rs`
- Modify: `app/src/persistence/mod.rs`
- Modify: `app/src/persistence/sqlite.rs`
- Modify: `app/src/project_organization/model.rs`
- Modify: `app/src/project_organization/model_tests.rs`

- [ ] **Step 1: Replace the model test persistence harness**

In `model_tests.rs`, replace the raw `Receiver<ModelEvent>` helper with a responder thread that records `RepositoryPersistenceOperation` and returns a configured result.

```rust
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
            operation_sender.send(request.operation).unwrap();
            request.response.send(result.clone()).unwrap();
        }
    });
    (
        RepositoryPersistence::new(Some(event_sender)),
        PersistenceHarness {
            operations: operation_receiver,
        },
    )
}
```

`create_model` accepts `RepositoryPersistence` instead of an optional sender.

- [ ] **Step 2: Write failing persistence atomicity tests**

Add tests for every mutation class. Example:

```rust
#[test]
fn repository_add_does_not_change_memory_when_persistence_fails() {
    App::test((), |mut app| async move {
        let tempdir = TempDir::new().unwrap();
        let repository_path = tempdir.path().join("repository");
        std::fs::create_dir(&repository_path).unwrap();
        let (persistence, _harness) = acknowledged_persistence(Err(
            RepositoryPersistenceError::Database {
                details: "injected failure".to_string(),
            },
        ));
        let model = create_model_with_persistence(&mut app, vec![], vec![], persistence);

        let error = model
            .update(&mut app, |model, ctx| {
                model.add_local_repository(&repository_path, ctx)
            })
            .unwrap_err();

        assert!(matches!(error, ProjectOrganizationError::Persistence { .. }));
        assert_eq!(model.read(&app, |model, _| model.repositories().count()), 0);
    });
}
```

Add equivalent tests for:

- repository update failure retains old path/index/data.
- repository delete failure retains record/path index.
- workspace insert failure creates no record/index.
- workspace update failure retains old branch/path indexes.
- workspace delete failure retains record/index.
- unavailable persistence fails without memory changes.
- successful acknowledgement emits exactly one domain event and records exactly one operation.

Use a real model subscription for the event assertion:

```rust
struct ProjectOrganizationEventProbe;

impl Entity for ProjectOrganizationEventProbe {
    type Event = ();
}

let emitted_events = Arc::new(Mutex::new(Vec::new()));
let captured_events = emitted_events.clone();
let subscribed_model = model.clone();
app.add_model(move |ctx| {
    ctx.subscribe_to_model(&subscribed_model, move |_, _, event, _| {
        captured_events.lock().unwrap().push(event.clone());
    });
    ProjectOrganizationEventProbe
});
```

After a successful mutation, assert one persistence operation and one matching `ProjectOrganizationEvent`. After a failed acknowledgement, assert the event vector remains empty.

- [ ] **Step 3: Run atomicity tests and verify RED**

Run:

```bash
cargo test -p warp --lib project_organization::model::model_tests::repository_add_does_not_change_memory_when_persistence_fails
cargo test -p warp --lib project_organization::model::model_tests::workspace_update_does_not_change_indexes_when_persistence_fails
```

Expected: FAIL because the model still treats channel enqueue as persistence success or permits missing persistence.

- [ ] **Step 4: Store `RepositoryPersistence` in the model**

Change the model field and constructor:

```rust
pub struct ProjectOrganizationModel {
    // indexes unchanged
    persistence: RepositoryPersistence,
}

pub fn try_new(
    persisted_repositories: Vec<PersistedRepository>,
    persisted_workspaces: Vec<PersistedWorkspace>,
    persistence: RepositoryPersistence,
    _ctx: &mut ModelContext<Self>,
) -> Result<Self, ProjectOrganizationError>;
```

Map acknowledgement errors without discarding context:

```rust
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
```

Replace all old `send_model_event` calls. Keep the existing validate -> persist -> memory/index -> domain event order.

After the model no longer constructs the old events:

- remove `ModelEvent::UpsertRepository`.
- remove `ModelEvent::DeleteRepository`.
- remove `ModelEvent::UpsertRepositoryWorkspace`.
- remove `ModelEvent::DeleteRepositoryWorkspace`.
- remove their four `handle_model_event` arms from `sqlite.rs`.

- [ ] **Step 5: Wire production initialization**

In `app/src/lib.rs`, construct the client from the writer sender:

```rust
let project_organization_persistence =
    persistence::RepositoryPersistence::new(persistence_writer.sender());
ctx.add_singleton_model(|ctx| {
    ProjectOrganizationModel::try_new(
        persisted_repositories,
        persisted_repository_workspaces,
        project_organization_persistence,
        ctx,
    )
    .unwrap_or_else(|error| panic!("Failed to initialize project organization: {error:#}"))
});
```

Do not replace unavailable persistence with a no-op client.

- [ ] **Step 6: Run model, persistence, and consumer tests**

Run:

```bash
cargo test -p warp --lib project_organization::model::model_tests
cargo test -p warp --lib persistence::sqlite::tests
cargo test -p warp --lib workspace::view::tests::repository_open_preflight_rejects_missing_path
cargo check -p warp
git diff --check
```

Expected: all tests pass; repository/workspace operations fail cleanly when persistence is unavailable or returns an error.

- [ ] **Step 7: Review residual event usage**

Run:

```bash
rg -n "UpsertRepository|DeleteRepository|UpsertRepositoryWorkspace|DeleteRepositoryWorkspace|RepositoryPersistence" app/src
```

Expected:

- old fire-and-forget repository CRUD variants have no remaining definitions or callers.
- repository/workspace mutations flow through `RepositoryPersistenceOperation`.
- SQLite writer is the only production acknowledgement responder.

- [ ] **Step 8: Commit Task 3**

```bash
git add app/src/lib.rs \
        app/src/persistence/mod.rs \
        app/src/persistence/sqlite.rs \
        app/src/project_organization/model.rs \
        app/src/project_organization/model_tests.rs
git commit -m "fix: commit repository state after persistence"
```

### Task 4: Final verification and Task 3 review closure

**Files:**
- Modify only files required by compiler or test feedback.

- [ ] **Step 1: Run focused verification**

```bash
cargo test -p warp --lib project_organization::model::model_tests
cargo test -p warp --lib persistence::sqlite::tests
cargo test -p warp --lib workspace::view::tests::repository_open_preflight_rejects_missing_path
cargo check -p warp
git diff --check
```

- [ ] **Step 2: Verify worktree scope**

```bash
git status --short
git diff --stat f13c23333657ab705365ace2fb3b6850c46543de..HEAD
```

Expected: no uncommitted changes; only repository-workspace plan/design and implementation files are changed.

- [ ] **Step 3: Re-run Task 3 spec and quality reviews**

Use `superpowers:requesting-code-review` with the complete Task 3 range. Confirm:

- canonical 0/1/many resolution is deterministic.
- acknowledgement covers unavailable, paused, disconnected, and database failure.
- model failure paths leave memory/indexes/events unchanged.
- Task 4 can use acknowledged workspace insert/delete as compensation boundaries.

- [ ] **Step 4: Commit review fixes if required**

Use a scoped English commit message describing only the reviewed fix. Do not squash or rewrite the existing task history unless explicitly requested.

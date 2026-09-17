# Local Repository Workspace Implementation Plan

> For agentic workers: REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox syntax for tracking.

Goal: Make the repository primary worktree available as a local workspace, create it automatically when a repository is added through the repository picker, and expose the same operation in the existing Use existing worktree workspace modal.

Architecture: Git validation returns the primary worktree current local branch and marks primary options explicitly. The project-organization model persists a new repository and its initial workspace through one SQLite transaction. The existing-worktree modal reuses the normal validation flow and skips git worktree add for the repository root. Workspace activation and the initial repository-root terminal use one shared helper.

Tech Stack: Rust, Cargo workspace, Git CLI wrapper, WarpUI, Diesel/SQLite persistence, cargo nextest.

---

## File map

- Modify app/src/project_organization/git.rs and git_tests.rs for primary metadata, validation, and detached-HEAD behavior.
- Modify app/src/project_organization/view/create_workspace_modal.rs and its tests for primary option display, local default naming, and warnings.
- Modify app/src/persistence/mod.rs, sqlite.rs, and sqlite_tests.rs for atomic repository-plus-workspace persistence.
- Modify app/src/project_organization/model.rs and model_tests.rs for initial workspace construction, memory commit, and events.
- Modify app/src/workspace/view.rs for repository-add wiring and shared activation.
- Modify app/src/workspace/view_test.rs only if the existing harness can isolate activation without unrelated setup.

## Task 1: Extend Git validation to understand the primary worktree

Files: app/src/project_organization/git.rs, app/src/project_organization/git_tests.rs

- [ ] Step 1: Write failing tests.

Add tests asserting that the primary root is included and marked, primary adoption returns its canonical path, validated repositories expose primary_branch == "main", and detached primary worktrees return GitWorkspaceError::PrimaryWorktreeDetached. Keep linked detached/prunable entries excluded.

    #[test]
    fn validates_the_primary_worktree_for_workspace_adoption() {
        let fixture = GitFixture::new();
        assert_eq!(
            validate_existing_worktree(&fixture.root, &fixture.root, "main").unwrap(),
            fixture.root.canonicalize().unwrap(),
        );
    }

    #[test]
    fn rejects_detached_primary_worktree() {
        let fixture = GitFixture::new();
        run_git(&fixture.root, &["checkout", "--detach", "HEAD"]);
        assert!(matches!(
            validate_repository(&fixture.root).unwrap_err(),
            GitWorkspaceError::PrimaryWorktreeDetached { path }
                if path == fixture.root.canonicalize().unwrap()
        ));
    }

- [ ] Step 2: Verify the tests fail for the missing behavior.

    cargo nextest run -p warp -E 'test(validates_the_primary_worktree_for_workspace_adoption) or test(rejects_detached_primary_worktree)'

Expected: failure because primary worktrees are currently filtered/rejected and ValidatedRepository has no primary branch.

- [ ] Step 3: Implement the Git changes.

Add primary_branch: String to ValidatedRepository and is_primary: bool to ExistingWorktreeOption. Keep ExistingWorktreeOption::new for linked worktrees and add a primary constructor:

    pub fn primary(path: PathBuf, branch_name: impl Into<String>) -> Self {
        Self {
            path,
            branch_name: branch_name.into(),
            is_primary: true,
        }
    }

In validate_repository, inspect list_worktrees(&root), find the root entry, reject a missing, detached, or empty branch with PrimaryWorktreeDetached { path }, and store the refs/heads/ short name. In existing_worktree_options, include the root entry as ExistingWorktreeOption::primary(...); continue excluding bare, detached, prunable, malformed, and empty-branch entries. Sort primary first, then branch name and path. In validate_existing_worktree, remove the unconditional PrimaryWorktreeCannotBeWorkspace rejection while retaining registration, prunable, branch, and ref checks.

- [ ] Step 4: Run tests, format, and commit.

    cargo fmt -- app/src/project_organization/git.rs app/src/project_organization/git_tests.rs
    cargo nextest run -p warp -E 'test(existing_worktree_options) or test(validates_registered_existing_worktree) or test(validate_repository)'
    git add app/src/project_organization/git.rs app/src/project_organization/git_tests.rs
    git commit -m "feat: recognize repository primary worktrees"

Expected: all selected Git tests pass.

## Task 2: Add the primary worktree to the existing-worktree modal

Files: app/src/project_organization/view/create_workspace_modal.rs and app/src/project_organization/view/create_workspace_modal_tests.rs

- [ ] Step 1: Write a failing modal test.

Add pure helpers and test the required label/default behavior:

    #[test]
    fn primary_existing_worktree_uses_local_label_and_name() {
        let option = ExistingWorktreeOption::primary(PathBuf::from("/repo"), "main");
        assert_eq!(existing_worktree_display_label(&option), "main (local)");
        assert_eq!(existing_worktree_default_name(&option), "local");
    }

Keep the existing request test asserting that the actual branch remains feature/adopt and the selected path is passed through unchanged.

- [ ] Step 2: Verify the modal test fails.

    cargo nextest run -p warp -E 'test(primary_existing_worktree_uses_local_label_and_name)'

Expected: compilation failure because the primary marker and helpers do not exist.

- [ ] Step 3: Implement modal behavior.

Add focused helpers:

    fn existing_worktree_display_label(worktree: &ExistingWorktreeOption) -> String {
        if worktree.is_primary {
            format!("{} (local)", worktree.branch_name)
        } else {
            worktree.branch_name.clone()
        }
    }

    fn existing_worktree_default_name(worktree: &ExistingWorktreeOption) -> &str {
        if worktree.is_primary {
            "local"
        } else {
            &worktree.branch_name
        }
    }

Use the label when building dropdown items. In select_existing_worktree, keep the real branch in CreateWorkspaceForm but reset the name editor from existing_worktree_default_name. Add primary_worktree_error: Option<String>; clear it on configure, close, and retry, and set it when the repository-root WorktreeInfo is detached. Render the warning in the existing-worktree section while still allowing valid linked worktrees to be selected. Do not use existing_worktree_fetch_error for this warning because a detached primary must not disable unrelated valid options. Leave CreateWorkspaceSource::ExistingWorktree { local_branch } unchanged; its selected path already flows into CreateWorkspaceRequest.

- [ ] Step 4: Run tests and commit.

    cargo fmt -- app/src/project_organization/view/create_workspace_modal.rs app/src/project_organization/view/create_workspace_modal_tests.rs
    cargo nextest run -p warp -E 'test(primary_existing_worktree) or test(existing_worktree_form_builds_a_workspace_creation_request) or test(existing_worktree_submit_is_disabled_until_a_selection_is_available)'
    git add app/src/project_organization/view/create_workspace_modal.rs app/src/project_organization/view/create_workspace_modal_tests.rs
    git commit -m "feat: expose primary worktree in workspace modal"

Expected: primary displays as main (local), defaults to local, and retains main as its branch.

## Task 3: Persist repository and initial local workspace atomically

Files: app/src/persistence/mod.rs, app/src/persistence/sqlite.rs, app/src/persistence/sqlite_tests.rs, app/src/project_organization/model.rs, app/src/project_organization/model_tests.rs

- [ ] Step 1: Write failing persistence tests.

Add UpsertRepositoryWithWorkspace tests. The success case must read both rows after acknowledgement. The failure case must create a path conflict in the workspace row, execute the paired operation, assert a database error, and assert that the new repository row was rolled back.

    #[test]
    fn failed_initial_workspace_upsert_rolls_back_the_new_repository() {
        // Insert a conflicting workspace, execute the paired upsert, then query SQLite.
        // The new repository must not exist after the workspace write fails.
    }

- [ ] Step 2: Verify persistence tests fail.

    cargo nextest run -p warp -E 'test(repository_and_initial_workspace_are_persisted_as_one_transaction) or test(failed_initial_workspace_upsert_rolls_back_the_new_repository)'

Expected: compilation failure because the paired operation is missing.

- [ ] Step 3: Implement the SQLite operation.

Add this enum variant in RepositoryPersistenceOperation:

    UpsertRepositoryWithWorkspace {
        repository: model::Repository,
        workspace: model::RepositoryWorkspace,
    },

Handle it with the existing Diesel connection:

    connection.immediate_transaction(|connection| {
        save_repository(connection, repository)?;
        save_repository_workspace(connection, workspace)?;
        Ok::<_, anyhow::Error>(())
    })

Keep existing single-row variants unchanged and preserve their error context.

- [ ] Step 4: Write failing model tests.

Using the current acknowledged_persistence harness, test a new method returning both IDs. Assert one paired persistence operation, repository/workspace memory entries, display_name == "local", branch == "main", root worktree_path, and RepositoryAdded then WorkspaceAdded events. Add a persistence-failure test asserting no memory entries/events, and retain duplicate branch/path tests for existing repositories.

    let (repository_id, workspace_id) = model.update(&mut app, |model, ctx| {
        model.add_local_repository_with_initial_workspace(
            &repository_path,
            None,
            "main",
            ctx,
        )
    }).unwrap();

- [ ] Step 5: Verify model tests fail.

    cargo nextest run -p warp -E 'test(adding_repository_with_initial_workspace) or test(repository_with_initial_workspace_does_not_change_memory_when_persistence_fails)'

Expected: compilation failure because the model method is missing.

- [ ] Step 6: Implement the model method.

Add:

    pub fn add_local_repository_with_initial_workspace(
        &mut self,
        path: impl AsRef<Path>,
        remote_url: Option<String>,
        primary_branch: impl Into<String>,
        ctx: &mut ModelContext<Self>,
    ) -> Result<(RepositoryId, RepositoryWorkspaceId), ProjectOrganizationError>

Reuse current canonical path/display-name validation. Construct a RepositoryWorkspace with name local, the supplied actual branch, repository root path, and a fresh ID. Validate repository and workspace uniqueness against the pending repository ID without mutating in-memory maps. Execute UpsertRepositoryWithWorkspace; only after success call commit_repository, commit_workspace, emit both events, and return both IDs. Leave low-level add_local_repository and touch_repository_path APIs unchanged because they do not receive validated branch metadata.

- [ ] Step 7: Run tests, format, and commit.

    cargo fmt -- app/src/persistence/mod.rs app/src/persistence/sqlite.rs app/src/persistence/sqlite_tests.rs app/src/project_organization/model.rs app/src/project_organization/model_tests.rs
    cargo nextest run -p warp -E 'test(project_organization) or test(repository_persistence)'
    git add app/src/persistence/mod.rs app/src/persistence/sqlite.rs app/src/persistence/sqlite_tests.rs app/src/project_organization/model.rs app/src/project_organization/model_tests.rs
    git commit -m "feat: persist initial local repository workspace"

Expected: paired persistence is atomic and the model never commits partial state.

## Task 4: Wire repository addition and shared workspace activation

File: app/src/workspace/view.rs

- [ ] Step 1: Extract the successful activation behavior.

Create a private helper used by both flows:

    fn activate_repository_workspace(
        &mut self,
        workspace_id: RepositoryWorkspaceId,
        initial_directory: PathBuf,
        ctx: &mut ViewContext<Self>,
    ) {
        self.switch_repository_workspace(Some(workspace_id), ctx);
        self.add_tab_with_pane_layout(
            PanesLayout::SingleTerminal(Box::new(
                NewTerminalOptions::default()
                    .with_initial_directory(initial_directory),
            )),
            Arc::new(HashMap::new()),
            None,
            ctx,
        );
    }

If app/src/workspace/view_test.rs can isolate this behavior, add an assertion for the active workspace ID and terminal tab initial directory before implementation. Otherwise rely on the manual assertions in Task 5 and avoid mock-heavy test setup.

- [ ] Step 2: Wire the repository picker.

In validate_and_add_repository, pass repository.primary_branch to add_local_repository_with_initial_workspace. On success, activate the returned workspace with repository.root. Detached-primary and persistence errors must use the existing toast paths and must not activate a workspace.

- [ ] Step 3: Reuse activation for ordinary workspace creation.

Replace the successful tail of create_repository_workspace with activate_repository_workspace(request.workspace_id, request.worktree_path, ctx). Keep source_creates_worktree unchanged: ExistingWorktree, including the primary root, never triggers cleanup; remote/local branch creation still does.

- [ ] Step 4: Compile, test, format, and commit.

    cargo fmt -- app/src/workspace/view.rs app/src/workspace/view_test.rs
    cargo nextest run -p warp -E 'test(project_organization) or test(repository_workspace) or test(workspace)'
    cargo check -p warp
    git add app/src/workspace/view.rs app/src/workspace/view_test.rs
    git commit -m "feat: activate local workspace after repository add"

Expected: ordinary workspace creation remains green and repository addition now creates/activates local.

## Task 5: Full verification and macOS app bundle

- [ ] Step 1: Run focused regression tests and compile check.

    cargo nextest run -p warp -E 'test(project_organization) or test(repository_workspace) or test(sqlite)'
    cargo check -p warp

- [ ] Step 2: Build the macOS app.

    ./script/bundle --debug --selfsign --nouniversal --channel local

Expected: a fresh Zap.app exists under target, normally target/debug/bundle/osx/Zap.app.

- [ ] Step 3: Verify the artifact.

    test -d target/debug/bundle/osx/Zap.app
    /usr/libexec/PlistBuddy -c 'Print :CFBundleExecutable' target/debug/bundle/osx/Zap.app/Contents/Info.plist
    codesign --verify --deep --strict --verbose=2 target/debug/bundle/osx/Zap.app

- [ ] Step 4: Manually validate acceptance criteria.

1. Add a normal checkout repository: repository and local appear, the app switches to local, and a terminal opens at the repository root.
2. For an existing repository, open the current create-workspace modal, choose Use existing worktree, select main (local), and verify name local, actual branch main, root path, and no new linked worktree directory.
3. Repeat the operation and verify a clear duplicate error with no state change.
4. Detach the primary worktree and verify the modal shows a clear warning; valid linked worktrees remain selectable.

- [ ] Step 5: Review the final diff and status.

    git diff --check HEAD~4..HEAD
    git status --short
    git log --oneline -6

Expected: no whitespace errors and a clean worktree after the implementation commits.


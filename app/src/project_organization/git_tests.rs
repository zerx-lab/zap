use std::path::{Path, PathBuf};

use super::git::*;

struct GitFixture {
    tempdir: tempfile::TempDir,
    root: PathBuf,
    remote: PathBuf,
}

impl GitFixture {
    fn new() -> Self {
        let tempdir = tempfile::tempdir().unwrap();
        let root = tempdir.path().join("repo with 'quote");
        std::fs::create_dir(&root).unwrap();
        run_git(&root, &["init", "-b", "main"]);
        std::fs::write(root.join("README.md"), "fixture").unwrap();
        run_git(&root, &["add", "README.md"]);
        run_git(
            &root,
            &[
                "-c",
                "user.name=Zap Tests",
                "-c",
                "user.email=zap@example.com",
                "commit",
                "-m",
                "init",
            ],
        );

        let remote = tempdir.path().join("remote repository.git");
        let remote_str = remote.to_str().unwrap();
        run_git(
            tempdir.path(),
            &["init", "--bare", "-b", "main", remote_str],
        );
        run_git(&root, &["remote", "add", "origin", remote_str]);
        run_git(&root, &["push", "-u", "origin", "main"]);
        run_git(&root, &["remote", "set-head", "origin", "-a"]);

        Self {
            tempdir,
            root,
            remote,
        }
    }

    fn add_linked_worktree(&self, branch: &str) -> PathBuf {
        let path = self
            .tempdir
            .path()
            .join(format!("worktree {} 'quoted'", branch.replace('/', "-")));
        self.add_linked_worktree_at(branch, &path);
        path
    }

    fn add_linked_worktree_at(&self, branch: &str, path: &Path) {
        let output = command::blocking::Command::new("git")
            .arg("-C")
            .arg(&self.root)
            .args(["worktree", "add", "-b", branch])
            .arg(path)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "failed to add worktree for {branch}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

struct RelativeWorktreeCleanup {
    repository: PathBuf,
    worktree_path: PathBuf,
}

impl Drop for RelativeWorktreeCleanup {
    fn drop(&mut self) {
        if !self.worktree_path.exists() {
            return;
        }
        match command::blocking::Command::new("git")
            .arg("-C")
            .arg(&self.repository)
            .args(["worktree", "remove"])
            .arg(&self.worktree_path)
            .status()
        {
            Ok(status) if status.success() => {}
            Ok(status) => eprintln!(
                "failed to clean relative test worktree `{}`: {status}",
                self.worktree_path.display()
            ),
            Err(error) => eprintln!(
                "failed to run cleanup for relative test worktree `{}`: {error}",
                self.worktree_path.display()
            ),
        }
    }
}

fn run_git(cwd: &Path, args: &[&str]) {
    let status = command::blocking::Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args(args)
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?} failed");
}

fn git_output(cwd: &Path, args: &[&str]) -> String {
    let output = command::blocking::Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_string()
}

fn current_branch(worktree: &Path) -> String {
    git_output(worktree, &["branch", "--show-current"])
}

fn branch_upstream(repository: &Path, branch: &str) -> Option<String> {
    let full_ref = format!("refs/heads/{branch}");
    let output = git_output(
        repository,
        &["for-each-ref", "--format=%(upstream)", &full_ref],
    );
    (!output.is_empty()).then_some(output)
}

fn ref_exists(repository: &Path, full_ref: &str) -> bool {
    command::blocking::Command::new("git")
        .arg("-C")
        .arg(repository)
        .args(["show-ref", "--verify", "--quiet", full_ref])
        .status()
        .unwrap()
        .success()
}

fn ref_oid(repository: &Path, full_ref: &str) -> String {
    git_output(repository, &["rev-parse", "--verify", full_ref])
}

fn fixture_commit_oid(repository: &Path, message: &str) -> String {
    git_output(
        repository,
        &[
            "-c",
            "user.name=Zap Tests",
            "-c",
            "user.email=zap@example.com",
            "commit-tree",
            "HEAD^{tree}",
            "-p",
            "HEAD",
            "-m",
            message,
        ],
    )
}

fn advance_remote_main(fixture: &GitFixture) {
    let full_ref = "refs/remotes/origin/main";
    let old_oid = ref_oid(&fixture.root, full_ref);
    let new_oid = fixture_commit_oid(&fixture.root, "advanced remote main");
    run_git(&fixture.root, &["update-ref", full_ref, &new_oid, &old_oid]);
}

fn set_gone_upstream(fixture: &GitFixture, branch: &str) {
    let remote_ref = format!("refs/remotes/origin/{branch}");
    run_git(&fixture.root, &["push", "-u", "origin", branch]);
    assert!(ref_exists(&fixture.root, &remote_ref));
    run_git(&fixture.root, &["update-ref", "-d", &remote_ref]);
    assert!(!ref_exists(&fixture.root, &remote_ref));
    assert_eq!(
        branch_upstream(&fixture.root, branch).as_deref(),
        Some(remote_ref.as_str())
    );
}

fn injected_command_error(operation: &'static str) -> GitWorkspaceError {
    GitWorkspaceError::CommandFailed {
        operation,
        args: vec!["worktree".to_string(), "add".to_string()],
        stderr: "injected creation failure".to_string(),
    }
}

#[test]
fn rejects_linked_worktree_as_repository() {
    let fixture = GitFixture::new();
    let worktree_path = fixture.add_linked_worktree("feature/a");

    let error = validate_repository(&worktree_path).unwrap_err();

    assert!(matches!(error, GitWorkspaceError::LinkedWorktree { .. }));
}

#[test]
fn rejects_directory_below_repository_root() {
    let fixture = GitFixture::new();
    let nested = fixture.root.join("nested");
    std::fs::create_dir(&nested).unwrap();

    let error = validate_repository(&nested).unwrap_err();

    assert!(matches!(error, GitWorkspaceError::NotRepositoryRoot { .. }));
}

#[test]
fn validates_repository_and_reads_remote_metadata() {
    let fixture = GitFixture::new();

    let repository = validate_repository(&fixture.root).unwrap();

    assert_eq!(repository.root, fixture.root.canonicalize().unwrap());
    assert_eq!(repository.primary_branch, "main");
    assert_eq!(repository.remote, "origin");
    assert_eq!(repository.remote_url, fixture.remote.to_str().unwrap());
    assert!(matches!(
        repository.default_branch,
        BranchRef::Remote {
            remote,
            name,
            full_ref
        } if remote == "origin" && name == "main" && full_ref == "refs/remotes/origin/main"
    ));
}

#[cfg(unix)]
#[test]
fn decodes_non_utf8_git_path_output_without_loss() {
    use std::os::unix::ffi::{OsStrExt, OsStringExt};

    let expected = PathBuf::from(std::ffi::OsString::from_vec(
        b"/tmp/repository-\xff ".to_vec(),
    ));
    let mut output = expected.as_os_str().as_bytes().to_vec();
    output.push(b'\n');

    let decoded = decode_git_path_output(&output, "decode test path").unwrap();

    assert_eq!(
        decoded.as_os_str().as_bytes(),
        expected.as_os_str().as_bytes()
    );
}

#[cfg(unix)]
#[test]
fn preserves_trailing_carriage_return_in_git_path_output() {
    use std::os::unix::ffi::{OsStrExt, OsStringExt};

    let expected = PathBuf::from(std::ffi::OsString::from_vec(
        b"/tmp/repository-with-trailing-cr\r".to_vec(),
    ));
    let mut output = expected.as_os_str().as_bytes().to_vec();
    output.push(b'\n');

    let decoded = decode_git_path_output(&output, "decode test path").unwrap();

    assert_eq!(
        decoded.as_os_str().as_bytes(),
        expected.as_os_str().as_bytes()
    );
}

#[test]
fn rejects_repository_without_remote() {
    let fixture = GitFixture::new();
    run_git(&fixture.root, &["remote", "remove", "origin"]);

    let error = validate_repository(&fixture.root).unwrap_err();

    assert!(matches!(error, GitWorkspaceError::RemoteNotFound { .. }));
}

#[test]
fn rejects_repository_without_remote_default_branch() {
    let fixture = GitFixture::new();
    run_git(
        &fixture.root,
        &["symbolic-ref", "--delete", "refs/remotes/origin/HEAD"],
    );

    let error = validate_repository(&fixture.root).unwrap_err();

    assert!(matches!(
        error,
        GitWorkspaceError::DefaultBranchNotFound { remote, .. } if remote == "origin"
    ));
}

#[test]
fn selects_first_remote_when_origin_is_absent() {
    let fixture = GitFixture::new();
    let remote_url = fixture.remote.to_str().unwrap();
    run_git(&fixture.root, &["remote", "remove", "origin"]);
    run_git(&fixture.root, &["remote", "add", "a", remote_url]);
    run_git(
        &fixture.root,
        &["remote", "add", "zzzz-longer-remote", remote_url],
    );
    run_git(&fixture.root, &["fetch", "a"]);
    run_git(&fixture.root, &["remote", "set-head", "a", "-a"]);

    let repository = validate_repository(&fixture.root).unwrap();

    assert_eq!(repository.remote, "a");
    assert!(matches!(
        repository.default_branch,
        BranchRef::Remote { remote, name, .. } if remote == "a" && name == "main"
    ));
}

#[test]
fn classifies_local_and_remote_refs_without_prefix_guessing() {
    let fixture = GitFixture::new();
    run_git(&fixture.root, &["branch", "origin/foo"]);
    run_git(
        &fixture.root,
        &["push", "origin", "main:refs/heads/team/remote-branch"],
    );
    run_git(&fixture.root, &["fetch", "origin"]);

    let refs = list_branch_refs(&fixture.root).unwrap();

    assert!(refs.iter().any(|branch_ref| matches!(
        branch_ref,
        BranchRef::Local { name, full_ref }
            if name == "origin/foo" && full_ref == "refs/heads/origin/foo"
    )));
    assert!(refs.iter().any(|branch_ref| matches!(
        branch_ref,
        BranchRef::Remote {
            remote,
            name,
            full_ref
        } if remote == "origin"
            && name == "team/remote-branch"
            && full_ref == "refs/remotes/origin/team/remote-branch"
    )));
}

#[test]
fn rejects_ambiguous_overlapping_remote_ref() {
    let remotes = vec!["foo".to_string(), "foo/bar".to_string()];

    let error = parse_branch_ref("refs/remotes/foo/bar/main", &remotes).unwrap_err();

    assert!(matches!(
        error,
        GitWorkspaceError::AmbiguousRemoteRef {
            full_ref,
            remotes: candidates,
        } if full_ref == "refs/remotes/foo/bar/main" && candidates == remotes
    ));
}

#[test]
fn rejects_direct_head_ref_ambiguous_between_overlapping_remotes() {
    let fixture = GitFixture::new();
    let remote_url = fixture.remote.to_str().unwrap();
    run_git(&fixture.root, &["remote", "add", "foo", remote_url]);
    run_git(&fixture.root, &["remote", "add", "foo/bar", remote_url]);
    run_git(
        &fixture.root,
        &["update-ref", "refs/remotes/foo/bar/HEAD", "HEAD"],
    );

    let error = list_branch_refs(&fixture.root).unwrap_err();

    assert!(matches!(
        error,
        GitWorkspaceError::AmbiguousRemoteRef {
            full_ref,
            remotes,
        } if full_ref == "refs/remotes/foo/bar/HEAD"
            && remotes == ["foo".to_string(), "foo/bar".to_string()]
    ));
}

#[test]
fn rejects_malformed_branch_ref_record() {
    let error = parse_branch_ref_records("refs/heads/main\n", &[]).unwrap_err();

    assert!(matches!(
        error,
        GitWorkspaceError::InvalidBranchRefRecord { record }
            if record == "refs/heads/main"
    ));
}

#[test]
fn fetches_remote_refs_before_listing_them() {
    let fixture = GitFixture::new();
    run_git(
        &fixture.root,
        &["push", "origin", "main:refs/heads/created-after-clone"],
    );
    run_git(
        &fixture.root,
        &[
            "update-ref",
            "-d",
            "refs/remotes/origin/created-after-clone",
        ],
    );

    let refs = fetch_and_list_refs(&fixture.root).unwrap();

    assert!(refs.iter().any(|branch_ref| matches!(
        branch_ref,
        BranchRef::Remote { remote, name, .. }
            if remote == "origin" && name == "created-after-clone"
    )));
}

#[test]
fn fetches_primary_remote_when_branch_has_no_upstream() {
    let fixture = GitFixture::new();
    run_git(
        &fixture.root,
        &["push", "origin", "main:refs/heads/primary-only"],
    );

    let secondary = fixture.tempdir.path().join("secondary.git");
    run_git(
        fixture.tempdir.path(),
        &["init", "--bare", "-b", "main", secondary.to_str().unwrap()],
    );
    run_git(
        &fixture.root,
        &["remote", "add", "zz-secondary", secondary.to_str().unwrap()],
    );
    run_git(&fixture.root, &["push", "zz-secondary", "main"]);
    run_git(&fixture.root, &["remote", "remove", "origin"]);
    run_git(
        &fixture.root,
        &[
            "remote",
            "add",
            "a-primary",
            fixture.remote.to_str().unwrap(),
        ],
    );
    run_git(
        &fixture.root,
        &["config", "branch.main.remote", "zz-secondary"],
    );
    let _ = command::blocking::Command::new("git")
        .arg("-C")
        .arg(&fixture.root)
        .args(["config", "--unset-all", "branch.main.merge"])
        .status()
        .unwrap();

    let refs = fetch_and_list_refs(&fixture.root).unwrap();

    assert!(refs.iter().any(|branch_ref| matches!(
        branch_ref,
        BranchRef::Remote { remote, name, .. }
            if remote == "a-primary" && name == "primary-only"
    )));
}

#[test]
fn omits_symbolic_remote_head_from_ref_lists() {
    let fixture = GitFixture::new();

    let listed_refs = list_branch_refs(&fixture.root).unwrap();
    let fetched_refs = fetch_and_list_refs(&fixture.root).unwrap();

    for refs in [listed_refs, fetched_refs] {
        assert!(!refs.iter().any(|branch_ref| matches!(
            branch_ref,
            BranchRef::Remote { name, full_ref, .. }
                if name == "HEAD" || full_ref == "refs/remotes/origin/HEAD"
        )));
    }
}

#[test]
fn parses_worktree_paths_and_full_branch_refs() {
    let fixture = GitFixture::new();
    let linked_path = fixture.add_linked_worktree("feature/worktree");

    let worktrees = list_worktrees(&fixture.root).unwrap();

    assert!(worktrees.iter().any(|worktree| {
        worktree.path == fixture.root.canonicalize().unwrap()
            && worktree.branch.as_deref() == Some("refs/heads/main")
    }));
    assert!(worktrees.iter().any(|worktree| {
        worktree.path == linked_path.canonicalize().unwrap()
            && worktree.branch.as_deref() == Some("refs/heads/feature/worktree")
    }));
}

#[test]
fn existing_worktree_options_include_primary_before_linked_worktrees() {
    let repository_root = PathBuf::from("/tmp/repository");
    let options = existing_worktree_options(
        &repository_root,
        [
            WorktreeInfo {
                path: repository_root.clone(),
                head: Some("a".to_string()),
                branch: Some("refs/heads/main".to_string()),
                is_bare: false,
                is_detached: false,
                is_locked: false,
                locked_reason: None,
                is_prunable: false,
                prunable_reason: None,
            },
            WorktreeInfo {
                path: PathBuf::from("/tmp/repository-feature"),
                head: Some("b".to_string()),
                branch: Some("refs/heads/feature/existing".to_string()),
                is_bare: false,
                is_detached: false,
                is_locked: false,
                locked_reason: None,
                is_prunable: false,
                prunable_reason: None,
            },
            WorktreeInfo {
                path: PathBuf::from("/tmp/repository-detached"),
                head: Some("c".to_string()),
                branch: None,
                is_bare: false,
                is_detached: true,
                is_locked: false,
                locked_reason: None,
                is_prunable: false,
                prunable_reason: None,
            },
            WorktreeInfo {
                path: PathBuf::from("/tmp/repository-prunable"),
                head: Some("d".to_string()),
                branch: Some("refs/heads/feature/prunable".to_string()),
                is_bare: false,
                is_detached: false,
                is_locked: false,
                locked_reason: None,
                is_prunable: true,
                prunable_reason: Some("missing".to_string()),
            },
        ],
    );

    assert_eq!(
        options,
        vec![
            ExistingWorktreeOption::primary(repository_root.clone(), "main"),
            ExistingWorktreeOption::new(
                PathBuf::from("/tmp/repository-feature"),
                "feature/existing",
            ),
        ],
    );
}

#[test]
fn existing_worktree_options_recognize_primary_path_aliases() {
    let fixture = GitFixture::new();
    let alias_parent = fixture.tempdir.path().join("repository-alias");
    std::fs::create_dir(&alias_parent).unwrap();
    let repository_root = alias_parent
        .join("..")
        .join(fixture.root.file_name().unwrap());
    let worktrees = list_worktrees(&fixture.root).unwrap();

    let options = existing_worktree_options(&repository_root, worktrees);

    assert_eq!(options.first().map(|option| option.is_primary), Some(true));
    assert_eq!(
        options.first().map(|option| option.branch_name.as_str()),
        Some("main")
    );
}

#[test]
fn validates_registered_existing_worktree_without_rejecting_dirty_contents() {
    let fixture = GitFixture::new();
    let worktree_path = fixture.add_linked_worktree("feature/adopt");
    std::fs::write(worktree_path.join("untracked.txt"), "dirty").unwrap();

    assert_eq!(
        validate_existing_worktree(&fixture.root, &worktree_path, "feature/adopt").unwrap(),
        worktree_path.canonicalize().unwrap(),
    );
}

#[test]
fn validates_repository_primary_worktree_for_existing_workspace_adoption() {
    let fixture = GitFixture::new();

    assert_eq!(
        validate_existing_worktree(&fixture.root, &fixture.root, "main").unwrap(),
        fixture.root.canonicalize().unwrap(),
    );
}

#[test]
fn rejects_detached_primary_worktree_during_repository_validation() {
    let fixture = GitFixture::new();
    run_git(&fixture.root, &["checkout", "--detach", "HEAD"]);

    let error = validate_repository(&fixture.root).unwrap_err();

    assert!(matches!(
        error,
        GitWorkspaceError::PrimaryWorktreeDetached { path }
            if path == fixture.root.canonicalize().unwrap()
    ));
}

#[test]
fn rejects_detached_primary_worktree_during_workspace_adoption() {
    let fixture = GitFixture::new();
    run_git(&fixture.root, &["checkout", "--detach", "HEAD"]);

    let error = validate_existing_worktree(&fixture.root, &fixture.root, "main").unwrap_err();

    assert!(matches!(
        error,
        GitWorkspaceError::PrimaryWorktreeDetached { path }
            if path == fixture.root.canonicalize().unwrap()
    ));
}

#[test]
fn rejects_prunable_existing_worktree_during_workspace_adoption() {
    let fixture = GitFixture::new();
    let worktree_path = fixture.add_linked_worktree("feature/prunable-adoption");
    std::fs::remove_file(worktree_path.join(".git")).unwrap();

    let error =
        validate_existing_worktree(&fixture.root, &worktree_path, "feature/prunable-adoption")
            .unwrap_err();

    assert!(matches!(
        error,
        GitWorkspaceError::PrunableWorktreeCannotBeWorkspace { path }
            if path == worktree_path.canonicalize().unwrap()
    ));
}

#[test]
fn preserves_prunable_worktree_when_its_path_no_longer_exists() {
    let fixture = GitFixture::new();
    let linked_path = fixture.add_linked_worktree("feature/prunable");
    std::fs::remove_dir_all(&linked_path).unwrap();

    let worktrees = list_worktrees(&fixture.root).unwrap();
    let expected_path = linked_path
        .parent()
        .unwrap()
        .canonicalize()
        .unwrap()
        .join(linked_path.file_name().unwrap());

    assert!(worktrees.iter().any(|worktree| {
        worktree.path == expected_path
            && worktree.branch.as_deref() == Some("refs/heads/feature/prunable")
            && worktree.is_prunable
    }));
}

#[test]
fn preserves_newline_worktree_path() {
    let fixture = GitFixture::new();
    let linked_path = fixture.tempdir.path().join("worktree\nnewline");
    fixture.add_linked_worktree_at("feature/newline", &linked_path);

    let worktrees = list_worktrees(&fixture.root).unwrap();

    assert!(worktrees
        .iter()
        .any(|worktree| worktree.path == linked_path.canonicalize().unwrap()));
}

#[cfg(unix)]
#[test]
fn preserves_non_utf8_worktree_path() {
    use std::os::unix::ffi::OsStringExt;

    let tempdir = tempfile::tempdir().unwrap();
    let linked_path = tempdir
        .path()
        .join(std::ffi::OsString::from_vec(b"worktree-\xff".to_vec()));
    let mut output = b"worktree ".to_vec();
    output.extend(linked_path.as_os_str().as_encoded_bytes());
    output.extend_from_slice(
        b"\0HEAD 0123456789abcdef\0branch refs/heads/feature/non-utf8\0prunable missing\0\0",
    );

    let worktrees = parse_worktrees(&output).unwrap();
    let expected_path = tempdir
        .path()
        .canonicalize()
        .unwrap()
        .join(std::ffi::OsString::from_vec(b"worktree-\xff".to_vec()));

    assert_eq!(worktrees[0].path, expected_path);
}

#[test]
fn clones_repository_into_path_with_spaces_and_quotes() {
    let fixture = GitFixture::new();
    let target = fixture.tempdir.path().join("clone path with 'quote");

    let repository = clone_repository(fixture.remote.to_str().unwrap(), Some(&target)).unwrap();

    assert_eq!(repository.root, target.canonicalize().unwrap());
    assert_eq!(repository.remote_url, fixture.remote.to_str().unwrap());
    assert_eq!(
        std::fs::read_to_string(target.join("README.md")).unwrap(),
        "fixture"
    );
}

#[test]
fn clone_uses_repository_name_when_target_is_not_provided() {
    let fixture = GitFixture::new();
    let parent = fixture.tempdir.path().join("clone parent");
    std::fs::create_dir(&parent).unwrap();

    let repository =
        clone_repository_into(fixture.remote.to_str().unwrap(), &parent, None).unwrap();

    assert_eq!(
        repository.root,
        parent.join("remote repository").canonicalize().unwrap()
    );
}

#[test]
fn clone_into_rejects_invalid_directory_names_without_escaping_parent() {
    let fixture = GitFixture::new();
    let parent = fixture.tempdir.path().join("clone parent");
    std::fs::create_dir(&parent).unwrap();
    let escaped = fixture.tempdir.path().join("escaped");
    let absolute = fixture.tempdir.path().join("absolute-target");
    let invalid_names = [
        "../escaped".to_string(),
        absolute.to_string_lossy().into_owned(),
        "nested/name".to_string(),
        ".".to_string(),
        "".to_string(),
    ];

    for directory_name in invalid_names {
        let error = clone_repository_into(
            fixture.remote.to_str().unwrap(),
            &parent,
            Some(&directory_name),
        )
        .unwrap_err();
        assert!(matches!(
            error,
            GitWorkspaceError::InvalidCloneDirectoryName { name }
                if name == directory_name
        ));
    }

    assert!(!escaped.exists());
    assert!(!absolute.exists());
    assert!(!parent.join("nested").exists());
}

#[test]
fn clone_into_accepts_single_normal_directory_name() {
    let fixture = GitFixture::new();
    let parent = fixture.tempdir.path().join("custom clone parent");
    std::fs::create_dir(&parent).unwrap();

    let repository = clone_repository_into(
        fixture.remote.to_str().unwrap(),
        &parent,
        Some("custom clone"),
    )
    .unwrap();

    assert_eq!(
        repository.root,
        parent.join("custom clone").canonicalize().unwrap()
    );
}

#[test]
fn clone_failure_removes_target_created_by_the_operation() {
    let tempdir = tempfile::tempdir().unwrap();
    let missing_source = tempdir.path().join("missing repository.git");
    let target = tempdir.path().join("new target");

    let error = clone_repository(missing_source.to_str().unwrap(), Some(&target)).unwrap_err();

    assert!(matches!(
        error,
        GitWorkspaceError::CommandFailed {
            operation,
            ref args,
            ref stderr,
        } if operation == "clone repository"
            && args.first().is_some_and(|arg| arg == "clone")
            && !stderr.is_empty()
    ));
    assert!(!target.exists());
}

#[test]
fn clone_never_deletes_preexisting_target() {
    let fixture = GitFixture::new();
    let target = fixture.tempdir.path().join("existing target");
    std::fs::create_dir(&target).unwrap();
    let sentinel = target.join("keep.txt");
    std::fs::write(&sentinel, "keep").unwrap();

    let error = clone_repository(fixture.remote.to_str().unwrap(), Some(&target)).unwrap_err();

    assert!(matches!(error, GitWorkspaceError::TargetExists { .. }));
    assert_eq!(std::fs::read_to_string(sentinel).unwrap(), "keep");
}

#[test]
fn cleanup_failure_displays_clone_and_cleanup_errors() {
    let error = GitWorkspaceError::CleanupFailed {
        path: PathBuf::from("clone-target"),
        cleanup_source: std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "cleanup permission denied",
        ),
        clone_error: Box::new(GitWorkspaceError::CommandFailed {
            operation: "clone repository",
            args: vec!["clone".to_string()],
            stderr: "fatal: source repository missing".to_string(),
        }),
    };

    let message = error.to_string();

    assert!(message.contains("fatal: source repository missing"));
    assert!(message.contains("cleanup permission denied"));
}

#[test]
fn cleanup_failure_exposes_clone_error_as_primary_source() {
    use std::error::Error;

    let error = GitWorkspaceError::CleanupFailed {
        path: PathBuf::from("clone-target"),
        cleanup_source: std::io::Error::other("cleanup denied"),
        clone_error: Box::new(injected_command_error("clone repository")),
    };

    assert!(
        matches!(error.source(), Some(source) if source.to_string().contains("clone repository"))
    );
}

#[test]
fn parses_repository_names_from_supported_git_urls() {
    assert_eq!(
        repository_name_from_url("https://github.com/acme/widgets.git").unwrap(),
        "widgets"
    );
    assert_eq!(
        repository_name_from_url("git@github.com:acme/widgets.git").unwrap(),
        "widgets"
    );
    assert_eq!(
        repository_name_from_url("ssh://git@example.com/acme/widgets/").unwrap(),
        "widgets"
    );
    assert_eq!(
        repository_name_from_url("/tmp/repositories/local-widgets.git").unwrap(),
        "local-widgets"
    );
}

#[test]
fn rejects_git_url_without_repository_name() {
    let error = repository_name_from_url("ssh://git@example.com/").unwrap_err();

    assert!(matches!(
        error,
        GitWorkspaceError::RepositoryNameMissing { .. }
    ));
}

#[test]
fn parses_repository_name_from_windows_drive_path() {
    assert_eq!(
        repository_name_from_url(r"C:\repositories\windows-widgets.git").unwrap(),
        "windows-widgets"
    );
}

#[test]
fn creates_filesystem_safe_workspace_directory_names() {
    assert_eq!(
        workspace_dir_name("feature/a b", "12345678"),
        "feature-a-b-12345678"
    );
    assert_eq!(
        workspace_dir_name("  feature\\a::b///c  ", "abcdef12-extra"),
        "feature-a-b-c-abcdef12"
    );
    assert_eq!(
        workspace_dir_name("///:::   ", "fedcba98"),
        "workspace-fedcba98"
    );
}

#[test]
fn creates_new_branch_from_remote_ref_without_tracking() {
    let fixture = GitFixture::new();
    let path = fixture.tempdir.path().join("remote worktree");

    create_from_remote(
        &fixture.root,
        "refs/remotes/origin/main",
        "feature/remote",
        &path,
    )
    .unwrap();

    assert_eq!(current_branch(&path), "feature/remote");
    assert_eq!(branch_upstream(&fixture.root, "feature/remote"), None);
}

#[test]
fn creates_remote_worktree_for_nested_claimed_target() {
    let fixture = GitFixture::new();
    let worktree_path = fixture
        .tempdir
        .path()
        .join("missing-parent")
        .join("remote worktree");

    create_from_remote(
        &fixture.root,
        "refs/remotes/origin/main",
        "feature/nested-remote",
        &worktree_path,
    )
    .unwrap();

    assert_eq!(current_branch(&worktree_path), "feature/nested-remote");
    assert!(worktree_path.is_dir());
}

#[test]
fn successful_remote_creation_rejects_new_upstream() {
    let fixture = GitFixture::new();
    let path = fixture.tempdir.path().join("unexpected upstream");

    let error = create_from_remote_with_runner(
        &fixture.root,
        "refs/remotes/origin/main",
        "feature/unexpected-upstream",
        &path,
        |repository, remote_ref, new_branch, claimed_path, _| {
            let status = command::blocking::Command::new("git")
                .arg("-C")
                .arg(repository)
                .args(["worktree", "add", "--no-track", "-b", new_branch])
                .arg(claimed_path)
                .arg(remote_ref)
                .status()
                .unwrap();
            assert!(status.success());
            run_git(
                repository,
                &[
                    "branch",
                    "--set-upstream-to=origin/main",
                    "feature/unexpected-upstream",
                ],
            );
            Ok(())
        },
    )
    .unwrap_err();

    assert!(matches!(
        error,
        GitWorkspaceError::WorktreeCreationVerificationFailed {
            verification_error,
            ..
        } if matches!(verification_error.as_ref(), GitWorkspaceError::UnexpectedBranchUpstream {
            branch,
            upstream,
        } if branch == "feature/unexpected-upstream" && upstream == "refs/remotes/origin/main")
    ));
}

#[test]
fn creates_remote_worktree_for_relative_claimed_target() {
    let fixture = GitFixture::new();
    assert_ne!(std::env::current_dir().unwrap(), fixture.root);
    let mut relative_target = std::ffi::OsString::from(".task2-relative-remote-");
    relative_target.push(fixture.tempdir.path().file_name().unwrap());
    let relative_target = PathBuf::from(relative_target);
    let worktree_path = std::env::current_dir().unwrap().join(&relative_target);
    let _cleanup = RelativeWorktreeCleanup {
        repository: fixture.root.clone(),
        worktree_path,
    };

    create_from_remote(
        &fixture.root,
        "refs/remotes/origin/main",
        "feature/relative-remote",
        &relative_target,
    )
    .unwrap();

    let canonical_target = relative_target.canonicalize().unwrap();
    assert_eq!(current_branch(&canonical_target), "feature/relative-remote");
    assert!(!fixture.root.join(&relative_target).exists());

    let status = command::blocking::Command::new("git")
        .arg("-C")
        .arg(&fixture.root)
        .args(["worktree", "remove"])
        .arg(&canonical_target)
        .status()
        .unwrap();
    assert!(status.success());
}

#[test]
fn creates_local_worktree_for_relative_claimed_target() {
    let fixture = GitFixture::new();
    assert_ne!(std::env::current_dir().unwrap(), fixture.root);
    run_git(&fixture.root, &["branch", "feature/relative-local"]);
    let mut relative_target = std::ffi::OsString::from(".task2-relative-local-");
    relative_target.push(fixture.tempdir.path().file_name().unwrap());
    let relative_target = PathBuf::from(relative_target);
    let worktree_path = std::env::current_dir().unwrap().join(&relative_target);
    let _cleanup = RelativeWorktreeCleanup {
        repository: fixture.root.clone(),
        worktree_path,
    };

    create_from_local(&fixture.root, "feature/relative-local", &relative_target).unwrap();

    let canonical_target = relative_target.canonicalize().unwrap();
    assert_eq!(current_branch(&canonical_target), "feature/relative-local");
    assert!(!fixture.root.join(&relative_target).exists());

    let status = command::blocking::Command::new("git")
        .arg("-C")
        .arg(&fixture.root)
        .args(["worktree", "remove"])
        .arg(&canonical_target)
        .status()
        .unwrap();
    assert!(status.success());
}

#[test]
fn creates_local_worktree_for_nested_claimed_target() {
    let fixture = GitFixture::new();
    run_git(&fixture.root, &["branch", "feature/nested-local"]);
    let worktree_path = fixture
        .tempdir
        .path()
        .join("missing-parent")
        .join("local worktree");

    create_from_local(&fixture.root, "feature/nested-local", &worktree_path).unwrap();

    assert_eq!(current_branch(&worktree_path), "feature/nested-local");
    assert!(worktree_path.is_dir());
}

#[test]
fn successful_remote_creation_rejects_branch_change_before_verification() {
    let fixture = GitFixture::new();
    let path = fixture.tempdir.path().join("changed after success");
    let expected_oid = ref_oid(&fixture.root, "refs/remotes/origin/main");
    let changed_oid = git_output(
        &fixture.root,
        &[
            "-c",
            "user.name=Zap Tests",
            "-c",
            "user.email=zap@example.com",
            "commit-tree",
            "HEAD^{tree}",
            "-p",
            "HEAD",
            "-m",
            "post-success branch change",
        ],
    );

    let error = create_from_remote_with_runner(
        &fixture.root,
        "refs/remotes/origin/main",
        "feature/post-success-change",
        &path,
        |repository, remote_ref, new_branch, claimed_path, runner_expected_oid| {
            let status = command::blocking::Command::new("git")
                .arg("-C")
                .arg(repository)
                .args(["worktree", "add", "--no-track", "-b", new_branch])
                .arg(claimed_path)
                .arg(remote_ref)
                .status()
                .unwrap();
            assert!(status.success());
            run_git(
                &fixture.root,
                &[
                    "update-ref",
                    "refs/heads/feature/post-success-change",
                    &changed_oid,
                    runner_expected_oid,
                ],
            );
            Ok(())
        },
    )
    .unwrap_err();

    assert!(matches!(
        error,
        GitWorkspaceError::WorktreeCreationVerificationFailed {
            worktree_path,
            branch,
            expected_oid: expected,
            verification_error,
        } if worktree_path == path
            && branch == "feature/post-success-change"
            && expected == expected_oid
            && matches!(verification_error.as_ref(), GitWorkspaceError::BranchChanged {
                branch,
                expected_oid: nested_expected,
                actual_oid: Some(actual),
                actual_symbolic_target: None,
            } if branch == "feature/post-success-change"
                && *nested_expected == expected_oid
                && *actual == changed_oid)
    ));
    assert_eq!(
        ref_oid(&fixture.root, "refs/heads/feature/post-success-change"),
        changed_oid
    );
    assert!(path.exists());
}

#[test]
fn rejects_invalid_or_symbolic_remote_refs_before_creation() {
    let fixture = GitFixture::new();

    for remote_ref in ["refs/heads/main", "refs/remotes/origin/HEAD"] {
        let branch = format!("feature/{}", remote_ref.replace('/', "-"));
        let path = fixture.tempdir.path().join(&branch);
        let error = create_from_remote(&fixture.root, remote_ref, &branch, &path).unwrap_err();

        assert!(matches!(
            error,
            GitWorkspaceError::InvalidRemoteRef { full_ref } if full_ref == remote_ref
        ));
        assert!(!ref_exists(&fixture.root, &format!("refs/heads/{branch}")));
        assert!(!path.exists());
    }
}

#[test]
fn rejects_missing_remote_ref_before_creation() {
    let fixture = GitFixture::new();
    let path = fixture.tempdir.path().join("missing remote");

    let error = create_from_remote(
        &fixture.root,
        "refs/remotes/origin/missing",
        "feature/missing-remote",
        &path,
    )
    .unwrap_err();

    assert!(matches!(
        error,
        GitWorkspaceError::BranchNotFound { full_ref }
            if full_ref == "refs/remotes/origin/missing"
    ));
    assert!(!ref_exists(
        &fixture.root,
        "refs/heads/feature/missing-remote"
    ));
    assert!(!path.exists());
}

#[test]
fn rejects_invalid_new_branch_name_before_creation() {
    let fixture = GitFixture::new();
    let path = fixture.tempdir.path().join("invalid branch");

    let error = create_from_remote(
        &fixture.root,
        "refs/remotes/origin/main",
        "invalid branch",
        &path,
    )
    .unwrap_err();

    assert!(matches!(
        error,
        GitWorkspaceError::InvalidBranchName { branch } if branch == "invalid branch"
    ));
    assert!(!path.exists());
}

#[test]
fn rejects_existing_new_branch_before_creation() {
    let fixture = GitFixture::new();
    run_git(&fixture.root, &["branch", "feature/existing"]);
    let path = fixture.tempdir.path().join("existing branch");

    let error = create_from_remote(
        &fixture.root,
        "refs/remotes/origin/main",
        "feature/existing",
        &path,
    )
    .unwrap_err();

    assert!(matches!(
        error,
        GitWorkspaceError::BranchAlreadyExists { branch } if branch == "feature/existing"
    ));
    assert!(!path.exists());
}

#[test]
fn rejects_existing_target_before_remote_creation() {
    let fixture = GitFixture::new();
    let path = fixture.tempdir.path().join("existing target");
    std::fs::create_dir(&path).unwrap();

    let error = create_from_remote(
        &fixture.root,
        "refs/remotes/origin/main",
        "feature/target-exists",
        &path,
    )
    .unwrap_err();

    assert!(matches!(
        error,
        GitWorkspaceError::TargetExists { path: error_path } if error_path == path
    ));
    assert!(!ref_exists(
        &fixture.root,
        "refs/heads/feature/target-exists"
    ));
}

#[test]
fn remote_creation_claims_target_before_git_command() {
    let fixture = GitFixture::new();
    let path = fixture.tempdir.path().join("remote claimed target");

    create_from_remote_with_after_target_claim_hook(
        &fixture.root,
        "refs/remotes/origin/main",
        "feature/remote-target-claim",
        &path,
        || {
            let error = std::fs::create_dir(&path).unwrap_err();
            assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
        },
    )
    .unwrap();

    assert_eq!(current_branch(&path), "feature/remote-target-claim");
}

#[test]
fn local_creation_claims_target_before_git_command() {
    let fixture = GitFixture::new();
    run_git(&fixture.root, &["branch", "feature/local-target-claim"]);
    let path = fixture.tempdir.path().join("local claimed target");

    create_from_local_with_after_target_claim_hook(
        &fixture.root,
        "feature/local-target-claim",
        &path,
        || {
            let error = std::fs::create_dir(&path).unwrap_err();
            assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
        },
    )
    .unwrap();

    assert_eq!(current_branch(&path), "feature/local-target-claim");
}

#[test]
fn creation_rejects_claim_replaced_by_file_before_git_command() {
    let fixture = GitFixture::new();
    let path = fixture.tempdir.path().join("replaced target");

    let error = create_from_remote_with_after_target_claim_hook(
        &fixture.root,
        "refs/remotes/origin/main",
        "feature/replaced-target",
        &path,
        || {
            std::fs::remove_dir(&path).unwrap();
            std::fs::write(&path, "replacement").unwrap();
        },
    )
    .unwrap_err();

    assert!(matches!(
        error,
        GitWorkspaceError::ClaimedTargetNotDirectory { path: error_path } if error_path == path
    ));
    assert!(!ref_exists(
        &fixture.root,
        "refs/heads/feature/replaced-target"
    ));
    assert_eq!(std::fs::read_to_string(path).unwrap(), "replacement");
}

#[cfg(unix)]
#[test]
fn rejects_dangling_symlink_target_without_creating_branch() {
    use std::os::unix::fs::symlink;

    let fixture = GitFixture::new();
    let path = fixture.tempdir.path().join("dangling target");
    symlink(fixture.tempdir.path().join("missing target"), &path).unwrap();

    let error = create_from_remote(
        &fixture.root,
        "refs/remotes/origin/main",
        "feature/dangling-target",
        &path,
    )
    .unwrap_err();

    assert!(matches!(
        error,
        GitWorkspaceError::TargetExists { path: error_path } if error_path == path
    ));
    assert!(!ref_exists(
        &fixture.root,
        "refs/heads/feature/dangling-target"
    ));
    assert!(std::fs::symlink_metadata(path)
        .unwrap()
        .file_type()
        .is_symlink());
}

#[test]
fn remote_runner_failure_preserves_branch_and_removes_empty_claimed_target() {
    let fixture = GitFixture::new();
    let path = fixture.tempdir.path().join("remote residual target");

    let error = create_from_remote_with_runner(
        &fixture.root,
        "refs/remotes/origin/main",
        "feature/residual",
        &path,
        |repository, _, new_branch, _, expected_oid| {
            run_git(
                repository,
                &["update-ref", "refs/heads/feature/residual", expected_oid],
            );
            assert_eq!(new_branch, "feature/residual");
            Err(injected_command_error(
                "inject remote residual creation failure",
            ))
        },
    )
    .unwrap_err();

    assert!(matches!(
        error,
        GitWorkspaceError::WorktreeCreationFailed {
            worktree_path,
            branch,
            branch_may_remain: true,
            worktree_registered: Some(false),
            claimed_directory_removed: true,
            create_error,
            cleanup_error: None,
        } if worktree_path == path
            && branch == "feature/residual"
            && matches!(create_error.as_ref(), GitWorkspaceError::CommandFailed {
                operation: "inject remote residual creation failure",
                ..
            })
    ));
    assert!(ref_exists(&fixture.root, "refs/heads/feature/residual"));
    assert!(!path.exists());
}

#[test]
fn remote_runner_failure_keeps_nonempty_claimed_target() {
    let fixture = GitFixture::new();
    let path = fixture.tempdir.path().join("remote nonempty target");
    let sentinel = path.join("keep.txt");

    let error = create_from_remote_with_runner(
        &fixture.root,
        "refs/remotes/origin/main",
        "feature/nonempty-residual",
        &path,
        |_, _, _, claimed_path, _| {
            std::fs::write(claimed_path.join("keep.txt"), "keep").unwrap();
            Err(injected_command_error(
                "inject remote nonempty creation failure",
            ))
        },
    )
    .unwrap_err();

    assert!(matches!(
        error,
        GitWorkspaceError::WorktreeCreationFailed {
            worktree_path,
            branch,
            branch_may_remain: true,
            worktree_registered: Some(false),
            claimed_directory_removed: false,
            create_error,
            cleanup_error: Some(cleanup_error),
        } if worktree_path == path
            && branch == "feature/nonempty-residual"
            && matches!(create_error.as_ref(), GitWorkspaceError::CommandFailed {
                operation: "inject remote nonempty creation failure",
                ..
            })
            && matches!(cleanup_error.as_ref(), GitWorkspaceError::ClaimedTargetNotEmpty {
                path: cleanup_path,
            } if cleanup_path == &path)
    ));
    assert_eq!(std::fs::read_to_string(sentinel).unwrap(), "keep");
}

#[test]
fn remote_runner_failure_preserves_registered_worktree() {
    let fixture = GitFixture::new();
    let path = fixture.tempdir.path().join("registered residual target");

    let error = create_from_remote_with_runner(
        &fixture.root,
        "refs/remotes/origin/main",
        "feature/registered-residual",
        &path,
        |repository, remote_ref, new_branch, claimed_path, _| {
            let status = command::blocking::Command::new("git")
                .arg("-C")
                .arg(repository)
                .args(["worktree", "add", "--no-track", "-b", new_branch])
                .arg(claimed_path)
                .arg(remote_ref)
                .status()
                .unwrap();
            assert!(status.success());
            Err(injected_command_error(
                "inject registered remote creation failure",
            ))
        },
    )
    .unwrap_err();

    assert!(matches!(
        error,
        GitWorkspaceError::WorktreeCreationFailed {
            worktree_path,
            branch,
            branch_may_remain: true,
            worktree_registered: Some(true),
            claimed_directory_removed: false,
            create_error,
            cleanup_error: None,
        } if worktree_path == path
            && branch == "feature/registered-residual"
            && matches!(create_error.as_ref(), GitWorkspaceError::CommandFailed {
                operation: "inject registered remote creation failure",
                ..
            })
    ));
    assert!(path.exists());
    assert!(ref_exists(
        &fixture.root,
        "refs/heads/feature/registered-residual"
    ));
}

#[test]
fn local_runner_failure_removes_empty_claimed_target_and_preserves_branch() {
    let fixture = GitFixture::new();
    let path = fixture.tempdir.path().join("local residual target");
    run_git(&fixture.root, &["branch", "feature/local-residual"]);

    let error =
        create_from_local_with_runner(&fixture.root, "feature/local-residual", &path, |_, _, _| {
            Err(injected_command_error("inject local creation failure"))
        })
        .unwrap_err();

    assert!(matches!(
        error,
        GitWorkspaceError::WorktreeCreationFailed {
            worktree_path,
            branch,
            branch_may_remain: false,
            worktree_registered: Some(false),
            claimed_directory_removed: true,
            create_error,
            cleanup_error: None,
        } if worktree_path == path
            && branch == "feature/local-residual"
            && matches!(create_error.as_ref(), GitWorkspaceError::CommandFailed {
                operation: "inject local creation failure",
                ..
            })
    ));
    assert!(!path.exists());
    assert!(ref_exists(
        &fixture.root,
        "refs/heads/feature/local-residual"
    ));
}

#[test]
fn rejects_missing_local_branch_before_creation() {
    let fixture = GitFixture::new();
    let path = fixture.tempdir.path().join("missing local");

    let error = create_from_local(&fixture.root, "feature/missing", &path).unwrap_err();

    assert!(matches!(
        error,
        GitWorkspaceError::BranchNotFound { full_ref }
            if full_ref == "refs/heads/feature/missing"
    ));
    assert!(!path.exists());
}

#[test]
fn reports_main_repository_path_when_local_branch_is_checked_out() {
    let fixture = GitFixture::new();
    let path = fixture.tempdir.path().join("second main");

    let error = create_from_local(&fixture.root, "main", &path).unwrap_err();

    assert!(matches!(
        error,
        GitWorkspaceError::BranchAlreadyCheckedOut { branch, path }
            if branch == "main" && path == fixture.root.canonicalize().unwrap()
    ));
    assert!(!path.exists());
}

#[test]
fn reports_the_path_that_already_checks_out_a_local_branch() {
    let fixture = GitFixture::new();
    let occupied = fixture.add_linked_worktree("feature/occupied");
    let path = fixture.tempdir.path().join("second occupied");

    let error = create_from_local(&fixture.root, "feature/occupied", &path).unwrap_err();

    assert!(matches!(
        error,
        GitWorkspaceError::BranchAlreadyCheckedOut { branch, path }
            if branch == "feature/occupied" && path == occupied.canonicalize().unwrap()
    ));
    assert!(!path.exists());
}

#[test]
fn creates_local_worktree_at_path_with_spaces_and_quotes() {
    let fixture = GitFixture::new();
    run_git(&fixture.root, &["branch", "feature/local"]);
    let path = fixture.tempdir.path().join("local worktree with 'quote");

    create_from_local(&fixture.root, "feature/local", &path).unwrap();

    assert_eq!(current_branch(&path), "feature/local");
}

#[cfg(unix)]
#[test]
fn os_arg_command_helper_preserves_non_utf8_bytes() {
    use std::os::unix::ffi::{OsStrExt, OsStringExt};

    let fixture = GitFixture::new();
    let raw_arg = std::ffi::OsString::from_vec(b"raw-path-\xff".to_vec());
    let args = [
        std::ffi::OsStr::new("rev-parse"),
        std::ffi::OsStr::new("--git-path"),
        raw_arg.as_os_str(),
    ];

    let output =
        git_output_with_os_args_for_operation(&fixture.root, "echo raw path", &args).unwrap();

    assert!(output.stdout.ends_with(b"raw-path-\xff\n"));
    assert_eq!(raw_arg.as_os_str().as_bytes(), b"raw-path-\xff");
}

#[test]
fn dirty_worktree_blocks_deletion_before_mutation() {
    let fixture = GitFixture::new();
    let worktree_path = fixture.add_linked_worktree("feature/dirty");

    let untracked = worktree_path.join("untracked.txt");
    std::fs::write(&untracked, "untracked").unwrap();
    assert!(matches!(
        deletion_preflight(&fixture.root, &worktree_path, true),
        Err(GitWorkspaceError::DirtyWorktree { path }) if path == worktree_path.canonicalize().unwrap()
    ));
    assert_eq!(std::fs::read_to_string(&untracked).unwrap(), "untracked");
    std::fs::remove_file(&untracked).unwrap();

    let tracked = worktree_path.join("README.md");
    std::fs::write(&tracked, "unstaged").unwrap();
    assert!(matches!(
        deletion_preflight(&fixture.root, &worktree_path, true),
        Err(GitWorkspaceError::DirtyWorktree { .. })
    ));
    run_git(&worktree_path, &["restore", "README.md"]);

    std::fs::write(&tracked, "staged").unwrap();
    run_git(&worktree_path, &["add", "README.md"]);
    assert!(matches!(
        deletion_preflight(&fixture.root, &worktree_path, true),
        Err(GitWorkspaceError::DirtyWorktree { .. })
    ));
    assert!(worktree_path.exists());
    assert!(ref_exists(&fixture.root, "refs/heads/feature/dirty"));
}

#[test]
fn worktree_branch_mismatch_blocks_removal_before_mutation() {
    let fixture = GitFixture::new();
    let worktree_path = fixture.add_linked_worktree("feature/actual");

    let error = remove_workspace(
        &fixture.root,
        &worktree_path,
        "feature/expected",
        true,
        false,
    )
    .unwrap_err();

    assert!(matches!(
        error,
        GitWorkspaceError::WorktreeBranchMismatch { expected, actual }
            if expected == "feature/expected" && actual == "feature/actual"
    ));
    assert!(worktree_path.exists());
    assert!(ref_exists(&fixture.root, "refs/heads/feature/actual"));
}

#[test]
fn missing_branch_blocks_deletion_before_mutation() {
    let fixture = GitFixture::new();
    let worktree_path = fixture.add_linked_worktree("feature/missing-ref");
    run_git(
        &fixture.root,
        &["update-ref", "-d", "refs/heads/feature/missing-ref"],
    );

    let error = deletion_preflight(&fixture.root, &worktree_path, true).unwrap_err();

    assert!(matches!(
        error,
        GitWorkspaceError::BranchNotFound { full_ref }
            if full_ref == "refs/heads/feature/missing-ref"
    ));
    assert!(worktree_path.exists());
}

#[test]
fn detached_worktree_is_rejected_before_mutation() {
    let fixture = GitFixture::new();
    let worktree_path = fixture.tempdir.path().join("detached worktree");
    let output = command::blocking::Command::new("git")
        .arg("-C")
        .arg(&fixture.root)
        .args(["worktree", "add", "--detach"])
        .arg(&worktree_path)
        .arg("HEAD")
        .output()
        .unwrap();
    assert!(output.status.success());

    let error = deletion_preflight(&fixture.root, &worktree_path, false).unwrap_err();

    assert!(matches!(
        error,
        GitWorkspaceError::WorktreeHasNoLocalBranch { path }
            if path == worktree_path.canonicalize().unwrap()
    ));
    assert!(worktree_path.exists());
}

#[test]
fn deletion_preflight_captures_merge_target_oid() {
    let fixture = GitFixture::new();
    let worktree_path = fixture.add_linked_worktree("feature/preflight-target");

    let preflight = deletion_preflight(&fixture.root, &worktree_path, true).unwrap();
    let merge_target = preflight.merge_target.as_ref().unwrap();

    assert_eq!(preflight.branch_ref, "refs/heads/feature/preflight-target");
    assert_eq!(merge_target.full_ref, "refs/remotes/origin/main");
    assert_eq!(
        merge_target.oid,
        ref_oid(&fixture.root, "refs/remotes/origin/main")
    );
    assert!(merge_target.is_merged);
}

#[test]
fn gone_upstream_falls_back_to_default_branch_and_removes_merged_workspace() {
    let fixture = GitFixture::new();
    let worktree_path = fixture.add_linked_worktree("feature/gone-upstream");
    set_gone_upstream(&fixture, "feature/gone-upstream");

    let preflight = deletion_preflight(&fixture.root, &worktree_path, true).unwrap();
    let merge_target = preflight.merge_target.as_ref().unwrap();

    assert_eq!(merge_target.full_ref, "refs/remotes/origin/main");
    assert_eq!(
        merge_target.oid,
        ref_oid(&fixture.root, "refs/remotes/origin/main")
    );
    assert!(merge_target.is_merged);

    remove_workspace(
        &fixture.root,
        &worktree_path,
        "feature/gone-upstream",
        true,
        false,
    )
    .unwrap();

    assert!(!worktree_path.exists());
    assert!(!ref_exists(
        &fixture.root,
        "refs/heads/feature/gone-upstream"
    ));
}

#[test]
fn gone_upstream_unmerged_branch_blocks_deletion_against_default_branch() {
    let fixture = GitFixture::new();
    let worktree_path = fixture.add_linked_worktree("feature/gone-unmerged");
    std::fs::write(worktree_path.join("feature.txt"), "feature").unwrap();
    run_git(&worktree_path, &["add", "feature.txt"]);
    run_git(
        &worktree_path,
        &[
            "-c",
            "user.name=Zap Tests",
            "-c",
            "user.email=zap@example.com",
            "commit",
            "-m",
            "feature commit",
        ],
    );
    set_gone_upstream(&fixture, "feature/gone-unmerged");

    let preflight = deletion_preflight(&fixture.root, &worktree_path, true).unwrap();
    let merge_target = preflight.merge_target.as_ref().unwrap();
    assert!(!merge_target.is_merged);
    assert_eq!(merge_target.full_ref, "refs/remotes/origin/main");

    let error = remove_workspace(
        &fixture.root,
        &worktree_path,
        "feature/gone-unmerged",
        true,
        false,
    )
    .unwrap_err();

    assert!(matches!(
        error,
        GitWorkspaceError::BranchNotMerged { branch, merge_target }
            if branch == "feature/gone-unmerged" && merge_target == "refs/remotes/origin/main"
    ));
    assert!(worktree_path.exists());
    assert!(ref_exists(
        &fixture.root,
        "refs/heads/feature/gone-unmerged"
    ));
}

#[test]
fn deletion_preflight_rejects_self_merge_target() {
    let fixture = GitFixture::new();
    let worktree_path = fixture.add_linked_worktree("feature/self-target");
    run_git(
        &fixture.root,
        &["config", "branch.feature/self-target.remote", "."],
    );
    run_git(
        &fixture.root,
        &[
            "config",
            "branch.feature/self-target.merge",
            "refs/heads/feature/self-target",
        ],
    );

    let error = deletion_preflight(&fixture.root, &worktree_path, true).unwrap_err();

    assert!(matches!(
        error,
        GitWorkspaceError::InvalidMergeTarget {
            branch_ref,
            target_ref,
        } if branch_ref == "refs/heads/feature/self-target"
            && target_ref == "refs/heads/feature/self-target"
    ));
}

#[test]
fn unmerged_branch_without_force_blocks_removal_before_mutation() {
    let fixture = GitFixture::new();
    let worktree_path = fixture.add_linked_worktree("feature/unmerged");
    std::fs::write(worktree_path.join("feature.txt"), "feature").unwrap();
    run_git(&worktree_path, &["add", "feature.txt"]);
    run_git(
        &worktree_path,
        &[
            "-c",
            "user.name=Zap Tests",
            "-c",
            "user.email=zap@example.com",
            "commit",
            "-m",
            "feature commit",
        ],
    );

    let preflight = deletion_preflight(&fixture.root, &worktree_path, true).unwrap();
    let merge_target = preflight.merge_target.as_ref().unwrap();
    assert!(!merge_target.is_merged);
    assert_eq!(merge_target.full_ref, "refs/remotes/origin/main");

    let error = remove_workspace(
        &fixture.root,
        &worktree_path,
        "feature/unmerged",
        true,
        false,
    )
    .unwrap_err();

    assert!(matches!(
        error,
        GitWorkspaceError::BranchNotMerged { branch, merge_target }
            if branch == "feature/unmerged" && merge_target == "refs/remotes/origin/main"
    ));
    assert!(worktree_path.exists());
    assert!(ref_exists(&fixture.root, "refs/heads/feature/unmerged"));
}

#[test]
fn merged_branch_is_safely_removed() {
    let fixture = GitFixture::new();
    let worktree_path = fixture.add_linked_worktree("feature/merged");
    run_git(
        &fixture.root,
        &["push", "origin", "main:refs/heads/integration"],
    );
    run_git(&fixture.root, &["fetch", "origin"]);
    run_git(
        &fixture.root,
        &[
            "branch",
            "--set-upstream-to=origin/integration",
            "feature/merged",
        ],
    );

    let preflight = deletion_preflight(&fixture.root, &worktree_path, true).unwrap();
    assert_eq!(preflight.branch, "feature/merged");
    let merge_target = preflight.merge_target.as_ref().unwrap();
    assert!(merge_target.is_merged);
    assert_eq!(merge_target.full_ref, "refs/remotes/origin/integration");

    remove_workspace(&fixture.root, &worktree_path, "feature/merged", true, false).unwrap();

    assert!(!worktree_path.exists());
    assert!(!ref_exists(&fixture.root, "refs/heads/feature/merged"));
}

#[test]
fn default_merge_target_deletion_does_not_depend_on_main_worktree_head() {
    let fixture = GitFixture::new();
    let worktree_path = fixture.add_linked_worktree("feature/default-target");
    std::fs::write(worktree_path.join("merged.txt"), "merged").unwrap();
    run_git(&worktree_path, &["add", "merged.txt"]);
    run_git(
        &worktree_path,
        &[
            "-c",
            "user.name=Zap Tests",
            "-c",
            "user.email=zap@example.com",
            "commit",
            "-m",
            "merged feature",
        ],
    );
    run_git(
        &fixture.root,
        &[
            "-c",
            "user.name=Zap Tests",
            "-c",
            "user.email=zap@example.com",
            "merge",
            "--no-ff",
            "feature/default-target",
            "-m",
            "merge feature",
        ],
    );
    run_git(&fixture.root, &["push", "origin", "main"]);
    run_git(&fixture.root, &["branch", "other", "HEAD~1"]);
    run_git(&fixture.root, &["switch", "other"]);

    let preflight = deletion_preflight(&fixture.root, &worktree_path, true).unwrap();
    let merge_target = preflight.merge_target.as_ref().unwrap();
    assert!(merge_target.is_merged);
    assert_eq!(merge_target.full_ref, "refs/remotes/origin/main");
    assert_eq!(
        preflight.worktree_path,
        worktree_path.canonicalize().unwrap()
    );
    assert_eq!(
        preflight.branch_oid,
        ref_oid(&fixture.root, "refs/heads/feature/default-target")
    );

    remove_workspace(
        &fixture.root,
        &worktree_path,
        "feature/default-target",
        true,
        false,
    )
    .unwrap();

    assert!(!worktree_path.exists());
    assert!(!ref_exists(
        &fixture.root,
        "refs/heads/feature/default-target"
    ));
}

#[test]
fn branch_oid_change_after_candidate_capture_blocks_worktree_mutation() {
    let fixture = GitFixture::new();
    let worktree_path = fixture.add_linked_worktree("feature/changed");
    let expected_oid = ref_oid(&fixture.root, "refs/heads/feature/changed");
    let changed_oid = git_output(
        &fixture.root,
        &[
            "-c",
            "user.name=Zap Tests",
            "-c",
            "user.email=zap@example.com",
            "commit-tree",
            "HEAD^{tree}",
            "-p",
            "HEAD",
            "-m",
            "changed ref",
        ],
    );

    let error = remove_workspace_with_hook(
        &fixture.root,
        &worktree_path,
        "feature/changed",
        true,
        false,
        || {
            run_git(
                &fixture.root,
                &[
                    "update-ref",
                    "refs/heads/feature/changed",
                    &changed_oid,
                    &expected_oid,
                ],
            );
        },
    )
    .unwrap_err();

    assert!(matches!(error, GitWorkspaceError::RefTransaction { .. }));
    assert!(worktree_path.exists());
    assert_eq!(
        ref_oid(&fixture.root, "refs/heads/feature/changed"),
        changed_oid
    );
}

#[test]
fn symbolic_branch_change_after_candidate_capture_blocks_worktree_mutation() {
    let fixture = GitFixture::new();
    let worktree_path = fixture.add_linked_worktree("feature/symbolic-change");
    run_git(&fixture.root, &["branch", "feature/symbolic-target"]);
    let expected_oid = ref_oid(&fixture.root, "refs/heads/feature/symbolic-change");

    let error = remove_workspace_with_hook(
        &fixture.root,
        &worktree_path,
        "feature/symbolic-change",
        true,
        false,
        || {
            run_git(
                &fixture.root,
                &[
                    "update-ref",
                    "--no-deref",
                    "-d",
                    "refs/heads/feature/symbolic-change",
                    &expected_oid,
                ],
            );
            run_git(
                &fixture.root,
                &[
                    "symbolic-ref",
                    "refs/heads/feature/symbolic-change",
                    "refs/heads/feature/symbolic-target",
                ],
            );
        },
    )
    .unwrap_err();

    assert!(matches!(
        error,
        GitWorkspaceError::WorktreeBranchMismatch { ref actual, .. }
            if actual == "feature/symbolic-target"
    ));
    assert!(worktree_path.exists());
    assert_eq!(
        git_output(
            &fixture.root,
            &["symbolic-ref", "refs/heads/feature/symbolic-change"]
        ),
        "refs/heads/feature/symbolic-target"
    );
    assert!(ref_exists(
        &fixture.root,
        "refs/heads/feature/symbolic-target"
    ));
}

#[cfg(unix)]
#[test]
fn removal_rejects_alias_retargeted_after_candidate_capture() {
    use std::os::unix::fs::symlink;

    let fixture = GitFixture::new();
    let original = fixture.add_linked_worktree("feature/alias-original");
    let other = fixture.add_linked_worktree("feature/alias-other");
    let alias = fixture.tempdir.path().join("worktree alias");
    symlink(&original, &alias).unwrap();

    let error = remove_workspace_with_transaction_hooks(
        &fixture.root,
        &alias,
        "feature/alias-original",
        true,
        false,
        || {
            std::fs::remove_file(&alias).unwrap();
            symlink(&other, &alias).unwrap();
        },
        || {},
    )
    .unwrap_err();

    assert!(matches!(
        error,
        GitWorkspaceError::WorktreeBranchMismatch { .. }
    ));
    assert!(original.exists());
    assert!(other.exists());
    assert!(ref_exists(
        &fixture.root,
        "refs/heads/feature/alias-original"
    ));
    assert!(ref_exists(&fixture.root, "refs/heads/feature/alias-other"));
}

#[test]
fn confirmed_force_removes_unmerged_branch() {
    let fixture = GitFixture::new();
    let worktree_path = fixture.add_linked_worktree("feature/force");
    std::fs::write(worktree_path.join("force.txt"), "force").unwrap();
    run_git(&worktree_path, &["add", "force.txt"]);
    run_git(
        &worktree_path,
        &[
            "-c",
            "user.name=Zap Tests",
            "-c",
            "user.email=zap@example.com",
            "commit",
            "-m",
            "force commit",
        ],
    );

    remove_workspace(&fixture.root, &worktree_path, "feature/force", true, true).unwrap();

    assert!(!worktree_path.exists());
    assert!(!ref_exists(&fixture.root, "refs/heads/feature/force"));
}

#[test]
fn removes_worktree_but_keeps_branch_without_remote_metadata() {
    let fixture = GitFixture::new();
    let worktree_path = fixture.add_linked_worktree("feature/keep");
    run_git(&fixture.root, &["remote", "remove", "origin"]);

    let preflight = deletion_preflight(&fixture.root, &worktree_path, false).unwrap();
    assert_eq!(preflight.branch, "feature/keep");
    assert!(preflight.merge_target.is_none());

    remove_workspace(&fixture.root, &worktree_path, "feature/keep", false, false).unwrap();

    assert!(!worktree_path.exists());
    assert!(ref_exists(&fixture.root, "refs/heads/feature/keep"));
}

#[test]
fn locked_deletion_accepts_same_oid_recreation_before_prepare() {
    let fixture = GitFixture::new();
    let worktree = fixture.add_linked_worktree("feature/same-oid-current");
    let branch_ref = "refs/heads/feature/same-oid-current";
    let oid = ref_oid(&fixture.root, branch_ref);

    remove_workspace_with_transaction_hooks(
        &fixture.root,
        &worktree,
        "feature/same-oid-current",
        true,
        false,
        || {
            run_git(&fixture.root, &["update-ref", "-d", branch_ref, &oid]);
            run_git(&fixture.root, &["update-ref", branch_ref, &oid]);
        },
        || {},
    )
    .unwrap();

    assert!(!worktree.exists());
    assert!(!ref_exists(&fixture.root, branch_ref));
}

#[test]
fn locked_deletion_rejects_different_oid_before_prepare() {
    let fixture = GitFixture::new();
    let worktree = fixture.add_linked_worktree("feature/different-oid");
    let branch_ref = "refs/heads/feature/different-oid";
    let old_oid = ref_oid(&fixture.root, branch_ref);
    let changed_oid = fixture_commit_oid(&fixture.root, "changed branch");

    let error = remove_workspace_with_transaction_hooks(
        &fixture.root,
        &worktree,
        "feature/different-oid",
        true,
        false,
        || {
            run_git(
                &fixture.root,
                &["update-ref", branch_ref, &changed_oid, &old_oid],
            )
        },
        || {},
    )
    .unwrap_err();

    assert!(matches!(error, GitWorkspaceError::RefTransaction { .. }));
    assert!(worktree.exists());
    assert_eq!(ref_oid(&fixture.root, branch_ref), changed_oid);
}

#[test]
fn locked_deletion_rejects_target_drift_before_prepare() {
    let fixture = GitFixture::new();
    let worktree = fixture.add_linked_worktree("feature/target-drift");

    let error = remove_workspace_with_transaction_hooks(
        &fixture.root,
        &worktree,
        "feature/target-drift",
        true,
        false,
        || advance_remote_main(&fixture),
        || {},
    )
    .unwrap_err();

    assert!(matches!(error, GitWorkspaceError::RefTransaction { .. }));
    assert!(worktree.exists());
    assert!(ref_exists(&fixture.root, "refs/heads/feature/target-drift"));
}

#[test]
fn force_deletion_does_not_require_remote_metadata() {
    let fixture = GitFixture::new();
    run_git(&fixture.root, &["remote", "remove", "origin"]);
    let worktree = fixture.add_linked_worktree("feature/force-without-remote");

    remove_workspace(
        &fixture.root,
        &worktree,
        "feature/force-without-remote",
        true,
        true,
    )
    .unwrap();

    assert!(!worktree.exists());
    assert!(!ref_exists(
        &fixture.root,
        "refs/heads/feature/force-without-remote"
    ));
}

#[test]
fn locked_deletion_aborts_when_worktree_becomes_dirty_after_prepare() {
    let fixture = GitFixture::new();
    let worktree = fixture.add_linked_worktree("feature/dirty-after-lock");

    let error = remove_workspace_with_transaction_hooks(
        &fixture.root,
        &worktree,
        "feature/dirty-after-lock",
        true,
        false,
        || {},
        || std::fs::write(worktree.join("keep.txt"), "keep").unwrap(),
    )
    .unwrap_err();

    assert!(matches!(error, GitWorkspaceError::DirtyWorktree { .. }));
    assert!(worktree.exists());
    assert!(ref_exists(
        &fixture.root,
        "refs/heads/feature/dirty-after-lock"
    ));
}

#[test]
fn locked_deletion_aborts_when_target_selection_changes_after_prepare() {
    let fixture = GitFixture::new();
    let worktree = fixture.add_linked_worktree("feature/target-selection");
    run_git(&fixture.root, &["push", "origin", "main:refs/heads/other"]);
    run_git(&fixture.root, &["fetch", "origin"]);

    let error = remove_workspace_with_transaction_hooks(
        &fixture.root,
        &worktree,
        "feature/target-selection",
        true,
        false,
        || {},
        || {
            run_git(
                &fixture.root,
                &[
                    "branch",
                    "--set-upstream-to=origin/other",
                    "feature/target-selection",
                ],
            );
        },
    )
    .unwrap_err();

    assert!(matches!(
        error,
        GitWorkspaceError::MergeTargetChanged { .. }
    ));
    assert!(worktree.exists());
    assert!(ref_exists(
        &fixture.root,
        "refs/heads/feature/target-selection"
    ));
}

#[test]
fn worktree_remove_failure_aborts_and_preserves_branch() {
    let fixture = GitFixture::new();
    let worktree = fixture.add_linked_worktree("feature/remove-failure");

    let error = remove_workspace_with_remove_runner(
        &fixture.root,
        &worktree,
        "feature/remove-failure",
        true,
        false,
        |_repository, _path| Err(injected_command_error("remove worktree")),
    )
    .unwrap_err();

    assert!(matches!(error, GitWorkspaceError::CommandFailed { .. }));
    assert!(worktree.exists());
    assert!(ref_exists(
        &fixture.root,
        "refs/heads/feature/remove-failure"
    ));
}

#[test]
fn commit_failure_reports_removed_worktree_partial_state() {
    let fixture = GitFixture::new();
    let worktree = fixture.add_linked_worktree("feature/commit-failure");

    let error = remove_workspace_with_after_remove_hook(
        &fixture.root,
        &worktree,
        "feature/commit-failure",
        true,
        false,
        |transaction| transaction.terminate_for_test(),
    )
    .unwrap_err();

    assert!(matches!(
        error,
        GitWorkspaceError::BranchDeleteTransactionFailed {
            worktree_removed: true,
            ..
        }
    ));
    assert!(!worktree.exists());
}

#[test]
fn wrapper_errors_expose_primary_sources() {
    use std::error::Error;

    let fixture = GitFixture::new();
    let target = fixture.tempdir.path().join("source chain target");
    let creation = create_from_remote_with_runner(
        &fixture.root,
        "refs/remotes/origin/main",
        "feature/source-chain",
        &target,
        |_repository, _remote_ref, _branch, _target, _expected_oid| {
            Err(injected_command_error("create worktree"))
        },
    )
    .unwrap_err();

    assert!(
        matches!(creation.source(), Some(source) if source.to_string().contains("create worktree"))
    );
}

#[tokio::test]
async fn async_worktree_wrappers_return_creation_preflight_and_removal_results() {
    let fixture = GitFixture::new();
    let worktree_path = fixture.tempdir.path().join("async worktree");

    create_from_remote_async(
        fixture.root.clone(),
        "refs/remotes/origin/main".to_string(),
        "feature/async".to_string(),
        worktree_path.clone(),
    )
    .await
    .unwrap();
    let preflight = deletion_preflight_async(fixture.root.clone(), worktree_path.clone(), true)
        .await
        .unwrap();
    assert_eq!(preflight.branch, "feature/async");
    assert!(preflight.merge_target.as_ref().unwrap().is_merged);

    remove_workspace_async(
        fixture.root.clone(),
        worktree_path.clone(),
        "feature/async".to_string(),
        true,
        false,
    )
    .await
    .unwrap();

    assert!(!worktree_path.exists());
    assert!(!ref_exists(&fixture.root, "refs/heads/feature/async"));
}

#[tokio::test]
async fn create_from_local_async_returns_created_worktree() {
    let fixture = GitFixture::new();
    run_git(&fixture.root, &["branch", "feature/async-local"]);
    let worktree_path = fixture.tempdir.path().join("async local worktree");

    create_from_local_async(
        fixture.root.clone(),
        "feature/async-local".to_string(),
        worktree_path.clone(),
    )
    .await
    .unwrap();

    assert_eq!(current_branch(&worktree_path), "feature/async-local");
    remove_workspace_async(
        fixture.root.clone(),
        worktree_path.clone(),
        "feature/async-local".to_string(),
        true,
        false,
    )
    .await
    .unwrap();
    assert!(!worktree_path.exists());
}

#[tokio::test]
async fn async_wrappers_run_git_operations_off_the_calling_task() {
    let fixture = GitFixture::new();
    let target = fixture.tempdir.path().join("async clone");

    let validated = validate_repository_async(fixture.root.clone())
        .await
        .unwrap();
    let refs = fetch_and_list_refs_async(fixture.root.clone())
        .await
        .unwrap();
    let worktrees = list_worktrees_async(fixture.root.clone()).await.unwrap();
    let cloned = clone_repository_async(
        fixture.remote.to_str().unwrap().to_string(),
        Some(target.clone()),
    )
    .await
    .unwrap();

    assert_eq!(validated.root, fixture.root.canonicalize().unwrap());
    assert!(!refs.is_empty());
    assert_eq!(worktrees.len(), 1);
    assert_eq!(cloned.root, target.canonicalize().unwrap());
}

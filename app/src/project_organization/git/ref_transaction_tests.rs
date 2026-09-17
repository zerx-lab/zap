use std::path::{Path, PathBuf};

use super::{LockedRef, PreparedRefDelete, RefTransactionError, RefTransactionStage};

struct TransactionFixture {
    _tempdir: tempfile::TempDir,
    root: PathBuf,
}

impl TransactionFixture {
    fn new() -> Self {
        let tempdir = tempfile::tempdir().unwrap();
        let root = tempdir.path().join("repository");
        std::fs::create_dir(&root).unwrap();
        run_git(&root, &["init", "-b", "main"]);
        run_git(&root, &["config", "user.name", "Zap Tests"]);
        run_git(&root, &["config", "user.email", "zap@example.com"]);
        std::fs::write(root.join("README.md"), "fixture\n").unwrap();
        run_git(&root, &["add", "README.md"]);
        run_git(&root, &["commit", "-m", "init"]);

        Self {
            _tempdir: tempdir,
            root,
        }
    }

    fn add_worktree(&self, branch: &str) -> PathBuf {
        let worktree = self
            .root
            .parent()
            .unwrap()
            .join(format!("worktree-{}", branch.replace('/', "-")));
        let status = command::blocking::Command::new("git")
            .arg("-C")
            .arg(&self.root)
            .args(["worktree", "add", "-b", branch])
            .arg(&worktree)
            .status()
            .unwrap();
        assert!(status.success());
        worktree
    }

    fn rev_parse(&self, full_ref: &str) -> String {
        let output = command::blocking::Command::new("git")
            .arg("-C")
            .arg(&self.root)
            .args(["rev-parse", "--verify", full_ref])
            .output()
            .unwrap();
        assert!(output.status.success());
        String::from_utf8(output.stdout).unwrap().trim().to_string()
    }

    fn ref_exists(&self, full_ref: &str) -> bool {
        command::blocking::Command::new("git")
            .arg("-C")
            .arg(&self.root)
            .args(["show-ref", "--verify", "--quiet", full_ref])
            .status()
            .unwrap()
            .success()
    }

    fn advance_ref(&self, full_ref: &str) -> String {
        let expected_oid = self.rev_parse(full_ref);
        let tree_ref = format!("{full_ref}^{{tree}}");
        let tree_oid = self.rev_parse(&tree_ref);
        let output = command::blocking::Command::new("git")
            .arg("-C")
            .arg(&self.root)
            .args([
                "commit-tree",
                &tree_oid,
                "-p",
                &expected_oid,
                "-m",
                "advance",
            ])
            .output()
            .unwrap();
        assert!(output.status.success());
        let changed_oid = String::from_utf8(output.stdout).unwrap().trim().to_string();
        run_git(
            &self.root,
            &["update-ref", full_ref, &changed_oid, &expected_oid],
        );
        changed_oid
    }
}

fn run_git(repository: &Path, args: &[&str]) {
    let status = command::blocking::Command::new("git")
        .arg("-C")
        .arg(repository)
        .args(args)
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?} failed");
}

#[test]
fn prepared_delete_transaction_spans_worktree_remove() {
    let fixture = TransactionFixture::new();
    let worktree = fixture.add_worktree("feature/transaction");
    let branch_ref = "refs/heads/feature/transaction";
    let branch_oid = fixture.rev_parse(branch_ref);
    let target_ref = "refs/heads/main";
    let target_oid = fixture.rev_parse(target_ref);
    let transaction = PreparedRefDelete::prepare(
        &fixture.root,
        branch_ref,
        &branch_oid,
        Some(LockedRef {
            full_ref: target_ref,
            oid: &target_oid,
        }),
    )
    .unwrap();
    let status = command::blocking::Command::new("git")
        .arg("-C")
        .arg(&fixture.root)
        .args(["worktree", "remove"])
        .arg(&worktree)
        .status()
        .unwrap();
    assert!(status.success());
    transaction.commit().unwrap();
    assert!(!worktree.exists());
    assert!(!fixture.ref_exists(branch_ref));
}

#[test]
fn prepare_rejects_merge_target_equal_to_branch() {
    let fixture = TransactionFixture::new();
    let worktree = fixture.add_worktree("feature/self-target");
    let branch_ref = "refs/heads/feature/self-target";
    let branch_oid = fixture.rev_parse(branch_ref);

    let error = PreparedRefDelete::prepare(
        &fixture.root,
        branch_ref,
        &branch_oid,
        Some(LockedRef {
            full_ref: branch_ref,
            oid: &branch_oid,
        }),
    )
    .unwrap_err();

    assert!(matches!(
        error,
        RefTransactionError::ConflictingRefUpdate { full_ref } if full_ref == branch_ref
    ));
    assert!(worktree.exists());
    assert!(fixture.ref_exists(branch_ref));
}

#[test]
fn prepared_transaction_blocks_branch_updates_until_abort() {
    let fixture = TransactionFixture::new();
    let branch_ref = "refs/heads/feature/locked";
    let worktree = fixture.add_worktree("feature/locked");
    let branch_oid = fixture.rev_parse(branch_ref);
    let changed_oid = fixture.advance_ref("refs/heads/main");
    let transaction =
        PreparedRefDelete::prepare(&fixture.root, branch_ref, &branch_oid, None).unwrap();
    let output = command::blocking::Command::new("git")
        .arg("-C")
        .arg(&fixture.root)
        .args(["update-ref", branch_ref, &changed_oid, &branch_oid])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("cannot lock ref"));
    transaction.abort().unwrap();

    let status = command::blocking::Command::new("git")
        .arg("-C")
        .arg(&fixture.root)
        .args(["update-ref", branch_ref, &changed_oid, &branch_oid])
        .status()
        .unwrap();
    assert!(status.success());

    assert!(worktree.exists());
    assert_eq!(fixture.rev_parse(branch_ref), changed_oid);
}

#[test]
fn dropping_prepared_transaction_releases_branch_lock() {
    let fixture = TransactionFixture::new();
    let branch_ref = "refs/heads/feature/drop-lock";
    let worktree = fixture.add_worktree("feature/drop-lock");
    let branch_oid = fixture.rev_parse(branch_ref);
    let changed_oid = fixture.advance_ref("refs/heads/main");
    let transaction =
        PreparedRefDelete::prepare(&fixture.root, branch_ref, &branch_oid, None).unwrap();

    drop(transaction);

    let status = command::blocking::Command::new("git")
        .arg("-C")
        .arg(&fixture.root)
        .args(["update-ref", branch_ref, &changed_oid, &branch_oid])
        .status()
        .unwrap();
    assert!(status.success());
    assert!(worktree.exists());
    assert_eq!(fixture.rev_parse(branch_ref), changed_oid);
}

#[test]
fn prepare_rejects_changed_branch_oid() {
    let fixture = TransactionFixture::new();
    fixture.add_worktree("feature/branch-drift");
    let branch_ref = "refs/heads/feature/branch-drift";
    let stale_branch_oid = fixture.rev_parse(branch_ref);
    let changed_branch_oid = fixture.advance_ref(branch_ref);

    let error =
        PreparedRefDelete::prepare(&fixture.root, branch_ref, &stale_branch_oid, None).unwrap_err();

    assert!(matches!(
        error,
        RefTransactionError::ProcessExited {
            stage: RefTransactionStage::Prepare,
            ..
        } | RefTransactionError::UnexpectedResponse {
            stage: RefTransactionStage::Prepare,
            ..
        }
    ));
    assert_eq!(fixture.rev_parse(branch_ref), changed_branch_oid);
}

#[test]
fn prepare_rejects_changed_merge_target() {
    let fixture = TransactionFixture::new();
    fixture.add_worktree("feature/target-drift");
    let branch_ref = "refs/heads/feature/target-drift";
    let branch_oid = fixture.rev_parse(branch_ref);
    let target_ref = "refs/heads/main";
    let stale_target_oid = fixture.rev_parse(target_ref);
    fixture.advance_ref(target_ref);
    let error = PreparedRefDelete::prepare(
        &fixture.root,
        branch_ref,
        &branch_oid,
        Some(LockedRef {
            full_ref: target_ref,
            oid: &stale_target_oid,
        }),
    )
    .unwrap_err();
    assert!(matches!(
        error,
        RefTransactionError::ProcessExited {
            stage: RefTransactionStage::Prepare,
            ..
        } | RefTransactionError::UnexpectedResponse {
            stage: RefTransactionStage::Prepare,
            ..
        }
    ));
}

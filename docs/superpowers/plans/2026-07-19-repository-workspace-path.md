# Repository Workspace Nested Path Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 让 repository workspace 在默认的多级 worktree 路径父目录不存在时也能成功创建。

**Architecture:** 保持 `TargetDirectoryClaim` 对最终目标目录使用 `create_dir` 的独占 claim 语义，只在 claim 前递归创建非空父目录。Git worktree 创建、验证、持久化和失败清理流程不变；回归测试覆盖 remote branch 和 local branch 两条创建路径。

**Tech Stack:** Rust 2021, Cargo, `std::fs`, Git CLI, Warp `warp` app crate 单元测试。

---

### Task 1: 为缺失父目录的 remote/local worktree 创建增加失败测试

**Files:**
- Modify: `app/src/project_organization/git_tests.rs:870-990`

- [ ] **Step 1: 写 remote nested path 回归测试**

在现有 `creates_new_branch_from_remote_ref_without_tracking` 测试之后增加：

```rust
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
```

- [ ] **Step 2: 写 local nested path 回归测试**

在现有 `creates_local_worktree_for_relative_claimed_target` 测试之后增加：

```rust
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
```

- [ ] **Step 3: 运行新增测试确认当前实现失败**

Run:

```bash
cargo test -p warp --lib nested_claimed_target
```

Expected: 测试失败，错误链包含 `TargetClaimFailed` 或底层 `No such file or directory`，因为 `TargetDirectoryClaim::acquire` 当前只调用 `create_dir`。

### Task 2: 在 claim 前创建缺失的父目录

**Files:**
- Modify: `app/src/project_organization/git.rs:1618-1634`

- [ ] **Step 1: 在 `TargetDirectoryClaim::acquire` 中准备父目录**

在最终目标目录的 `create_dir` 前加入父目录创建；跳过空的相对路径父组件，保持现有相对路径测试兼容：

```rust
if let Some(parent) = path
    .parent()
    .filter(|parent| !parent.as_os_str().is_empty())
{
    std::fs::create_dir_all(parent).map_err(|source| GitWorkspaceError::TargetClaimFailed {
        path: path.to_path_buf(),
        source,
    })?;
}
```

保留后续现有的 `create_dir(path)`、`AlreadyExists` 映射、canonicalize 和 `TargetDirectoryClaim` 构造代码不变。父目录不在 claim 结构中，后续失败清理仍只删除最终空目标目录。

- [ ] **Step 2: 运行两条回归测试确认通过**

Run:

```bash
cargo test -p warp --lib nested_claimed_target
```

Expected: 两条测试通过，Git branch 与 worktree 路径均正确。

### Task 3: 回归验证并检查变更范围

**Files:**
- Verify: `app/src/project_organization/git.rs`
- Verify: `app/src/project_organization/git_tests.rs`

- [ ] **Step 1: 运行整个 project organization Git 测试模块**

Run:

```bash
cargo test -p warp --lib project_organization::git_tests
```

Expected: 该模块测试全部通过。

- [ ] **Step 2: 运行格式和编译检查**

Run:

```bash
cargo fmt --all -- --check
cargo check -p warp
```

Expected: 格式检查退出码为 0，`cargo check -p warp` 退出码为 0。

- [ ] **Step 3: 检查最终 diff**

Run:

```bash
git diff --check
git status --short
git diff -- app/src/project_organization/git.rs app/src/project_organization/git_tests.rs
```

Expected: 只有父目录准备逻辑和两个嵌套路径测试发生变更，不包含无关格式化或行为修改。

- [ ] **Step 4: 提交实现**

```bash
git add app/src/project_organization/git.rs app/src/project_organization/git_tests.rs
git commit -m "fix: create parent directories for repository workspaces"
```

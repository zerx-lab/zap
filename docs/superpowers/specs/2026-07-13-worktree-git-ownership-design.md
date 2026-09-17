# Worktree Git Ownership 与安全删除设计

## 背景

Repository Workspaces 的 Task 5 已实现 worktree 创建、删除 preflight 和分支删除，但质量审查发现以下并发安全问题：

1. 仅比较 ref 名称和 commit OID 无法识别同 OID 的 delete/recreate ABA，创建失败 cleanup 或 workspace 删除可能误删其他操作重新创建的 branch。
2. 安全删除只保存 merge target 名称。upstream 或默认分支在 preflight 后后退时，旧的 merged 结论会失效。
3. 目标路径的 `symlink_metadata` 检查与 `git worktree add` 之间仍有 TOCTOU；Git 会接受并接管并发创建的空目录。
4. Remote worktree 创建成功后的验证没有检查 upstream 仍为空。
5. 部分包装错误没有通过 `std::error::Error::source()` 暴露主底层错误。

本设计优先保证不误删用户数据。创建失败时无法证明 branch ownership，就保留残留并返回结构化错误；删除时把权威校验移动到 ref transaction 持锁之后，消除校验完成后的 ref 漂移窗口。

## 目标

- 关闭权威删除校验与 branch mutation 之间的同 OID ABA 窗口。
- 在权威校验和 mutation 期间锁定 branch ref 与 merge target ref。
- 原子 claim worktree 目标目录，避免接管并发创建的空目录。
- 创建成功后验证 worktree 注册状态、attached branch、branch OID 和 upstream 后置条件。
- 对无法自动补偿的残留状态提供完整、可操作、可追踪的结构化错误。
- 保持所有 Git 参数独立传递，Path 使用 `OsStr`，blocking Git 继续由 async wrapper 调度到后台线程。

## 非目标

- 不在 Git 服务中实现数据库、Workspace UI 或页签补偿事务。
- 不尝试让多个 Git/文件系统操作成为真正的跨资源原子事务。
- 不在无法证明 ownership 时猜测 branch 是否由本次操作创建。
- 不追踪跨 UI preflight 的历史 branch generation；同名、同 OID branch 若在 prepared transaction 获取锁前被重建，按当前 branch 重新校验。
- 不为外部进程直接篡改 `.git` 内部文件提供安全保证；支持范围是标准 Git 命令产生的状态变化。

## 方案比较

### 方案 A：残留优先创建 + 锁内权威删除校验

创建失败时不自动删除无法证明 ownership 的 branch。删除时先 prepare `git update-ref --stdin` transaction，锁定 branch 和 merge target，再重新执行 worktree、branch、dirty 和 merge 权威校验，随后跨越 `git worktree remove` 提交 branch delete。

优点：权威校验完成后相关 refs 不再漂移，不需要不可移植的文件 identity 或持久化 ownership marker。缺点：锁前发生的同名同 OID 重建按当前 branch 处理；实现必须处理 Git transaction 协议与部分 mutation。

### 方案 B：使用完整 reflog SHA-256 作为 generation token

在 preflight 和 mutation 之间比较完整 branch reflog 的 SHA-256。

优点：实现表面简单。缺点：实证表明 Git 删除 branch 时会删除 reflog；用相同 OID、committer 和时间重建 branch 可生成字节完全相同的 reflog，因此该 token 可重放，不能证明 generation。

### 方案 C：永不自动删除 branch

创建失败和 workspace 删除都只移除 worktree，始终保留 branch。

优点：数据安全边界最简单。缺点：实质取消“同时删除本地分支”，产品行为退化。

采用方案 A。

## 创建流程

### 目标目录 claim

1. 完成 repository、remote ref、branch name 和 branch 不存在等只读校验。
2. 使用 `std::fs::create_dir(worktree_path)` 原子 claim 目标路径。
3. `AlreadyExists` 返回 `TargetExists`；其他 I/O 错误返回结构化 claim 错误。
4. claim 成功后记录该目录由本次调用创建。

并发方在 claim 后调用 `create_dir` 会得到 `AlreadyExists`。本设计不把“同一用户主动删除并替换已 claim 目录”视为普通并发场景；创建完成后仍会重新验证 canonical registered path，发现身份或注册状态异常时返回残留错误。

### Git 创建

目标目录已由本次调用 claim 后，执行：

```text
git -C <repository> worktree add --no-track -b <new-branch> <claimed-path> <remote-ref>
```

branch 由同一个 Git 命令创建，不再在命令前创建最终 local ref。若命令因并发 branch、Git I/O 或其他原因失败，Git 服务不自动删除可能存在的 branch，因为 ref 名称和 OID 无法证明其 generation ownership。

### 创建后验证

Git 命令成功后验证：

1. `list_worktrees` 中仅有一个 canonical registered path 与目标一致。
2. worktree 为 attached、非 bare，并指向 `refs/heads/<new-branch>`。
3. local branch 为 direct ref，OID 等于 remote ref 创建前解析的 expected OID。
4. local branch upstream 为空。

任一验证失败时返回 `WorktreeCreationVerificationFailed`。错误包含 worktree path、branch、expected OID、实际 direct/symbolic 状态和 upstream。该路径不删除无法证明 ownership 的 branch 或 worktree，而是明确报告残留，交由后续模型补偿与启动一致性检查处理。

### 创建失败目录清理

Git 命令失败后：

1. 检查目标是否已注册为 worktree；已注册时不删除目录。
2. 未注册且目录仍为空时，删除本次 claim 的目录。
3. 目录不为空、类型变化或检查失败时保留目录，并在错误中记录 cleanup failure。
4. 无论目录是否清理，均不自动删除可能残留的 local branch。

`WorktreeCreationFailed` 必须保留原始 Git stderr/IO 错误，并显式提供 `branch_may_remain`、`worktree_registered`、`claimed_directory_removed` 等残留状态。

## 删除 preflight

`DeletionPreflight` 继续提供 UI 确认所需的只读快照：

- canonical registered worktree path；
- branch full ref 与 branch OID；
- merge target full ref 与 merge target OID；
- dirty、attached 和 merge 结论。

该快照只用于展示和确认，不是 mutation 的 ownership 证明。`remove_workspace` 不接收或信任较早的 `DeletionPreflight`；它在本次删除调用中读取 transaction 候选值，并在 transaction prepared 后重新执行权威校验。

### Merge target

- 有 upstream 时保存 upstream full ref 与解析后的 OID。
- 无 upstream 时保存 primary remote default branch full ref 与 OID。
- `merge-base --is-ancestor <branch-oid> <merge-target-oid>` 使用固定 OID，而不是后续可变 ref 名称。
- merge target full ref 与待删 branch full ref 相同时，在启动 transaction 前返回结构化 invalid-target 错误；不向同一个 ref 排队 `verify` 和 `delete`。

## 持锁删除事务

branch 删除使用 `git update-ref --stdin` 子进程。命令 stdin/stdout/stderr 均通过 `crates/command` 管理。

### Transaction 准备

普通安全删除向 transaction 发送：

```text
start
verify <merge-target-ref> <merge-target-oid>
delete <branch-ref> <branch-oid>
prepare
```

force delete 只发送带 expected old OID 的 `delete`，不验证 merge target。`delete <branch-ref> <branch-oid>` 本身同时执行 branch compare-and-delete；不能再为同一 branch ref 添加 `verify`，因为 Git 会以 `multiple updates for ref` 拒绝 transaction。

`prepare` 成功后 Git 持有相关 ref locks，pending delete 尚未提交。实现必须解析每个协议响应；未知、缺失或失败响应均 abort 并返回结构化 transaction 错误。

### 锁内验证与 mutation

1. 重新读取 `list_worktrees`，要求 canonical registered path 唯一、非 bare、非 detached，并仍 attached 到请求的 local branch。
2. 重新读取 branch direct ref，要求 OID 等于 transaction 中的 expected OID，且不是 symbolic ref。
3. 重新检查 worktree status，存在 tracked 或 untracked change 时 abort。
4. 非 force 删除重新解析 upstream 或 primary remote default branch，要求 target full ref 仍等于 transaction 锁定的 target ref。
5. 使用锁定的 branch OID 和 target OID 重新执行 `merge-base --is-ancestor`；未合入时 abort。
6. 全部校验通过后执行 `git worktree remove <canonical-registered-path>`。
7. worktree remove 失败时发送 `abort`，branch 保留。
8. worktree remove 成功后发送 `commit`，提交 branch delete。

在读取候选值与 `prepare` 之间，branch 改到不同 OID 或 merge target 漂移会使 transaction prepare 失败。同名、同 OID branch 若在 `prepare` 前被重建，按选定语义作为当前 branch 进入锁内权威校验；`prepare` 之后的 ref mutation 被 Git ref lock 阻止。

真实 Git prototype 已验证：只对 merge target 使用 `verify`、对 branch 使用带 expected OID 的 `delete` 时，prepared transaction 可以跨越 `git worktree remove` 并成功 commit。

### 部分 mutation

worktree remove 成功但 transaction commit、响应读取或子进程等待失败时，返回 `BranchDeleteTransactionFailed`，明确包含：

- canonical worktree path；
- `worktree_removed: true`；
- branch/merge-target ref 与 expected OID；
- transaction 阶段；
- Git stderr/IO source；
- branch 当前状态检查结果或检查错误。

不得把该状态包装成普通 command error。

## Target 目录身份与清理

目标目录通过原子 `create_dir` claim。创建命令前和创建后均验证该路径仍为目录；Git 成功后以 canonical registered path 作为最终身份。

失败清理只删除仍为空且未注册的 claimed 目录。目录中出现任何内容时不递归删除，避免清理并发方或用户写入的数据。

## 错误链

- 单一主底层错误字段使用 `#[source]`。
- 同时存在 operation error 和 cleanup/inspection error 时，operation error 为主 source；次级错误保留在字段和 Display 中。
- transaction protocol、abort、commit、wait 和 post-failure inspection 分别保留阶段信息。
- 所有用户可见错误保留 Git stderr 中的关键原因。

## 测试设计

### 创建

- claim 前目标不存在，claim 后并发 `create_dir` 返回 `AlreadyExists`。
- 并发创建空目标目录不能被 Git 静默接管。
- Git 创建失败时可能残留 branch，错误明确报告且不自动删除 branch。
- 成功后 hook 设置 upstream，verification 返回结构化残留错误。
- registered path、attached branch、direct OID 和 upstream none 的正常成功路径。
- claimed 目录仅在未注册且为空时清理；非空目录保留。

### 删除

- 同 OID delete/recreate 若发生在 transaction prepare 前，按当前 branch 完成锁内权威校验；prepare 后的 ref mutation 被锁拒绝。
- branch 改到不同 OID 或 merge target 在候选读取后漂移，transaction `delete`/`verify`/`prepare` 失败且无 mutation。
- transaction prepared 后 worktree path、attached branch、dirty 状态或 target selection 改变，锁内权威校验失败并 abort。
- prepared transaction 下 worktree remove 成功，commit 后 branch 消失。
- worktree remove 失败触发 abort，branch 保留。
- commit/IO/协议失败明确报告部分 mutation。
- force delete 跳过 merge target verify 和 merge 判断，但不跳过 branch OID、worktree identity、attached branch 和 dirty 校验。

### 错误链

- 对主包装错误调用 `source()` 能获得底层 CommandFailed/CommandIo/transaction error。
- 次级 cleanup/inspection 错误仍出现在 Display 和结构化字段中。

## 风险与约束

- 锁前发生的同名、同 OID branch 重建不会作为历史 identity change 单独拒绝；它必须通过锁内全部当前状态校验。
- ref transaction 不锁 worktree metadata 或 repository config，因此所有依赖这些状态的判断都在 prepared 后重做；最终 `git worktree remove` 仍作为 dirty/locked 等 Git 约束的最后防线。
- 当前 Git prototype 已通过，但 transaction protocol 与 `worktree remove` 的组合仍需在支持平台的测试环境验证。
- `git.rs` 已较大。实现计划应把 transaction protocol 拆为职责单一的私有单元；不新增 reflog 模块，不做与 Task 5 无关的重构。

## 规格同步

实现时需要同步更新 `specs/repository-workspaces/TECH.md`：

- 创建失败无法证明 branch ownership 时保留 branch 并报告残留；
- 目标目录通过原子 claim；
- 删除使用 prepared ref transaction，并在持锁后执行权威校验；
- merge target 以 ref + OID 固定，在 transaction 中验证并在锁内重新确认 target selection。

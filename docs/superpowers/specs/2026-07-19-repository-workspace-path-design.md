# Repository Workspace 路径创建修复设计

## 背景

创建 repository workspace 时，创建窗口默认生成多级路径：

```text
~/.warp/worktrees/<repository>/<branch>
```

`TargetDirectoryClaim::acquire` 使用 `std::fs::create_dir` 直接创建最终目录。当 `.warp`、`worktrees` 或 repository 目录尚不存在时，系统返回 `No such file or directory`，导致点击确认后 workspace 创建失败。

## 目标

- 允许默认的多级 workspace 路径在父目录不存在时正常创建。
- 保留最终目标目录的原子占位语义：目标已存在时仍返回 `TargetExists`。
- 不改变现有 Git worktree 创建、验证、清理和 UI 流程。

## 方案

在 `TargetDirectoryClaim::acquire` 中先确保目标的父目录存在，再使用 `create_dir` 创建最终目标目录：

1. 对 `path.parent()` 调用 `create_dir_all`。
2. 对 `path` 调用现有的 `create_dir`，继续区分 `AlreadyExists` 和其他 claim 错误。
3. 成功后继续 canonicalize 目标路径并返回 claim。

父目录创建属于路径准备，不改变目标目录的 claim 竞争行为。已有的失败清理只负责删除由 claim 创建的最终空目录，因此不扩展为删除父目录，避免误删可能由其他流程共享的目录。

## 错误处理

- 父目录创建失败时直接返回 `TargetClaimFailed`，错误路径为原始目标路径，保留底层 `io::Error`。
- 最终目录已存在时仍返回 `TargetExists`。
- 最终目录创建成功后，后续 Git 操作和现有清理逻辑保持不变。

## 测试

在 `app/src/project_organization/git_tests.rs` 增加回归测试，使用临时目录下不存在的嵌套目标路径，分别验证：

- 从 remote ref 创建 worktree 会创建所有父目录并成功完成。
- 从 local branch 创建 worktree 会创建所有父目录并成功完成。

测试同时验证目标目录存在且 Git worktree 已注册，覆盖用户点击确认时使用的默认路径形态。

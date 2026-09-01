# Project Organization Persistence Acknowledgement Design

## 背景

`ProjectOrganizationModel` 当前通过异步 `ModelEvent` 向 SQLite writer 发送 repository/workspace CRUD。发送成功只表示事件进入队列，不表示 SQLite 已提交。writer 暂停、线程退出或数据库写入失败时，模型仍可能更新内存并发送 UI event，导致当前进程显示成功、重启后数据丢失。

持久化路径还存在恢复后的唯一性问题。启动时不可访问的 persisted path 必须原样保留，以便后续 health reconciliation；路径恢复后，多个历史 alias 可能收敛到同一 canonical path。现有单值查找无法可靠区分唯一匹配和歧义。

## 目标

- repository/workspace CRUD 只有在 SQLite writer 确认提交后才更新内存和发送领域事件。
- writer 不可用、暂停、线程断开或 SQLite 写入失败时返回结构化错误，且不产生内存半状态。
- 不改造其他既有 `ModelEvent` 的异步语义。
- repository/workspace path lookup 统一返回零个、一个或多个 canonical 匹配。
- 多匹配必须失败并暴露冲突记录，不能依赖 `HashMap` 遍历顺序。
- 为 Task 4 的 worktree 创建和补偿流程提供可确认的单次数据库 mutation 边界。

## 非目标

- 不为全部 persistence 事件增加 acknowledgement。
- 不在本次改动中实现跨多个 repository/workspace row 的通用事务编排。
- 不实现 pending/failed persistence UI、重试队列或 Task 8 的 health reconciliation。
- 不调整 repository/workspace 树和 modal 视觉设计。

## 方案选择

采用领域专用 acknowledged request，复用现有 SQLite writer 线程。

未采用的方案：

- 通用 `ModelEvent` acknowledgement：需要修改大量无关事件和调用点，超出当前范围。
- 独立 SQLite 写连接：会复制连接生命周期、暂停/重建和事务协调逻辑。

## 持久化请求

新增显式 operation 类型：

```rust
enum RepositoryPersistenceOperation {
    UpsertRepository { repository: model::Repository },
    DeleteRepository { repository_id: String },
    UpsertRepositoryWorkspace { workspace: model::RepositoryWorkspace },
    DeleteRepositoryWorkspace { workspace_id: String },
}
```

`ModelEvent` 增加领域请求变体，携带 operation 和一次性响应通道。旧的四个 fire-and-forget repository/workspace 变体被移除，避免同一领域存在两种写入语义。

模型发送请求后同步等待 writer 响应。repository/workspace 单行 CRUD 是短时本地 SQLite 操作；本设计不用于 clone、fetch 或其他长时操作。

## Writer 行为

SQLite writer 对领域请求执行以下流程：

1. writer 已暂停：不执行 SQL，立即返回 paused 错误。
2. writer 正常：执行对应 CRUD。
3. SQL 成功提交：返回成功。
4. SQL 或 Diesel 失败：返回包含操作上下文的错误，同时保留现有日志/遥测。
5. caller 已断开响应通道：记录错误，但 writer 继续服务后续请求。

sender 不存在、请求发送失败或响应通道断开均由模型映射为 `ProjectOrganizationError::Persistence`。生产环境不允许把缺少 sender 当作成功。测试通过受控 writer/fake responder 显式返回成功或失败。

## 模型提交顺序

所有写操作统一遵循：

1. 完成只读验证和 canonical identity 检查。
2. 构造 persistence operation。
3. 等待 SQLite acknowledgement。
4. 更新内存主表和辅助索引。
5. 发送 `ProjectOrganizationEvent`。

acknowledgement 失败时，第 4、5 步不得发生。删除和更新同样遵循该顺序。

## Canonical Path Resolver

repository 和 workspace 分别提供统一 resolver。输入路径先严格 canonicalize；随后对全部已加载记录计算当前可用的 canonical identity：

- persisted path 当前可 canonicalize：使用 canonical path比较。
- persisted path 当前不可访问：仅保留原 key，不把它映射到其他位置。
- 匹配 0 条：`None`。
- 匹配 1 条：`Unique(id)`。
- 匹配多条：`Ambiguous(ids)`。

`add_local_repository`、`touch_repository_path`、repository path update、workspace insert/update 都使用 resolver。更新自身时从候选中排除自身 ID。

新增结构化错误：

- ambiguous repository canonical path，包含 canonical path 和冲突 repository IDs。
- ambiguous workspace canonical path，包含 canonical path 和冲突 workspace IDs。

任何歧义都必须失败，不能自动选择、合并或删除记录。

## 错误处理

- persistence unavailable/paused/disconnected/SQL failure：操作返回错误，模型和 UI event 不变化。
- duplicate canonical path：返回已有唯一记录的 duplicate 错误。
- ambiguous canonical path：返回全部冲突 IDs，交由后续 reconciliation/UI 处理。
- persisted missing path：继续保留原始 path，不在启动时失败。
- persisted UUID/source/parent reference 损坏：继续 fail-fast。

## 测试策略

### Path identity

- 启动时两个缺失 alias，运行中路径恢复并收敛到同一 repository：add/touch 返回 ambiguity。
- workspace 使用相同场景验证 insert/update ambiguity。
- 唯一恢复 alias 能被 add/touch 识别为已有记录。
- persisted missing path 仍保留；可访问 alias 仍归一化。
- update 改 path/branch 后旧索引失效，新索引生效。
- 成功删除后 repository/workspace/path/branch 索引全部清理。

### Acknowledgement

- 实际 SQLite writer 确认 upsert/delete 后，数据库可从独立连接读取结果。
- SQLite unique/FK 错误通过 acknowledgement 返回调用者。
- writer paused 时请求失败且数据库不变化。
- sender 缺失、sender 断开、response 断开均返回 persistence error。
- ack 失败时模型记录、辅助索引和 `ProjectOrganizationEvent` 均不变化。
- ack 成功时模型和领域事件只发生一次。

## 兼容与后续任务

- 其他 `ModelEvent` 保持现有 fire-and-forget 行为。
- Task 4 创建 workspace 时可使用 acknowledged workspace upsert 作为数据库提交边界；后续步骤失败时使用 acknowledged delete 做补偿。
- Task 8 负责把 missing/ambiguous external state 转换为 health 状态和用户可操作 UI。

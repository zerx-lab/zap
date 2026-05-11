-- 删除 LSP 持久化与"已访问 git 仓库历史"两张表。
-- workspace_language_server 通过 workspace_id FK 引用 workspace_metadata,
-- 因此必须先删 child 表。
DROP TABLE IF EXISTS workspace_language_server;
DROP TABLE IF EXISTS workspace_metadata;

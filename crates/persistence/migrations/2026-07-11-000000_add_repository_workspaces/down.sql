ALTER TABLE windows DROP COLUMN active_repository_workspace_id;
ALTER TABLE tabs DROP COLUMN repository_workspace_id;

DROP TABLE repository_workspace_window_states;
DROP TABLE repository_workspaces;
DROP TABLE repositories;

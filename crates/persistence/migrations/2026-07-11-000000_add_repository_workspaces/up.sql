CREATE TABLE repositories (
  id             TEXT PRIMARY KEY NOT NULL,
  display_name   TEXT NOT NULL,
  path           TEXT NOT NULL UNIQUE,
  remote_url     TEXT,
  source         TEXT NOT NULL CHECK(source IN ('local', 'cloned')),
  created_at     TIMESTAMP NOT NULL,
  last_opened_at TIMESTAMP NOT NULL
);

INSERT INTO repositories (
  id,
  display_name,
  path,
  remote_url,
  source,
  created_at,
  last_opened_at
)
SELECT
  lower(hex(randomblob(4))) || '-' ||
    lower(hex(randomblob(2))) || '-4' ||
    substr(lower(hex(randomblob(2))), 2) || '-' ||
    substr('89ab', abs(random()) % 4 + 1, 1) ||
    substr(lower(hex(randomblob(2))), 2) || '-' ||
    lower(hex(randomblob(6))),
  path,
  path,
  NULL,
  'local',
  added_ts,
  COALESCE(last_opened_ts, added_ts)
FROM projects;

CREATE TABLE repository_workspaces (
  id             TEXT PRIMARY KEY NOT NULL,
  repository_id  TEXT NOT NULL REFERENCES repositories(id) ON DELETE RESTRICT,
  display_name   TEXT NOT NULL,
  branch         TEXT NOT NULL,
  worktree_path  TEXT NOT NULL UNIQUE,
  created_at     TIMESTAMP NOT NULL,
  last_opened_at TIMESTAMP NOT NULL,
  UNIQUE(repository_id, branch)
);

CREATE TABLE repository_workspace_window_states (
  window_id               INTEGER NOT NULL REFERENCES windows(id) ON DELETE CASCADE,
  repository_workspace_id TEXT NOT NULL REFERENCES repository_workspaces(id) ON DELETE CASCADE,
  active_tab_index         INTEGER NOT NULL,
  PRIMARY KEY(window_id, repository_workspace_id)
);

ALTER TABLE tabs ADD COLUMN repository_workspace_id TEXT REFERENCES repository_workspaces(id) ON DELETE SET NULL;
ALTER TABLE windows ADD COLUMN active_repository_workspace_id TEXT REFERENCES repository_workspaces(id) ON DELETE SET NULL;

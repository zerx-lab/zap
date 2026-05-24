CREATE TABLE ssh_servers_backup (
  node_id           TEXT PRIMARY KEY REFERENCES ssh_nodes(id) ON DELETE CASCADE,
  host              TEXT NOT NULL,
  port              INTEGER DEFAULT 22,
  username          TEXT DEFAULT '',
  auth_type         TEXT CHECK(auth_type IN ('password','key')) DEFAULT 'password',
  key_path          TEXT,
  last_connected_at TIMESTAMP
);
INSERT INTO ssh_servers_backup SELECT node_id, host, port, username, auth_type, key_path, last_connected_at FROM ssh_servers;
DROP TABLE ssh_servers;
ALTER TABLE ssh_servers_backup RENAME TO ssh_servers;

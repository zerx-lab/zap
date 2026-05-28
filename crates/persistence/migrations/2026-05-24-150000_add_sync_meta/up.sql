CREATE TABLE IF NOT EXISTS sync_meta (
  key   TEXT PRIMARY KEY NOT NULL,
  value TEXT NOT NULL
);

INSERT OR IGNORE INTO sync_meta (key, value) VALUES ('sync_version', '0');
INSERT OR IGNORE INTO sync_meta (key, value) VALUES ('last_sync_time', '');
INSERT OR IGNORE INTO sync_meta (key, value) VALUES ('last_sync_platform', '');

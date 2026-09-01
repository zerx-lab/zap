CREATE TABLE projects (
  path           TEXT PRIMARY KEY NOT NULL,
  added_ts       TIMESTAMP NOT NULL,
  last_opened_ts TIMESTAMP
);

INSERT INTO projects (path, added_ts, last_opened_ts)
SELECT path, created_at, last_opened_at
FROM repositories;

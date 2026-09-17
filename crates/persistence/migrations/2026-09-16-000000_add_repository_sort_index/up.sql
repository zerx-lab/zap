ALTER TABLE repositories ADD COLUMN sort_index INTEGER NOT NULL DEFAULT 0;

UPDATE repositories
SET sort_index = (
  SELECT COUNT(*)
  FROM repositories AS earlier
  WHERE earlier.created_at < repositories.created_at
     OR (
       earlier.created_at = repositories.created_at
       AND earlier.display_name < repositories.display_name
     )
     OR (
       earlier.created_at = repositories.created_at
       AND earlier.display_name = repositories.display_name
       AND earlier.id < repositories.id
     )
);

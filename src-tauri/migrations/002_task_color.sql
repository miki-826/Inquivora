ALTER TABLE tasks ADD COLUMN color TEXT NOT NULL DEFAULT 'blue'
  CHECK (color IN ('blue', 'indigo', 'violet', 'pink', 'red', 'orange', 'green', 'teal'));

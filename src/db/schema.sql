CREATE TABLE IF NOT EXISTS skills (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    source_type TEXT NOT NULL,
    source_path TEXT,
    kind TEXT NOT NULL DEFAULT 'pool',
    sha TEXT,
    hash TEXT NOT NULL
);

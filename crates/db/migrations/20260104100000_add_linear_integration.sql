-- Add Linear integration support
-- 1. Add linear_api_key to projects
-- 2. Add linear_issue_id to tasks
-- 3. Add 'backlog' to task status CHECK constraint (requires table rebuild)

-- Simple column additions
ALTER TABLE projects ADD COLUMN linear_api_key TEXT;
ALTER TABLE tasks ADD COLUMN linear_issue_id TEXT;

-- Index for Linear issue lookups (to avoid duplicates during sync)
CREATE INDEX idx_tasks_linear_issue_id ON tasks(linear_issue_id) WHERE linear_issue_id IS NOT NULL;

-- Rebuild tasks table to add 'backlog' to status CHECK constraint
-- sqlx workaround: end auto-transaction to allow PRAGMA to take effect
COMMIT;

PRAGMA foreign_keys = OFF;

BEGIN TRANSACTION;

-- Create new tasks table with updated CHECK constraint
-- Note: shared_task_id no longer has FK constraint (shared_tasks table was dropped in migrate_to_electric)
CREATE TABLE tasks_new (
    id                    BLOB PRIMARY KEY,
    project_id            BLOB NOT NULL,
    title                 TEXT NOT NULL,
    description           TEXT,
    status                TEXT NOT NULL DEFAULT 'todo'
                             CHECK (status IN ('backlog','todo','inprogress','inreview','done','cancelled')),
    parent_workspace_id   BLOB,
    shared_task_id        BLOB,
    linear_issue_id       TEXT,
    created_at            TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at            TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);

-- Copy existing data
INSERT INTO tasks_new (id, project_id, title, description, status, parent_workspace_id, shared_task_id, linear_issue_id, created_at, updated_at)
SELECT id, project_id, title, description, status, parent_workspace_id, shared_task_id, linear_issue_id, created_at, updated_at
FROM tasks;

-- Drop old table and rename new one
DROP TABLE tasks;
ALTER TABLE tasks_new RENAME TO tasks;

-- Recreate indexes
CREATE INDEX idx_tasks_project_id ON tasks(project_id);
CREATE INDEX idx_tasks_parent_workspace_id ON tasks(parent_workspace_id);
CREATE INDEX idx_tasks_linear_issue_id ON tasks(linear_issue_id) WHERE linear_issue_id IS NOT NULL;
CREATE UNIQUE INDEX idx_tasks_shared_task_unique ON tasks(shared_task_id) WHERE shared_task_id IS NOT NULL;

-- Verify foreign key constraints
PRAGMA foreign_key_check;

COMMIT;

PRAGMA foreign_keys = ON;

-- sqlx workaround: start empty transaction for sqlx to close gracefully
BEGIN TRANSACTION;

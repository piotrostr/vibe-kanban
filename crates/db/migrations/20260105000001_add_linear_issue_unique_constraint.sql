-- Add UNIQUE constraint on (project_id, linear_issue_id) to prevent duplicates during sync
-- This prevents race conditions where concurrent sync requests could create duplicate tasks

CREATE UNIQUE INDEX idx_tasks_linear_issue_unique
ON tasks(project_id, linear_issue_id)
WHERE linear_issue_id IS NOT NULL;

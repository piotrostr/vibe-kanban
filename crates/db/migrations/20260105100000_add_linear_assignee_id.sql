-- Add Linear assignee ID to projects for filtering backlog issues by assignee
ALTER TABLE projects ADD COLUMN linear_assignee_id TEXT;

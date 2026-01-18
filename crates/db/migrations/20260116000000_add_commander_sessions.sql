-- Commander sessions: project-scoped persistent chat for managing tasks via MCP
CREATE TABLE commander_sessions (
    id              BLOB PRIMARY KEY,
    project_id      BLOB NOT NULL UNIQUE,
    container_ref   TEXT,
    executor        TEXT,
    system_prompt   TEXT NOT NULL DEFAULT 'You are Commander, a repository operator for this project.

RULES:
- NEVER push directly to main/master branches
- NEVER force push to any branch
- Always use PRs for merging changes to main
- For consolidating changes across tickets, create a new branch first
- Use `gh pr merge` for merging approved PRs
- Before any destructive git operation, explain what you are about to do and ask for confirmation

You have access to the vibe-kanban MCP for task management.',
    created_at      TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);

CREATE INDEX idx_commander_sessions_project_id ON commander_sessions(project_id);

-- Add commander_session_id to execution_processes (nullable - processes can belong to either session or commander_session)
ALTER TABLE execution_processes ADD COLUMN commander_session_id BLOB REFERENCES commander_sessions(id) ON DELETE CASCADE;

CREATE INDEX idx_execution_processes_commander_session_id ON execution_processes(commander_session_id);

-- Add 'importedsession' to execution_process run_reason CHECK constraint
-- SQLite requires table recreation to modify CHECK constraints

PRAGMA foreign_keys=OFF;

CREATE TABLE execution_processes_new (
    id TEXT PRIMARY KEY NOT NULL,
    session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    commander_session_id BLOB REFERENCES commander_sessions(id) ON DELETE CASCADE,
    run_reason TEXT NOT NULL
                       CHECK (run_reason IN ('setupscript','codingagent','devserver','cleanupscript','quickcommand','slashcommand','importedsession')),
    executor_action TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'running'
                       CHECK (status IN ('running','completed','failed','killed')),
    exit_code INTEGER,
    dropped INTEGER NOT NULL DEFAULT 0,
    started_at TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

INSERT INTO execution_processes_new SELECT
    id, session_id, commander_session_id, run_reason, executor_action,
    status, exit_code, dropped, started_at, completed_at, created_at, updated_at
FROM execution_processes;

DROP TABLE execution_processes;

ALTER TABLE execution_processes_new RENAME TO execution_processes;

CREATE INDEX idx_execution_processes_session_id ON execution_processes(session_id);
CREATE INDEX idx_execution_processes_status ON execution_processes(status);
CREATE INDEX idx_execution_processes_commander_session_id ON execution_processes(commander_session_id);

PRAGMA foreign_keys=ON;

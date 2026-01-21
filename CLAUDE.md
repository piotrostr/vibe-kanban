## Build and Run Commands (TUI)

```bash
# Run the TUI
cargo run --bin vibe

# Run with logging
RUST_LOG=info cargo run --bin vibe

# Run tests
cargo test -p tui

# Run a single test
cargo test -p tui test_slugify

# Check/lint
cargo clippy -p tui -- -D warnings
cargo fmt --all
```

Logs are written to `~/.vibe/vibe.log`.

## TUI Architecture

The TUI is a Ratatui-based terminal app in `crates/tui/`. It orchestrates Claude Code sessions within git worktrees via Zellij.

### Core Loop (`app.rs`)

The `App` struct owns all state and runs the main loop:
1. Poll background channels for worktree/session/PR updates
2. Render current view
3. Handle keyboard input via action dispatch
4. Exit on quit action

Background loading uses `tokio::task::spawn_blocking` with mpsc channels to avoid blocking the UI thread.

### Module Structure

- **state/** - View state for each screen (kanban, tasks, worktrees, sessions, search, logs). `AppState` in `app_state.rs` aggregates all view states.
- **input/** - `keybindings.rs` maps keys to `Action` enum based on current view. View-specific bindings in separate functions.
- **ui/** - Ratatui rendering functions. One file per view (kanban.rs, worktrees.rs, etc.).
- **storage/** - File-based task storage. Tasks are markdown files in `~/.vibe/projects/{project}/tasks/` with YAML frontmatter.
- **external/** - Shell-out wrappers:
  - `zellij.rs` - Session listing, attach, kill, attention detection
  - `worktrunk.rs` - `wt` CLI wrapper for worktree management
  - `terminal_spawn.rs` - Session launch logic with `wt switch -x`
  - `gh.rs` - GitHub CLI for PR info
  - `editor.rs` - External editor invocation

### Session Launch Flow

1. Task selected -> derive branch name from title
2. Create launcher script that handles session state (new/running/EXITED)
3. Call `wt switch [--create] branch -x launcher.sh` from project directory
4. `wt` switches to worktree, runs launcher which starts/attaches Zellij with Claude

### Key Bindings

View-specific bindings in `input/keybindings.rs`. Global: `q` quit, `?` help, `/` search, `Esc` back.

Kanban: `j/k` navigate, `J/K` change columns, `g` launch session, `p` launch with plan mode, `e` edit, `c` create, `d` delete, `v` view PR, `w` worktrees, `S` sessions.

### Task Storage Format

Markdown files with YAML frontmatter:
```markdown
---
id: uuid
linear_id: TEAM-123  # optional
created: 2024-01-15
---

# Task Title

Description here...
```

Tasks stored at `~/.vibe/projects/{cwd_dirname}/tasks/`.

### Dependencies

- `wt` CLI (worktrunk) - must be installed at `~/.cargo/bin/wt` or set `WORKTRUNK_BIN`
- `zellij` - terminal multiplexer for Claude sessions
- `gh` CLI - optional, for PR status

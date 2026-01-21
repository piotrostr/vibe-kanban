<p align="center">
  <img src="frontend/public/vibe-512.png" alt="Vibe Logo" width="200">
</p>

<p align="center">AI agent orchestration for coding tasks</p>

<p align="center">
<img width="6400" height="2620" alt="image" src="https://github.com/user-attachments/assets/f45434b5-f177-4277-a42c-8318567573a6" />
</p>

Fork of [BloopAI/vibe-kanban](https://github.com/BloopAI/vibe-kanban) - [vibekanban.com](https://www.vibekanban.com/).

## What's Different

- Tauri desktop app for macOS (native menu bar)
- Native system notifications
- Privacy mode (screenshot-safe UI)
- Linear integration (bidirectional sync with labels and issue IDs)
- GitHub PR status badges and PR binding
- No analytics or tracking
- Gruvbox theme + Liga SFMono font
- PWA support (installable web app)
- Unified task board with project filter sidebar
- Claude Code-style tool call rendering

## Install

Desktop (macOS):

```bash
task desktop-install
```

Development:

```bash
task dev
```

Production build:

```bash
task start
```

## Development

Prerequisites:

- [Rust](https://rustup.rs/) (latest stable)
- [Node.js](https://nodejs.org/) (>=18)
- [pnpm](https://pnpm.io/) (>=8)
- [Task](https://taskfile.dev/) (task runner)

Additional tools:

```bash
cargo install cargo-watch
cargo install sqlx-cli
cargo install tauri-cli
```

Key commands:

| Command | Description |
|---------|-------------|
| `task dev` | Hot-reload dev server (frontend + backend) |
| `task check` | Type-check frontend + backend |
| `task test` | Run Rust tests |
| `task lint` | Lint all code |
| `task format` | Format all code |
| `task build` | Production build |
| `task desktop-install` | Build and install macOS app |

## TUI (Terminal UI)

A standalone terminal-based kanban board that works with Zellij sessions:

```bash
cargo install --path crates/tui
vibe
```

Features:
- File-based task storage (`.vibe/tasks/*.md`)
- Zellij session management for Claude Code
- Git worktree integration
- PR status tracking via `gh` CLI

### Claude Activity Indication (Optional)

For real-time Claude session status indicators (thinking/waiting/idle), configure Claude Code's statusline:

1. Create `~/.vibe/claude-statusline.sh`:

```bash
#!/bin/bash
STATE_DIR="$HOME/.vibe/claude-activity"
mkdir -p "$STATE_DIR"

input=$(cat)
working_dir=$(echo "$input" | jq -r '.workspace.current_dir // empty')
input_tokens=$(echo "$input" | jq -r '.context_window.current_usage.input_tokens // "null"')
output_tokens=$(echo "$input" | jq -r '.context_window.current_usage.output_tokens // "null"')

if [ -n "$working_dir" ]; then
    dir_hash=$(echo -n "$working_dir" | md5 | cut -c1-16)
    cat > "$STATE_DIR/$dir_hash.json" << EOF
{"working_dir":"$working_dir","input_tokens":$input_tokens,"output_tokens":$output_tokens,"timestamp":$(date +%s)}
EOF
fi

# Optional: display git branch
cd "$working_dir" 2>/dev/null || true
branch=$(git rev-parse --abbrev-ref HEAD 2>/dev/null)
[ -n "$branch" ] && printf '\033[33mgit:\033[31m%s\033[0m' "$branch"
```

2. Make executable: `chmod +x ~/.vibe/claude-statusline.sh`

3. Add to `~/.claude/settings.json`:

```json
{
  "statusLine": {
    "type": "command",
    "command": "~/.vibe/claude-statusline.sh"
  }
}
```

Activity indicators:
- `[spinner]` (yellow) - Claude is thinking
- `[!]` (red) - Claude is waiting for input
- `[-]` (gray) - Session idle/stale

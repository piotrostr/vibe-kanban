# Vibe Kanban justfile

# Default recipe
default:
    @just --list

# Run the TUI (starts backend if not running)
vibe:
    #!/usr/bin/env bash
    set -euo pipefail

    # Check if backend is running by looking for port file or health check
    backend_running=false

    # Try to read port from port file
    port_file="${TMPDIR:-/tmp}/vibe/vibe.port"
    if [[ -f "$port_file" ]]; then
        port=$(cat "$port_file")
        if curl -s "http://127.0.0.1:${port}/api/health" > /dev/null 2>&1; then
            backend_running=true
        fi
    fi

    # If not running, start the backend in background
    if [[ "$backend_running" == "false" ]]; then
        echo "Starting backend..."
        cargo run -p server --release &
        backend_pid=$!

        # Wait for backend to be ready (max 30 seconds)
        for i in {1..60}; do
            if [[ -f "$port_file" ]]; then
                port=$(cat "$port_file")
                if curl -s "http://127.0.0.1:${port}/api/health" > /dev/null 2>&1; then
                    echo "Backend ready on port $port"
                    break
                fi
            fi
            sleep 0.5
        done
    fi

    # Run the TUI
    cargo run -p tui --release

# Run the TUI only (assumes backend is already running)
tui:
    cargo run -p tui --release

# Run the backend only
backend:
    cargo run -p server --release

# Run backend in dev mode with watch
backend-dev:
    cargo watch -x 'run -p server'

# Build the TUI
build-tui:
    cargo build -p tui --release

# Build everything
build:
    cargo build --release

# Run tests
test:
    cargo test --workspace

# Check all crates
check:
    cargo check --workspace

# Run the Tauri desktop app
desktop:
    pnpm run desktop:dev

# Run frontend dev server
frontend:
    pnpm run frontend:dev

# Full dev setup (backend + frontend)
dev:
    pnpm run dev

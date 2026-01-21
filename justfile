# Vibe Kanban justfile

# Default recipe
default:
    @just --list

# Run the TUI (with embedded server)
vibe:
    cargo run -p tui --bin vibe

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

# Install vibe to ~/.cargo/bin
install:
    cargo install --path crates/tui

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

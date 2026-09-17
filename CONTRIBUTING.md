# Contributing to HyprFile

## Setup

```bash
# Clone
git clone https://github.com/4keiler/HyprFile.git
cd HyprFile

# Build
cargo build --workspace

# Test
cargo test --workspace

# Run CLI (currently shows args only until UI is connected)
cargo run --bin hyprfiles -- /path/to/dir
```

## Architecture

- `crates/hyprfiles-core`: filesystem, jobs, preview, search, MIME, trash, config, theme, events
- `crates/hyprfiles-ui`: graphical interface layer
- `crates/hyprfiles-integrations`: Hyprland, XDG, portals, terminal, desktop entries
- `crates/hyprfiles-plugins`: future plugin runtime
- `crates/hyprfiles-cli`: entry point and CLI flags

## Guidelines

- Keep core decoupled from UI.
- All heavy operations must be async and non-blocking.
- Tests use `tempfile`; never touch real user files.
- Use `tracing` for logs, `anyhow` for app errors, `thiserror` for library errors.
- Follow `cargo fmt` and `cargo clippy`.

## Roadmap

See `docs/project-summary.md` and `HyprFile.md`.

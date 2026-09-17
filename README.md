# HyprFile

A modern, keyboard-first file manager for Linux, deeply integrated with Hyprland and Wayland.

> **Status**: Early development — architecture and scaffolding phase.

## Vision
Combine the speed and keyboard-driven philosophy of [Yazi](https://github.com/sxyazi/yazi) with a beautiful, configurable graphical interface that feels at home in any Hyprland rice.

## Principles
- **Keyboard-first**, mouse-friendly.
- **Fast**: async operations, lazy previews, virtualized lists.
- **Safe**: trash by default, clear confirmations, never block the UI.
- **Native Linux**: XDG, MIME, freedesktop trash, portals.
- **Modular**: core decoupled from UI, ready for future plugins.

## Architecture
Multi-crate Rust workspace:
- `hyprfiles-core` — filesystem, jobs, previews, search, config
- `hyprfiles-ui` — graphical interface (framework TBD)
- `hyprfiles-integrations` — Hyprland, XDG, portals
- `hyprfiles-plugins` — extension runtime (future)
- `hyprfiles-cli` — entry point and command-line flags

## Development
See `docs/project-summary.md` and `docs/arch/` for technical details.

## License
MIT OR Apache-2.0

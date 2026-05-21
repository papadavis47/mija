# Agents

## Project

Mija is a terminal pomodoro timer in Rust (edition 2024). It has two modes: a ratatui TUI (default) and a daemon mode that writes status to a file for tmux.

## Architecture

- `config.rs` — `Config` struct with durations and round count
- `timer.rs` — `Timer` state machine (`Idle → Work → ShortBreak/LongBreak → Work`), `Transition` struct, `State` enum
- `alerts.rs` — `AlertDispatcher` with pluggable `AlertSender` trait (tmux, bell, desktop notifications)
- `status_file.rs` — Writes formatted status to a file, cleans up on `Drop`
- `app.rs` — `App` struct wrapping `Timer` + `AlertDispatcher`, handles `Action` dispatch
- `ui.rs` — Ratatui rendering (status, countdown, progress gauge, round info, help)
- `cli.rs` — Clap `Args` struct, converts to `Config`
- `main.rs` — Entry point, wires CLI → Config → App → TUI or daemon loop

## Conventions

- **TDD**: Red/green workflow. Write failing tests first, then minimal implementation. All logic must be tested.
- **Test location**: Unit tests live in `#[cfg(test)] mod tests` at the bottom of each module.
- **Testability**: Use trait-based injection (e.g., `AlertSender`) so side effects can be mocked. See `MockSender` in `alerts.rs` tests.
- **Modules**: Flat module structure in `src/`. No nested directories.
- **No unnecessary dependencies**: Use `std` where sufficient. Only add crates for real needs.
- **Dev dependencies**: Test-only crates (e.g., `tempfile`) go in `[dev-dependencies]`.

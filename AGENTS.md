# Agents

## Project

Mija is a terminal Pomodoro timer in Rust (edition 2024). It runs as a Ratatui
TUI and automatically sends native notifications when running inside Herdr.

## Running the app

From the repository root:

```sh
cargo run
```

Use custom durations with:

```sh
cargo run -- --work 25 --short-break 5 --long-break 15 --rounds 4
```

Add `--bell` for terminal bells or `--notify` for desktop notifications.

For local Herdr plugin development:

```sh
cargo build --release
herdr plugin link "$(pwd)"
herdr plugin pane open --plugin mija.timer --entrypoint timer
```

Re-run `cargo build --release` after code changes. Unlink the checkout with
`herdr plugin unlink mija.timer`.

## Architecture

- `config.rs` — `Config` (durations in seconds) plus `FileConfig`, the optional
  `config.toml` layer (durations in minutes, every key optional). Parsing and
  path resolution are pure functions taking their inputs as arguments, so
  neither test needs to mutate the environment
- `timer.rs` — `Timer` state machine (`Idle → Work → ShortBreak/LongBreak → Work`), `Transition` struct, `State` enum
- `alerts.rs` — Structured alerts and `AlertDispatcher` with pluggable `AlertSender` implementations for Herdr, terminal bells, and desktop notifications
- `app.rs` — `App` struct wrapping `Timer` + `AlertDispatcher`, handles `Action` dispatch
- `theme.rs` — Monochrome rose palette (`#c84e89` primary), truecolor/256-colour
  detection, state→tone mapping
- `digits.rs` — 3×5 glyph bitmaps scaled to fill the pane, used for the clock
- `ui.rs` — Ratatui rendering (size tiers, framed layout, draining clock, round
  pips, session ribbon, help)
- `cli.rs` — Clap `Args` struct, converts to `Config`
- `main.rs` — Entry point, wires CLI → Config → alert senders → App → TUI

## Conventions

- **TDD**: Red/green workflow. Write failing tests first, then minimal implementation. All logic must be tested.
- **Test location**: Unit tests live in `#[cfg(test)] mod tests` at the bottom of each module.
- **Testability**: Use trait-based injection (e.g., `AlertSender`) so side effects can be mocked. See `MockSender` in `alerts.rs` tests.
- **Modules**: Flat module structure in `src/`. No nested directories.
- **No unnecessary dependencies**: Use `std` where sufficient. Only add crates for real needs.
- **Dev dependencies**: Test-only crates (e.g., `tempfile`) go in `[dev-dependencies]`.

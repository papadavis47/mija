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

Add `--bell` for terminal bells or `--notify` for desktop notifications;
`--no-bell` / `--no-notify` override a config file that turns them on.

For local Herdr plugin development:

```sh
cargo build --release
herdr plugin link "$(pwd)"
herdr plugin pane open --plugin mija.timer --entrypoint timer
```

Re-run `cargo build --release` after code changes. After editing
`herdr-plugin.toml` (including version bumps), also re-run
`herdr plugin link "$(pwd)"`, because Herdr caches the manifest at link time.
Unlink the checkout with `herdr plugin unlink mija.timer`.

## Architecture

- `config.rs` — `Config` (durations in seconds) plus `FileConfig`, the optional
  `config.toml` layer (durations in minutes, every key optional). Parsing and
  path resolution are pure functions taking their inputs as arguments, so
  neither test needs to mutate the environment. `prepare` writes a commented
  template on first run (default location only; never fatal) and returns a
  `Notice`; `main` prints it to stderr and hands it to `App` for the TUI.
  `check` enforces 1..=`MAX_MINUTES` (1440) / `MAX_ROUNDS` (24) and is shared
  by the serde deserializers and the clap validators
- `timer.rs` — `Timer` state machine (`Idle → Work → ShortBreak/LongBreak → Work`), `Transition` struct, `State` enum
- `alerts.rs` — Structured alerts and `AlertDispatcher` with pluggable `AlertSender` implementations for Herdr, terminal bells, and desktop notifications
- `app.rs` — `App` struct wrapping `Timer` + `AlertDispatcher`, handles `Action`
  dispatch and key presses (`press_key`); holds the startup `notice`. `Clock`
  paces ticks from a `Reading` (monotonic + wall time) and reports a sleep or
  stopped process as `Due::Suspended`, which pauses instead of replaying ticks
- `theme.rs` — Monochrome rose palette (`#c84e89` primary), truecolor/256-colour
  detection, state→tone mapping
- `digits.rs` — 3×5 glyph bitmaps scaled to fit the pane, capped at 5×4 so the
  clock stays modest on maximized screens
- `ui.rs` — Ratatui rendering (size tiers, framed layout, draining clock, round
  pips, session ribbon, help, notice popup anchored bottom-right above the
  footer)
- `cli.rs` — Clap `Args` struct, converts to `Config`; `command` injects the
  config path into `--help`. A help template puts name + version above the
  subtitle; the footer ends with the repository URL (from `Cargo.toml`
  `repository`). `--x` / `--no-x` alert pairs use `overrides_with` (last wins)
  and beat the config file
- `main.rs` — Entry point, wires CLI → Config → alert senders → App → TUI

Outside `src/`:

- `herdr-plugin.toml` — Herdr plugin manifest (`mija.timer`): release build
  step, `timer` pane, and the `open` action (`mija.timer.open`, which users
  bind to a key)
- `.github/workflows/ci.yml` — CI on Linux and macOS
- `assets/` — README logo
- `learning/` — gitignored personal notes; never commit or rely on them

## Conventions

- **TDD**: Red/green workflow. Write failing tests first, then minimal implementation. All logic must be tested.
- **Test location**: Unit tests live in `#[cfg(test)] mod tests` at the bottom of each module.
- **Testability**: Use trait-based injection (e.g., `AlertSender`) so side effects can be mocked. See `MockSender` in `alerts.rs` tests.
- **Modules**: Flat module structure in `src/`. No nested directories.
- **No unnecessary dependencies**: Use `std` where sufficient. Only add crates for real needs.
- **Dev dependencies**: Test-only crates (e.g., `tempfile`) go in `[dev-dependencies]`.
- **Before committing**: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
  and `cargo test` must pass. CI runs these plus `cargo build --release`.

## Releases

1. Bump the version in `Cargo.toml`, `Cargo.lock` (via `cargo build`), and
   `herdr-plugin.toml`, and update the README's `--ref vX.Y.Z` example.
2. Commit, then create an annotated tag with release notes:
   `git tag -a vX.Y.Z --cleanup=whitespace -F notes.md` (the cleanup flag keeps
   `##` headings).
3. Push with `git push origin main --follow-tags`, wait for CI to pass, then
   `gh release create vX.Y.Z --verify-tag --notes-file …`.

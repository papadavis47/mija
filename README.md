# Mija

A terminal pomodoro timer built in Rust. Named after the tomato.

## Usage

```sh
cargo run
```

### TUI Mode (default)

Runs a full-screen terminal UI with countdown, progress bar, and session tracking.
When Mija runs inside [Herdr](https://herdr.dev/), it automatically sends native
Herdr notifications as work and break periods finish.

| Key     | Action       |
|---------|--------------|
| `space` | Pause/Resume |
| `s`     | Skip         |
| `q`     | Quit         |

### Herdr

Run `mija` in a dedicated Herdr tab so the timer can continue while you work in
another tab. Completed work periods use Herdr's `done` notification sound;
completed breaks use its `request` sound.

For local plugin development, build Mija and link this checkout:

```sh
cargo build --release
herdr plugin link "$(pwd)"
```

Then open the timer as a Herdr-managed tab:

```sh
herdr plugin pane open --plugin mija.timer --entrypoint timer
```

The plugin manifest builds the release binary when installed from GitHub.
Unlink a development checkout with `herdr plugin unlink mija.timer`.

### Options

```
--work <MIN>          Work duration in minutes (default: 25)
--short-break <MIN>   Short break duration (default: 5)
--long-break <MIN>    Long break duration (default: 15)
--rounds <N>          Rounds before long break (default: 4)
--bell                Terminal bell on transitions
--notify              Desktop notifications on transitions
```

## Tests

```sh
cargo test
```

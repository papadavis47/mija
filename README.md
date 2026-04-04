# Linda

A terminal pomodoro timer built in Rust. Named after the tomato.

## Usage

```sh
cargo run
```

### TUI Mode (default)

Runs a full-screen terminal UI with countdown, progress bar, and session tracking.

| Key     | Action       |
|---------|--------------|
| `space` | Pause/Resume |
| `s`     | Skip         |
| `q`     | Quit         |

### Daemon Mode

Writes timer state to `/tmp/pomodoro_status` for use in a tmux status bar:

```sh
cargo run -- --daemon
```

Add to `~/.tmux.conf`:

```
set -g status-right '#(cat /tmp/pomodoro_status)'
set -g status-interval 1
```

### Options

```
--work <MIN>          Work duration in minutes (default: 25)
--short-break <MIN>   Short break duration (default: 5)
--long-break <MIN>    Long break duration (default: 15)
--rounds <N>          Rounds before long break (default: 4)
--bell                Terminal bell on transitions
--notify              Desktop notifications on transitions
--daemon              Run in daemon mode
```

## Tests

```sh
cargo test
```

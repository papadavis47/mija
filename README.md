<p>
  <img src="assets/logo-mark-rose.png" width="128" alt="Mija logo: a pomodoro dial with an M monogram">
</p>

# Mija

[![CI](https://github.com/papadavis47/mija/actions/workflows/ci.yml/badge.svg)](https://github.com/papadavis47/mija/actions/workflows/ci.yml)

Mija is a terminal Pomodoro timer built in Rust.

**Mija** is a colloquial Spanish word for "my daughter," pronounced **MEE-hah** (IPA: `/ˈmi.xa/`; the `j` sounds like an English `h`). Shortened from "_**mi hija**_".

The name comes from the word I often use when speaking to my daughter.

I frequently see her while I am working at my computer, and her visits are a
welcome reminder to take a break and spend time interacting with her.

## Build and install

Mija needs a Rust toolchain with edition 2024 support (Rust 1.85 or newer);
install one with [rustup](https://rustup.rs/).

Build an optimised binary from the project directory:

```sh
cargo build --release
```

It lands at `target/release/mija` and can be run from there directly.

To put `mija` on your `PATH`, install it with Cargo:

```sh
cargo install --path .
```

This builds in release mode and copies the binary to `~/.cargo/bin` (make sure
that directory is on your `PATH`). Re-run the same command after pulling
changes to update it, and remove it with:

```sh
cargo uninstall mija
```

Once installed, every `cargo run --` example below works as plain `mija`, e.g.
`mija --work 50`.

## Run Mija

From the project directory, start the app with:

```sh
cargo run
```

This opens the full-screen terminal UI and starts a 25-minute work period. The
clock fills the window and drains from the top as the period runs down, so it
doubles as the progress indicator. Below it, round pips track the current cycle
and a ribbon records the pomodoros finished this session.

The UI adapts to the space it is given, down to a single status line in a small
pane. It uses 24-bit colour where the terminal advertises it (`COLORTERM`) and
falls back to a 256-colour palette otherwise.
When Mija runs inside [Herdr](https://herdr.dev/), it automatically sends native
Herdr notifications as work and break periods finish.

### Controls

| Key     | Action       |
| ------- | ------------ |
| `space` | Pause/Resume |
| `s`     | Skip         |
| `q`     | Quit         |

### Custom durations and alerts

Durations are specified in minutes:

```sh
cargo run -- --work 25 --short-break 5 --long-break 15 --rounds 4
```

Add `--bell` for terminal bells or `--notify` for desktop notifications:

```sh
cargo run -- --bell
cargo run -- --notify
```

### Configuration

Rather than passing flags every time, put your preferences in
`~/.config/mija/config.toml` (or `$XDG_CONFIG_HOME/mija/config.toml`; set
`MIJA_CONFIG` to point somewhere else entirely):

```toml
work = 30          # minutes
short_break = 7
long_break = 20
rounds = 3
bell = true
notify = false
```

On first run Mija creates this file with every key commented out and tells you
where it is — `created config at …` stays over the timer until you press a key.
Uncomment a line to change it. `mija --help` always shows where the file lives.

Every key is optional — anything you leave out keeps its default. Durations are
in minutes, matching the flags. Durations must be 1–1440 minutes (up to a day) and
`rounds` 1–24, in the file and on the command line.

Settings are layered, most specific first:

1. a command-line flag
2. the config file
3. the built-in default

So with the file above, `mija` starts a 30-minute work period while
`mija --work 5` starts a 5-minute one. Alerts work the same way: `--bell` and
`--notify` switch them on for a run, `--no-bell` and `--no-notify` switch them
off even when the file turns them on. If both forms are given, the last wins.

If `MIJA_CONFIG` points to a file that does not exist, Mija says so the same
way and runs with the defaults; it only ever creates the file at the default location.

An unreadable file, a malformed one, or an unrecognised key stops Mija with an
error on stderr instead of starting with settings you did not ask for.

### Herdr

Run `mija` in a dedicated Herdr tab so the timer can continue while you work in
another tab. Completed work periods use Herdr's `done` notification sound;
completed breaks use its `request` sound.

#### Install as a Herdr plugin

Install Mija straight from GitHub. Herdr clones the repository and builds the
release binary on your machine, so a Rust toolchain (1.85 or newer) must be
installed first:

```sh
herdr plugin install papadavis47/mija              # latest main
herdr plugin install papadavis47/mija --ref v0.3.0 # a tagged release
```

Then open the timer as a Herdr-managed tab:

```sh
herdr plugin pane open --plugin mija.timer --entrypoint timer
```

To open it with a key instead, bind the plugin's `open` action in
`~/.config/herdr/config.toml`:

```toml
[[keys.command]]
key = "prefix+m"
type = "plugin_action"
command = "mija.timer.open"
description = "open mija timer"
```

Reload the config (`herdr server reload-config`), then press your prefix
followed by `m`.

Remove it with:

```sh
herdr plugin uninstall mija.timer
```

#### Local plugin development

For local plugin development, build Mija and link this checkout:

```sh
cargo build --release
herdr plugin link "$(pwd)"
```

Then open the timer as a Herdr-managed tab:

```sh
herdr plugin pane open --plugin mija.timer --entrypoint timer
```

After changing Mija's code, rebuild the linked plugin with:

```sh
cargo build --release
```

Unlink a development checkout with:

```sh
herdr plugin unlink mija.timer
```

### Options

```
--work <MIN>          Work duration in minutes, 1–1440 (default: 25)
--short-break <MIN>   Short break duration, 1–1440 (default: 5)
--long-break <MIN>    Long break duration, 1–1440 (default: 15)
--rounds <N>          Rounds before long break, 1–24 (default: 4)
--bell                Terminal bell on transitions
--no-bell             No terminal bell, even if the config turns it on
--notify              Desktop notifications on transitions
--no-notify           No desktop notifications, even if the config turns them on
-h, --help            Print help
-V, --version         Print version
```

Flags override the config file; see [Configuration](#configuration).

## Tests

```sh
cargo test
```

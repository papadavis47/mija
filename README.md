# Mija

Mija is a terminal Pomodoro timer built in Rust.

**Mija** is a colloquial Spanish word for "my daughter," pronounced **MEE-hah** (IPA: `/ˈmi.xa/`; the `j` sounds like an English `h`). Shortened from "_**mi hija**_".

The name comes from the word I often use when speaking to my daughter.

I frequently see her while I am working at my computer, and her visits are a
welcome reminder to take a break and spend time interacting with her.

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

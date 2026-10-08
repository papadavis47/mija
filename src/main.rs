mod alerts;
mod app;
mod cli;
mod config;
mod digits;
mod theme;
mod timer;
mod ui;

use alerts::{AlertDispatcher, AlertSender, BellSender, DesktopSender, HerdrSender};
use app::{Action, App};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use std::io;
use std::time::{Duration, Instant};
use theme::Theme;

fn main() -> io::Result<()> {
    let location = config::config_path();
    // Parse first so `--help` and `--version` never create a config file.
    let args = cli::parse(location.as_ref().map(|location| location.path.as_path()));

    // Read the config before the TUI claims the screen — once ratatui is up,
    // a message on stderr would be painted over.
    // The notice also goes in the TUI, where it must be acknowledged; stderr
    // keeps a copy in the scrollback once the alternate screen is gone.
    let notice = config::prepare(location.as_ref()).map(|notice| notice.to_string());
    if let Some(notice) = &notice {
        eprintln!("mija: {notice}");
    }
    let file = match config::load(location.as_ref()) {
        Ok(file) => file,
        Err(err) => {
            eprintln!("mija: {err}");
            std::process::exit(1);
        }
    };
    let config = args.to_config(&file);

    let mut senders: Vec<Box<dyn AlertSender>> = Vec::new();
    if let Some(sender) = HerdrSender::from_env() {
        senders.push(Box::new(sender));
    }
    if args.bell_enabled(&file) {
        senders.push(Box::new(BellSender::stdout()));
    }
    if args.notify_enabled(&file) {
        senders.push(Box::new(DesktopSender));
    }
    let alerts = AlertDispatcher::new(senders);

    run_tui(config, alerts, notice)
}

fn run_tui(
    config: config::Config,
    alerts: AlertDispatcher,
    notice: Option<String>,
) -> io::Result<()> {
    /// Redraw cadence. Fast enough for the colon to breathe and the
    /// transition sweep to read as motion.
    const FRAME: Duration = Duration::from_millis(100);
    const TICK: Duration = Duration::from_secs(1);

    let mut terminal = ratatui::init();
    let mut app = App::new(config, alerts);
    app.notice = notice;
    let theme = Theme::from_env();
    let started = Instant::now();
    let mut next_tick = Instant::now() + TICK;

    let result = loop {
        terminal.draw(|frame| ui::draw(frame, &app, &theme, started.elapsed()))?;

        if event::poll(FRAME)?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            match key.code {
                KeyCode::Char(c) => app.press_key(c),
                // Non-character keys still dismiss a notice.
                _ => app.notice = None,
            }
        }

        // Driven by the clock rather than the poll timeout, so holding a key
        // can no longer stall the countdown.
        while Instant::now() >= next_tick {
            next_tick += TICK;
            app.handle_action(Action::Tick);
        }

        if app.should_quit {
            break Ok(());
        }
    };

    ratatui::restore();
    result
}

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
use clap::Parser;
use cli::Args;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use std::io;
use std::time::{Duration, Instant};
use theme::Theme;

fn main() -> io::Result<()> {
    let args = Args::parse();

    // Read the config before the TUI claims the screen — once ratatui is up,
    // a message on stderr would be painted over.
    let file = match config::load() {
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

    run_tui(config, alerts)
}

fn run_tui(config: config::Config, alerts: AlertDispatcher) -> io::Result<()> {
    /// Redraw cadence. Fast enough for the colon to breathe and the
    /// transition sweep to read as motion.
    const FRAME: Duration = Duration::from_millis(100);
    const TICK: Duration = Duration::from_secs(1);

    let mut terminal = ratatui::init();
    let mut app = App::new(config, alerts);
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
                KeyCode::Char('q') => app.handle_action(Action::Quit),
                KeyCode::Char(' ') => app.toggle_pause(),
                KeyCode::Char('s') => app.handle_action(Action::Skip),
                _ => {}
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

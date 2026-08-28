mod alerts;
mod app;
mod cli;
mod config;
mod timer;
mod ui;

use alerts::{AlertDispatcher, AlertSender, BellSender, DesktopSender, HerdrSender};
use app::{Action, App};
use clap::Parser;
use cli::Args;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use std::io;
use std::time::Duration;

fn main() -> io::Result<()> {
    let args = Args::parse();
    let config = args.to_config();

    let mut senders: Vec<Box<dyn AlertSender>> = Vec::new();
    if let Some(sender) = HerdrSender::from_env() {
        senders.push(Box::new(sender));
    }
    if args.bell {
        senders.push(Box::new(BellSender::stdout()));
    }
    if args.notify {
        senders.push(Box::new(DesktopSender));
    }
    let alerts = AlertDispatcher::new(senders);

    run_tui(config, alerts)
}

fn run_tui(config: config::Config, alerts: AlertDispatcher) -> io::Result<()> {
    let mut terminal = ratatui::init();
    let mut app = App::new(config, alerts);

    let result = loop {
        terminal.draw(|frame| ui::draw(frame, &app))?;

        if event::poll(Duration::from_secs(1))? {
            if let Event::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press
            {
                match key.code {
                    KeyCode::Char('q') => app.handle_action(Action::Quit),
                    KeyCode::Char(' ') => app.toggle_pause(),
                    KeyCode::Char('s') => app.handle_action(Action::Skip),
                    _ => {}
                }
            }
        } else {
            app.handle_action(Action::Tick);
        }

        if app.should_quit {
            break Ok(());
        }
    };

    ratatui::restore();
    result
}

mod alerts;
mod app;
mod cli;
mod config;
mod status_file;
mod timer;
mod ui;

use alerts::{AlertDispatcher, AlertSender, BellSender, DesktopSender, TmuxSender};
use app::{Action, App};
use clap::Parser;
use cli::Args;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use status_file::StatusFile;
use std::io;
use std::path::PathBuf;
use std::time::Duration;
use timer::Timer;

fn main() -> io::Result<()> {
    let args = Args::parse();
    let config = args.to_config();

    let mut senders: Vec<Box<dyn AlertSender>> = vec![Box::new(TmuxSender)];
    if args.bell {
        senders.push(Box::new(BellSender::stdout()));
    }
    if args.notify {
        senders.push(Box::new(DesktopSender));
    }
    let alerts = AlertDispatcher::with_senders(senders);

    if args.daemon {
        run_daemon(config, &alerts);
        Ok(())
    } else {
        run_tui(config, alerts)
    }
}

fn run_daemon(config: config::Config, alerts: &AlertDispatcher) {
    let mut timer = Timer::new(config);
    let status_file = StatusFile::new(PathBuf::from("/tmp/pomodoro_status"));

    timer.start();

    loop {
        let _ = status_file.write(&timer.format_status());

        std::thread::sleep(Duration::from_secs(1));

        if let Some(transition) = timer.tick() {
            alerts.on_transition(transition);
        }

        if timer.state() == timer::State::Idle {
            break;
        }
    }
}

fn run_tui(config: config::Config, alerts: AlertDispatcher) -> io::Result<()> {
    let mut terminal = ratatui::init();
    let mut app = App::new(config, alerts);

    let result = loop {
        terminal.draw(|frame| ui::draw(frame, &app))?;

        if event::poll(Duration::from_secs(1))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => app.handle_action(Action::Quit),
                        KeyCode::Char(' ') => app.toggle_pause(),
                        KeyCode::Char('s') => app.handle_action(Action::Skip),
                        _ => {}
                    }
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

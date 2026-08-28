use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, Padding, Paragraph};

use crate::app::App;
use crate::timer::State;

pub fn draw(frame: &mut Frame, app: &App) {
    let timer = &app.timer;
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3), // status
            Constraint::Length(5), // timer display
            Constraint::Length(3), // progress bar
            Constraint::Length(3), // round info
            Constraint::Min(0),    // help
        ])
        .split(area);

    draw_status(frame, timer.state(), chunks[0]);
    draw_countdown(frame, &timer.format_remaining(), chunks[1]);
    draw_progress(frame, timer.progress(), timer.state(), chunks[2]);
    draw_round_info(
        frame,
        timer.current_round(),
        timer.total_rounds(),
        timer.completed_pomodoros(),
        chunks[3],
    );
    draw_help(frame, timer.state(), chunks[4]);
}

fn state_color(state: State) -> Color {
    match state {
        State::Work => Color::Red,
        State::ShortBreak => Color::Green,
        State::LongBreak => Color::Cyan,
        State::Paused => Color::Yellow,
        State::Idle => Color::DarkGray,
    }
}

fn draw_status(frame: &mut Frame, state: State, area: Rect) {
    let label = match state {
        State::Idle => "⏹  IDLE",
        State::Work => "🍅 WORK",
        State::ShortBreak => "☕ SHORT BREAK",
        State::LongBreak => "🌴 LONG BREAK",
        State::Paused => "⏸  PAUSED",
    };
    let paragraph = Paragraph::new(Line::from(Span::styled(
        label,
        Style::default()
            .fg(state_color(state))
            .add_modifier(Modifier::BOLD),
    )))
    .block(Block::default().borders(Borders::NONE));
    frame.render_widget(paragraph, area);
}

fn draw_countdown(frame: &mut Frame, time_str: &str, area: Rect) {
    let paragraph = Paragraph::new(Line::from(Span::styled(
        time_str,
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Timer ")
            .padding(Padding::horizontal(1)),
    );
    frame.render_widget(paragraph, area);
}

fn draw_progress(frame: &mut Frame, progress: f64, state: State, area: Rect) {
    let ratio = progress.clamp(0.0, 1.0);
    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::NONE))
        .gauge_style(
            Style::default()
                .fg(state_color(state))
                .add_modifier(Modifier::BOLD),
        )
        .ratio(ratio);
    frame.render_widget(gauge, area);
}

fn draw_round_info(frame: &mut Frame, current: u32, total: u32, completed: u32, area: Rect) {
    let text = format!("Round {current}/{total}  •  {completed} pomodoros completed");
    let paragraph = Paragraph::new(Line::from(Span::styled(
        text,
        Style::default().fg(Color::DarkGray),
    )))
    .block(Block::default().borders(Borders::NONE));
    frame.render_widget(paragraph, area);
}

fn draw_help(frame: &mut Frame, state: State, area: Rect) {
    let pause_hint = match state {
        State::Paused => "space: resume",
        _ => "space: pause",
    };
    let text = format!("{pause_hint}  |  s: skip  |  q: quit");
    let paragraph = Paragraph::new(Line::from(Span::styled(
        text,
        Style::default().fg(Color::DarkGray),
    )))
    .block(Block::default().borders(Borders::NONE));
    frame.render_widget(paragraph, area);
}

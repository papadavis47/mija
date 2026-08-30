use std::time::Duration;

use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Padding, Paragraph};

use crate::app::App;
use crate::digits::{self, Mask};
use crate::theme::{Theme, Tone, accent};
use crate::timer::State;

/// How long the colour sweep across the screen lasts after a state change.
const SWEEP: Duration = Duration::from_millis(400);
/// Seconds for one full breath of the colon and the active pip.
const BREATH_SECS: f64 = 2.4;
/// How far the colon fades towards plum at the bottom of a breath.
const BREATH_DEPTH: f64 = 0.3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    Full,
    Medium,
    Compact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pip {
    Done,
    Active,
    Todo,
}

// ---- helpers ----
pub fn tier(area: Rect) -> Tier {
    if area.height >= 26 && area.width >= 56 {
        Tier::Full
    } else if area.height >= 14 && area.width >= 44 {
        Tier::Medium
    } else {
        Tier::Compact
    }
}

pub fn state_label(state: State) -> &'static str {
    match state {
        State::Work => "work",
        State::ShortBreak => "break",
        State::LongBreak => "long break",
        State::Paused => "paused",
        State::Idle => "ready",
    }
}

/// Letterspaced lowercase, so the state reads as a quiet caption under the
/// clock rather than a shouted heading.
pub fn spaced(label: &str) -> String {
    label
        .chars()
        .map(|c| c.to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn pips(state: State, current: u32, total: u32) -> Vec<Pip> {
    let working = state == State::Work;
    let done = if working {
        current.saturating_sub(1) as usize
    } else {
        current as usize
    };
    let active = if working {
        done
    } else {
        // No round is in flight during a break.
        usize::MAX
    };
    (0..total as usize)
        .map(|i| {
            if i == active {
                Pip::Active
            } else if i < done {
                Pip::Done
            } else {
                Pip::Todo
            }
        })
        .collect()
}

/// Filled and total slots for today's ribbon. The ribbon grows past its
/// resting width once you outrun it.
pub fn ribbon(completed: u32, min_slots: usize) -> (usize, usize) {
    let filled = completed as usize;
    (filled, min_slots.max(filled))
}

/// How much of a clock row has drained, 0.0 (full) to 1.0 (empty). The
/// boundary row lands on a fraction so the drain line can be blended.
pub fn drain_at(row: usize, rows: usize, remaining: f64) -> f64 {
    if rows == 0 {
        return 0.0;
    }
    let boundary = rows as f64 * (1.0 - remaining.clamp(0.0, 1.0));
    (boundary - row as f64).clamp(0.0, 1.0)
}

/// Column the transition sweep has reached, or None once it is done.
pub fn sweep_x(age: Duration, width: u16) -> Option<u16> {
    if age >= SWEEP {
        return None;
    }
    let progress = age.as_secs_f64() / SWEEP.as_secs_f64();
    Some((progress * width as f64) as u16)
}
// ---- end helpers ----

/// A 0.0..1.0 triangle-free breath, used for the colon and the active pip.
fn wave(elapsed: Duration, period_secs: f64) -> f64 {
    let phase = (elapsed.as_secs_f64() % period_secs) / period_secs;
    (1.0 - (phase * std::f64::consts::TAU).cos()) / 2.0
}

pub fn draw(frame: &mut Frame, app: &App, theme: &Theme, elapsed: Duration) {
    let area = frame.area();
    frame.render_widget(
        Block::default().style(Style::default().bg(theme.color(Tone::Ink))),
        area,
    );

    match tier(area) {
        Tier::Compact => draw_compact(frame, app, theme, elapsed, area),
        Tier::Medium => draw_framed(frame, app, theme, elapsed, area, false),
        Tier::Full => draw_framed(frame, app, theme, elapsed, area, true),
    }

    if let Some(age) = app.last_transition.map(|at| at.elapsed())
        && let Some(x) = sweep_x(age, area.width)
    {
        paint_sweep(frame, area, x, theme);
    }
}

fn remaining_ratio(app: &App) -> f64 {
    match app.timer.state() {
        State::Idle => 1.0,
        _ => app.timer.progress().clamp(0.0, 1.0),
    }
}

fn draw_framed(
    frame: &mut Frame,
    app: &App,
    theme: &Theme,
    elapsed: Duration,
    area: Rect,
    full: bool,
) {
    let timer = &app.timer;
    let state = timer.state();
    let tone = accent(state);
    let paused = state == State::Paused;

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.color(tone)))
        .style(Style::default().bg(theme.color(Tone::Ink)))
        .padding(Padding::symmetric(2, 1))
        .title(Span::styled(
            " mija ",
            Style::default()
                .fg(theme.color(Tone::Rose))
                .add_modifier(Modifier::BOLD),
        ))
        .title(
            Line::from(Span::styled(
                format!(
                    " round {} of {} ",
                    timer.current_round().max(1),
                    timer.total_rounds()
                ),
                Style::default().fg(theme.color(Tone::Mist)),
            ))
            .right_aligned(),
        );
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height == 0 || inner.width == 0 {
        return;
    }

    let footer_h: u16 = if full { 2 } else { 1 };
    let content_h = inner.height.saturating_sub(footer_h + 1);
    let content = Rect::new(inner.x, inner.y, inner.width, content_h);

    draw_stack(frame, app, theme, elapsed, content);

    let footer_y = inner.y + content_h + 1;
    if full {
        render_line(
            frame,
            Rect::new(inner.x, footer_y, inner.width, 1),
            ribbon_line(
                theme,
                tone,
                timer.completed_pomodoros(),
                inner.width as usize,
            ),
            Alignment::Left,
        );
    }
    render_line(
        frame,
        Rect::new(inner.x, footer_y + footer_h - 1, inner.width, 1),
        help_line(theme, paused),
        Alignment::Left,
    );
}

fn draw_stack(frame: &mut Frame, app: &App, theme: &Theme, elapsed: Duration, content: Rect) {
    let timer = &app.timer;
    let state = timer.state();
    let tone = accent(state);
    let paused = state == State::Paused;
    let clock_text = timer.format_remaining();

    let scale = digits::fit_scale(
        &clock_text,
        content.width as usize,
        content.height.saturating_sub(4) as usize,
    );
    let clock_h = scale.map_or(1u16, |(_, sy)| (digits::GLYPH_H * sy) as u16);
    let stack_h = clock_h + 4;
    if content.height < stack_h {
        return;
    }
    let top = content.y + (content.height - stack_h) / 2;

    render_line(
        frame,
        Rect::new(content.x, top, content.width, 1),
        Line::from(Span::styled(
            spaced(state_label(state)),
            Style::default().fg(theme.color(Tone::Mist)),
        )),
        Alignment::Center,
    );

    let breath = if paused {
        BREATH_DEPTH
    } else {
        BREATH_DEPTH * wave(elapsed, BREATH_SECS)
    };
    let clock_area = Rect::new(content.x, top + 2, content.width, clock_h);
    match scale {
        Some((sx, sy)) => {
            let mask = digits::render(&clock_text, sx, sy);
            let lines = clock_lines(&mask, theme, tone, remaining_ratio(app), breath);
            frame.render_widget(
                Paragraph::new(lines).alignment(Alignment::Center),
                clock_area,
            );
        }
        None => render_line(
            frame,
            clock_area,
            Line::from(Span::styled(
                clock_text,
                Style::default()
                    .fg(theme.color(tone))
                    .add_modifier(Modifier::BOLD),
            )),
            Alignment::Center,
        ),
    }

    render_line(
        frame,
        Rect::new(content.x, top + 2 + clock_h + 1, content.width, 1),
        pip_line(
            theme,
            tone,
            state,
            timer.current_round(),
            timer.total_rounds(),
            elapsed,
            paused,
        ),
        Alignment::Center,
    );
}

fn draw_compact(frame: &mut Frame, app: &App, theme: &Theme, elapsed: Duration, area: Rect) {
    let timer = &app.timer;
    let state = timer.state();
    let tone = accent(state);
    let paused = state == State::Paused;

    let mut spans = vec![
        Span::styled(
            timer.format_remaining(),
            Style::default()
                .fg(theme.color(tone))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  ", Style::default()),
        Span::styled(
            state_label(state),
            Style::default().fg(theme.color(Tone::Mist)),
        ),
        Span::styled("  ", Style::default()),
    ];
    spans.extend(
        pip_line(
            theme,
            tone,
            state,
            timer.current_round(),
            timer.total_rounds(),
            elapsed,
            paused,
        )
        .spans,
    );

    let y = area.y + area.height / 2;
    render_line(
        frame,
        Rect::new(area.x, y, area.width, 1),
        Line::from(spans),
        Alignment::Center,
    );
}

fn render_line(frame: &mut Frame, area: Rect, line: Line<'static>, alignment: Alignment) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    frame.render_widget(Paragraph::new(line).alignment(alignment), area);
}

/// The signature: numerals that start rose and drain to plum from the top as
/// the period runs out, so the clock is its own progress indicator.
fn clock_lines(
    mask: &Mask,
    theme: &Theme,
    tone: Tone,
    remaining: f64,
    breath: f64,
) -> Vec<Line<'static>> {
    let mut lines = Vec::with_capacity(mask.height);
    for y in 0..mask.height {
        let drained = drain_at(y, mask.height, remaining);
        let digit_color = theme.drained(tone, drained);
        let colon_color = theme.blend(tone, Tone::Plum, (drained + breath).min(0.9));

        let mut spans: Vec<Span<'static>> = Vec::new();
        let mut run = String::new();
        let mut run_color: Color = digit_color;
        for x in 0..mask.width {
            let color = if mask.is_colon(x) {
                colon_color
            } else {
                digit_color
            };
            if !run.is_empty() && color != run_color {
                spans.push(Span::styled(
                    std::mem::take(&mut run),
                    Style::default().fg(run_color),
                ));
            }
            run_color = color;
            run.push(if mask.get(x, y) { '█' } else { ' ' });
        }
        if !run.is_empty() {
            spans.push(Span::styled(run, Style::default().fg(run_color)));
        }
        lines.push(Line::from(spans));
    }
    lines
}

fn pip_line(
    theme: &Theme,
    tone: Tone,
    state: State,
    current: u32,
    total: u32,
    elapsed: Duration,
    paused: bool,
) -> Line<'static> {
    let pulse = if paused {
        0.0
    } else {
        wave(elapsed, BREATH_SECS)
    };
    let spans = pips(state, current, total)
        .into_iter()
        .map(|pip| match pip {
            Pip::Done => Span::styled("● ", Style::default().fg(theme.color(tone))),
            Pip::Active => Span::styled(
                "● ",
                Style::default().fg(theme.blend(Tone::Blush, tone, pulse)),
            ),
            Pip::Todo => Span::styled("○ ", Style::default().fg(theme.color(Tone::Plum))),
        })
        .collect::<Vec<_>>();
    Line::from(spans)
}

fn ribbon_line(theme: &Theme, tone: Tone, completed: u32, width: usize) -> Line<'static> {
    let label = "today  ";
    let budget = width.saturating_sub(label.len());
    let (filled, slots) = ribbon(completed, 8);
    let slots = slots.min(budget);
    let filled = filled.min(slots);
    Line::from(vec![
        Span::styled(label, Style::default().fg(theme.color(Tone::Mist))),
        Span::styled("▮".repeat(filled), Style::default().fg(theme.color(tone))),
        Span::styled(
            "▯".repeat(slots - filled),
            Style::default().fg(theme.color(Tone::Plum)),
        ),
    ])
}

fn help_line(theme: &Theme, paused: bool) -> Line<'static> {
    let key = Style::default()
        .fg(theme.color(Tone::Blush))
        .add_modifier(Modifier::BOLD);
    let word = Style::default().fg(theme.color(Tone::Mist));
    let sep = Style::default().fg(theme.color(Tone::Plum));
    Line::from(vec![
        Span::styled("space", key),
        Span::styled(if paused { " resume" } else { " pause" }, word),
        Span::styled("  ·  ", sep),
        Span::styled("s", key),
        Span::styled(" skip", word),
        Span::styled("  ·  ", sep),
        Span::styled("q", key),
        Span::styled(" quit", word),
    ])
}

fn paint_sweep(frame: &mut Frame, area: Rect, x: u16, theme: &Theme) {
    let band = 2u16;
    let blush = theme.color(Tone::Blush);
    let buffer = frame.buffer_mut();
    let lo = x.saturating_sub(band);
    let hi = (x + band).min(area.width.saturating_sub(1));
    for offset in lo..=hi {
        for row in area.y..area.y + area.height {
            buffer[(area.x + offset, row)].set_fg(blush);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(w: u16, h: u16) -> Rect {
        Rect::new(0, 0, w, h)
    }

    use crate::alerts::AlertDispatcher;
    use crate::config::Config;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn render_at(width: u16, height: u16, remaining_secs: u32) -> ratatui::buffer::Buffer {
        let mut app = App::new(Config::default(), AlertDispatcher::new(Vec::new()));
        while app.timer.remaining_secs() > remaining_secs {
            app.handle_action(crate::app::Action::Tick);
        }
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|frame| {
                draw(
                    frame,
                    &app,
                    &Theme::detect(Some("truecolor")),
                    Duration::from_millis(600),
                )
            })
            .unwrap();
        terminal.backend().buffer().clone()
    }

    #[test]
    fn every_tier_renders_without_panicking() {
        for (w, h) in [
            (200u16, 52u16),
            (80, 30),
            (66, 40),
            (56, 26),
            (44, 14),
            (30, 10),
            (10, 3),
            (1, 1),
        ] {
            render_at(w, h, 1122);
        }
    }

    #[test]
    fn the_clock_drains_as_the_period_runs_out() {
        // Same cell, early and late in the period: rose at the start, plum-ward
        // once the drain line has passed it.
        let full = render_at(80, 30, 1500);
        let empty = render_at(80, 30, 10);
        let mut differed = false;
        for x in 0..80 {
            for y in 0..30 {
                if full[(x, y)].symbol() == "\u{2588}" && full[(x, y)].fg != empty[(x, y)].fg {
                    differed = true;
                }
            }
        }
        assert!(differed, "drain should recolour the clock over time");
    }

    #[test]
    fn full_tier_needs_both_height_and_width() {
        assert_eq!(tier(rect(56, 26)), Tier::Full);
        assert_eq!(tier(rect(120, 40)), Tier::Full);
        assert_eq!(tier(rect(55, 26)), Tier::Medium);
        assert_eq!(tier(rect(56, 25)), Tier::Medium);
    }

    #[test]
    fn medium_tier_covers_a_split_herdr_pane() {
        assert_eq!(tier(rect(44, 14)), Tier::Medium);
        assert_eq!(tier(rect(200, 17)), Tier::Medium);
    }

    #[test]
    fn compact_tier_is_the_last_resort() {
        assert_eq!(tier(rect(43, 14)), Tier::Compact);
        assert_eq!(tier(rect(44, 13)), Tier::Compact);
        assert_eq!(tier(rect(20, 5)), Tier::Compact);
    }

    #[test]
    fn state_labels_are_plain_and_lowercase() {
        assert_eq!(state_label(State::Work), "work");
        assert_eq!(state_label(State::ShortBreak), "break");
        assert_eq!(state_label(State::LongBreak), "long break");
        assert_eq!(state_label(State::Paused), "paused");
        assert_eq!(state_label(State::Idle), "ready");
    }

    #[test]
    fn labels_are_letterspaced() {
        assert_eq!(spaced("work"), "w o r k");
        assert_eq!(spaced("long break"), "l o n g   b r e a k");
    }

    #[test]
    fn working_rounds_show_one_active_pip() {
        assert_eq!(
            pips(State::Work, 2, 4),
            vec![Pip::Done, Pip::Active, Pip::Todo, Pip::Todo]
        );
        assert_eq!(
            pips(State::Work, 1, 4),
            vec![Pip::Active, Pip::Todo, Pip::Todo, Pip::Todo]
        );
    }

    #[test]
    fn breaks_show_completed_rounds_with_nothing_active() {
        assert_eq!(
            pips(State::ShortBreak, 1, 4),
            vec![Pip::Done, Pip::Todo, Pip::Todo, Pip::Todo]
        );
        assert_eq!(pips(State::LongBreak, 0, 4), vec![Pip::Todo; 4]);
    }

    #[test]
    fn pips_are_bounded_by_the_round_count() {
        assert!(pips(State::Work, 1, 0).is_empty());
        assert_eq!(pips(State::Work, 9, 4).len(), 4);
    }

    #[test]
    fn ribbon_rests_at_its_minimum_width() {
        assert_eq!(ribbon(0, 8), (0, 8));
        assert_eq!(ribbon(3, 8), (3, 8));
    }

    #[test]
    fn ribbon_grows_once_you_outrun_it() {
        assert_eq!(ribbon(10, 8), (10, 10));
    }

    #[test]
    fn a_full_period_has_drained_nothing() {
        assert_eq!(drain_at(0, 10, 1.0), 0.0);
        assert_eq!(drain_at(9, 10, 1.0), 0.0);
    }

    #[test]
    fn an_expired_period_has_drained_everything() {
        assert_eq!(drain_at(0, 10, 0.0), 1.0);
        assert_eq!(drain_at(9, 10, 0.0), 1.0);
    }

    #[test]
    fn the_drain_line_sits_where_the_remaining_time_puts_it() {
        assert_eq!(drain_at(4, 10, 0.5), 1.0);
        assert_eq!(drain_at(5, 10, 0.5), 0.0);
    }

    #[test]
    fn the_boundary_row_carries_a_fraction_so_it_can_be_blended() {
        assert!((drain_at(4, 10, 0.55) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn drain_clamps_nonsense_ratios() {
        assert_eq!(drain_at(0, 10, 2.0), 0.0);
        assert_eq!(drain_at(0, 10, -1.0), 1.0);
    }

    #[test]
    fn the_sweep_crosses_the_screen_then_stops() {
        assert_eq!(sweep_x(Duration::ZERO, 100), Some(0));
        assert_eq!(sweep_x(Duration::from_millis(200), 100), Some(50));
        assert_eq!(sweep_x(SWEEP, 100), None);
        assert_eq!(sweep_x(Duration::from_secs(5), 100), None);
    }
}

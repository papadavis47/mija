use std::time::{Duration, Instant, SystemTime};

use crate::alerts::AlertDispatcher;
use crate::config::Config;
use crate::timer::Timer;

pub enum Action {
    Quit,
    Tick,
    Pause,
    Resume,
    Skip,
}

/// How often the timer counts down a second.
const TICK: Duration = Duration::from_secs(1);

/// A gap this long between readings means the machine slept or the process
/// was stopped, not that the loop lagged.
const SUSPEND_GAP: Duration = Duration::from_secs(5);

/// What the clock says the timer owes.
#[derive(Debug, PartialEq, Eq)]
pub enum Due {
    Ticks(u32),
    /// The gap was too long to replay; replaying it would fire a burst of
    /// stale transitions and alerts.
    Suspended,
}

/// Both clocks at one moment. `Instant` stops while the machine sleeps
/// (`CLOCK_MONOTONIC` on Linux, `CLOCK_UPTIME_RAW` on macOS) and the wall
/// clock does not, so a sleep shows up as the wall clock pulling ahead.
#[derive(Clone, Copy)]
pub struct Reading {
    pub mono: Instant,
    pub wall: SystemTime,
}

impl Reading {
    pub fn now() -> Self {
        Self {
            mono: Instant::now(),
            wall: SystemTime::now(),
        }
    }
}

/// Paces ticks off real time rather than the poll timeout, so holding a key
/// cannot stall the countdown.
pub struct Clock {
    next_tick: Instant,
    last: Reading,
}

impl Clock {
    pub fn new(now: Reading) -> Self {
        Self {
            next_tick: now.mono + TICK,
            last: now,
        }
    }

    pub fn due(&mut self, now: Reading) -> Due {
        let awake = now.mono.saturating_duration_since(self.last.mono);
        // A wall clock set backwards reads as no time passed.
        let wall = now.wall.duration_since(self.last.wall).unwrap_or_default();
        let slept = wall.saturating_sub(awake);
        let overdue = now.mono.saturating_duration_since(self.next_tick);
        self.last = now;

        if slept >= SUSPEND_GAP || overdue >= SUSPEND_GAP {
            self.next_tick = now.mono + TICK;
            return Due::Suspended;
        }
        let mut ticks = 0;
        while now.mono >= self.next_tick {
            self.next_tick += TICK;
            ticks += 1;
        }
        Due::Ticks(ticks)
    }
}

pub struct App {
    pub timer: Timer,
    alerts: AlertDispatcher,
    pub should_quit: bool,
    /// When the last state change happened, so the UI can play its sweep.
    pub last_transition: Option<Instant>,
    /// A message the user must acknowledge, shown over the timer until a key
    /// is pressed.
    pub notice: Option<String>,
}

impl App {
    pub fn new(config: Config, alerts: AlertDispatcher) -> Self {
        let mut timer = Timer::new(config);
        timer.start();
        Self {
            timer,
            alerts,
            should_quit: false,
            last_transition: None,
            notice: None,
        }
    }

    pub fn handle_action(&mut self, action: Action) {
        let transition = match action {
            Action::Quit => {
                self.should_quit = true;
                return;
            }
            Action::Tick => self.timer.tick(),
            Action::Pause => {
                self.timer.pause();
                None
            }
            Action::Resume => {
                self.timer.resume();
                None
            }
            Action::Skip => self.timer.skip(),
        };
        if let Some(t) = transition {
            self.last_transition = Some(Instant::now());
            self.alerts.on_transition(t);
        }
    }

    /// Map a key press to an action. While a notice is up the press only
    /// dismisses it, so a stray key cannot skip or pause — except `q`.
    pub fn press_key(&mut self, key: char) {
        if key == 'q' {
            self.handle_action(Action::Quit);
            return;
        }
        if self.notice.take().is_some() {
            return;
        }
        match key {
            ' ' => self.toggle_pause(),
            's' => self.handle_action(Action::Skip),
            _ => {}
        }
    }

    /// Apply what the clock owes. After a suspend the timer pauses where it
    /// was, so the user picks up the session rather than finding it gone.
    pub fn catch_up(&mut self, due: Due) {
        match due {
            Due::Ticks(n) => {
                for _ in 0..n {
                    self.handle_action(Action::Tick);
                }
            }
            Due::Suspended => self.handle_action(Action::Pause),
        }
    }

    pub fn toggle_pause(&mut self) {
        if self.timer.state() == crate::timer::State::Paused {
            self.handle_action(Action::Resume);
        } else {
            self.handle_action(Action::Pause);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alerts::{Alert, AlertDispatcher, AlertSender};
    use crate::timer::State;
    use std::sync::{Arc, Mutex};

    struct MockSender {
        alerts: Arc<Mutex<Vec<Alert>>>,
    }

    impl AlertSender for MockSender {
        fn send(&self, alert: Alert) {
            self.alerts.lock().unwrap().push(alert);
        }
    }

    fn test_app() -> (App, Arc<Mutex<Vec<Alert>>>) {
        let alerts = Arc::new(Mutex::new(Vec::new()));
        let sender = MockSender {
            alerts: Arc::clone(&alerts),
        };
        let dispatcher = AlertDispatcher::new(vec![Box::new(sender)]);
        let app = App::new(Config::default(), dispatcher);
        (app, alerts)
    }

    #[test]
    fn app_starts_in_work_state() {
        let (app, _) = test_app();
        assert_eq!(app.timer.state(), State::Work);
        assert!(!app.should_quit);
    }

    #[test]
    fn quit_action_sets_should_quit() {
        let (mut app, _) = test_app();
        app.handle_action(Action::Quit);
        assert!(app.should_quit);
    }

    #[test]
    fn tick_action_decrements_timer() {
        let (mut app, _) = test_app();
        let before = app.timer.remaining_secs();
        app.handle_action(Action::Tick);
        assert_eq!(app.timer.remaining_secs(), before - 1);
    }

    #[test]
    fn pause_action_pauses_timer() {
        let (mut app, _) = test_app();
        app.handle_action(Action::Pause);
        assert_eq!(app.timer.state(), State::Paused);
    }

    #[test]
    fn resume_action_resumes_timer() {
        let (mut app, _) = test_app();
        app.handle_action(Action::Pause);
        app.handle_action(Action::Resume);
        assert_eq!(app.timer.state(), State::Work);
    }

    #[test]
    fn skip_action_transitions() {
        let (mut app, _) = test_app();
        app.handle_action(Action::Skip);
        assert_eq!(app.timer.state(), State::ShortBreak);
    }

    #[test]
    fn skip_action_fires_alert() {
        let (mut app, alerts) = test_app();
        app.handle_action(Action::Skip);
        assert_eq!(alerts.lock().unwrap().len(), 1);
    }

    #[test]
    fn tick_transition_fires_alert() {
        let alerts = Arc::new(Mutex::new(Vec::new()));
        let sender = MockSender {
            alerts: Arc::clone(&alerts),
        };
        let dispatcher = AlertDispatcher::new(vec![Box::new(sender)]);
        let config = Config {
            work_duration_secs: 1,
            ..Config::default()
        };
        let mut app = App::new(config, dispatcher);
        app.handle_action(Action::Tick);
        assert_eq!(alerts.lock().unwrap().len(), 1);
    }

    #[test]
    fn transitions_are_timestamped_for_the_ui() {
        let (mut app, _) = test_app();
        assert!(app.last_transition.is_none());
        app.handle_action(Action::Skip);
        assert!(app.last_transition.is_some());
    }

    #[test]
    fn pausing_does_not_count_as_a_transition() {
        let (mut app, _) = test_app();
        app.handle_action(Action::Pause);
        assert!(app.last_transition.is_none());
    }

    #[test]
    fn toggle_pause_pauses_when_running() {
        let (mut app, _) = test_app();
        app.toggle_pause();
        assert_eq!(app.timer.state(), State::Paused);
    }

    #[test]
    fn toggle_pause_resumes_when_paused() {
        let (mut app, _) = test_app();
        app.toggle_pause();
        app.toggle_pause();
        assert_eq!(app.timer.state(), State::Work);
    }

    #[test]
    fn app_starts_without_a_notice() {
        let (app, _) = test_app();
        assert_eq!(app.notice, None);
    }

    #[test]
    fn keys_drive_the_timer_when_no_notice_is_up() {
        let (mut app, _) = test_app();
        app.press_key('s');
        assert_eq!(app.timer.state(), State::ShortBreak);
        app.press_key(' ');
        assert_eq!(app.timer.state(), State::Paused);
        app.press_key('q');
        assert!(app.should_quit);
    }

    #[test]
    fn unknown_keys_do_nothing() {
        let (mut app, _) = test_app();
        app.press_key('x');
        assert_eq!(app.timer.state(), State::Work);
        assert!(!app.should_quit);
    }

    #[test]
    fn a_key_only_dismisses_a_notice() {
        let (mut app, _) = test_app();
        app.notice = Some("created config at /x".into());
        app.press_key('s');
        assert_eq!(app.notice, None);
        assert_eq!(app.timer.state(), State::Work, "dismissing must not skip");
    }

    #[test]
    fn quit_still_works_while_a_notice_is_up() {
        let (mut app, _) = test_app();
        app.notice = Some("created config at /x".into());
        app.press_key('q');
        assert!(app.should_quit);
    }
    /// A reading taken `awake` after the start, during which the machine
    /// also slept for `asleep`: the monotonic clock skips the sleep, the
    /// wall clock does not.
    fn reading(start: (Instant, SystemTime), awake: Duration, asleep: Duration) -> Reading {
        Reading {
            mono: start.0 + awake,
            wall: start.1 + awake + asleep,
        }
    }

    fn at(start: (Instant, SystemTime), awake: Duration) -> Reading {
        reading(start, awake, Duration::ZERO)
    }

    fn start() -> (Instant, SystemTime) {
        (Instant::now(), SystemTime::now())
    }

    fn clock(start: (Instant, SystemTime)) -> Clock {
        Clock::new(at(start, Duration::ZERO))
    }

    #[test]
    fn clock_owes_nothing_before_the_first_tick() {
        let s = start();
        let mut clock = clock(s);
        assert_eq!(clock.due(at(s, Duration::from_millis(900))), Due::Ticks(0));
    }

    #[test]
    fn clock_catches_up_on_a_short_lag() {
        let s = start();
        let mut clock = clock(s);
        assert_eq!(clock.due(at(s, Duration::from_millis(3500))), Due::Ticks(3));
        // The ticks it paid are not owed again.
        assert_eq!(clock.due(at(s, Duration::from_millis(3600))), Due::Ticks(0));
        assert_eq!(clock.due(at(s, Duration::from_secs(4))), Due::Ticks(1));
    }

    #[test]
    fn clock_reports_a_machine_sleep_as_a_suspend() {
        // Monotonic time barely moves across a sleep; only the wall clock
        // shows the eight hours.
        let s = start();
        let mut clock = clock(s);
        let woke = reading(s, Duration::from_millis(500), Duration::from_secs(8 * 3600));
        assert_eq!(clock.due(woke), Due::Suspended);
    }

    #[test]
    fn clock_reports_a_stopped_process_as_a_suspend() {
        // Ctrl-Z then `fg`: both clocks ran, but the loop did not.
        let s = start();
        let mut clock = clock(s);
        assert_eq!(clock.due(at(s, Duration::from_secs(3600))), Due::Suspended);
    }

    #[test]
    fn small_wall_clock_adjustments_are_not_a_suspend() {
        let s = start();
        let mut clock = clock(s);
        let nudged = reading(s, Duration::from_secs(1), Duration::from_secs(2));
        assert_eq!(clock.due(nudged), Due::Ticks(1));
    }

    #[test]
    fn wall_clock_going_backwards_is_not_a_suspend() {
        let s = start();
        let mut clock = clock(s);
        let back = Reading {
            mono: s.0 + Duration::from_secs(1),
            wall: s.1 - Duration::from_secs(3600),
        };
        assert_eq!(clock.due(back), Due::Ticks(1));
    }

    #[test]
    fn clock_restarts_its_cadence_after_a_suspend() {
        let s = start();
        let mut clock = clock(s);
        let woke = Duration::from_secs(3600);
        clock.due(at(s, woke));
        assert_eq!(
            clock.due(at(s, woke + Duration::from_millis(900))),
            Due::Ticks(0)
        );
        assert_eq!(
            clock.due(at(s, woke + Duration::from_secs(1))),
            Due::Ticks(1)
        );
    }

    #[test]
    fn sleep_is_measured_between_readings_not_since_start() {
        // A small skew each reading must not add up to a false suspend.
        let s = start();
        let mut clock = clock(s);
        for n in 1..=10 {
            let r = reading(s, Duration::from_secs(n), Duration::from_secs(n));
            assert_eq!(clock.due(r), Due::Ticks(1), "reading {n}");
        }
    }

    #[test]
    fn catch_up_applies_owed_ticks() {
        let (mut app, _) = test_app();
        let before = app.timer.remaining_secs();
        app.catch_up(Due::Ticks(3));
        assert_eq!(app.timer.remaining_secs(), before - 3);
    }

    #[test]
    fn waking_from_suspend_pauses_without_alerts() {
        let (mut app, alerts) = test_app();
        let before = app.timer.remaining_secs();
        app.catch_up(Due::Suspended);
        assert_eq!(app.timer.state(), State::Paused);
        assert_eq!(app.timer.remaining_secs(), before);
        assert!(alerts.lock().unwrap().is_empty());
    }
}

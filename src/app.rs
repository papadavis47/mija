use std::time::Instant;

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
}

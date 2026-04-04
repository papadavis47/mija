use crate::alerts::AlertDispatcher;
use crate::config::Config;
use crate::timer::{Timer, Transition};

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
}

impl App {
    pub fn new(config: Config, alerts: AlertDispatcher) -> Self {
        let mut timer = Timer::new(config);
        timer.start();
        Self {
            timer,
            alerts,
            should_quit: false,
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
            self.alerts.on_transition(t);
        }
    }

    pub fn toggle_pause(&mut self) {
        if self.timer.state() == crate::timer::State::Paused {
            self.timer.resume();
        } else {
            self.timer.pause();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alerts::{AlertDispatcher, AlertSender};
    use crate::timer::State;
    use std::sync::{Arc, Mutex};

    struct MockSender {
        messages: Arc<Mutex<Vec<String>>>,
    }

    impl AlertSender for MockSender {
        fn send(&self, message: &str) {
            self.messages.lock().unwrap().push(message.to_string());
        }
    }

    fn test_app() -> (App, Arc<Mutex<Vec<String>>>) {
        let messages = Arc::new(Mutex::new(Vec::new()));
        let sender = MockSender {
            messages: Arc::clone(&messages),
        };
        let alerts = AlertDispatcher::new(Box::new(sender));
        let app = App::new(Config::default(), alerts);
        (app, messages)
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
        let (mut app, messages) = test_app();
        app.handle_action(Action::Skip);
        assert_eq!(messages.lock().unwrap().len(), 1);
    }

    #[test]
    fn tick_transition_fires_alert() {
        let messages = Arc::new(Mutex::new(Vec::new()));
        let sender = MockSender {
            messages: Arc::clone(&messages),
        };
        let alerts = AlertDispatcher::new(Box::new(sender));
        let config = Config {
            work_duration_secs: 1,
            ..Config::default()
        };
        let mut app = App::new(config, alerts);
        app.handle_action(Action::Tick);
        assert_eq!(messages.lock().unwrap().len(), 1);
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
}

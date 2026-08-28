use std::ffi::{OsStr, OsString};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Mutex;

use crate::timer::{State, Transition};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Alert {
    pub title: &'static str,
    pub body: &'static str,
    pub kind: AlertKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertKind {
    Completed,
    Attention,
}

pub struct AlertDispatcher {
    senders: Vec<Box<dyn AlertSender>>,
}

pub trait AlertSender: Send {
    fn send(&self, alert: Alert);
}

impl AlertDispatcher {
    pub fn new(senders: Vec<Box<dyn AlertSender>>) -> Self {
        Self { senders }
    }

    pub fn on_transition(&self, transition: Transition) {
        let alert = match (transition.from, transition.to) {
            (State::Work, State::ShortBreak) => Alert {
                title: "Mija",
                body: "🍅 Pomodoro complete! Time for a short break.",
                kind: AlertKind::Completed,
            },
            (State::Work, State::LongBreak) => Alert {
                title: "Mija",
                body: "🌴 Long break! You've earned it.",
                kind: AlertKind::Completed,
            },
            (State::ShortBreak | State::LongBreak, State::Work) => Alert {
                title: "Mija",
                body: "🍅 Back to work!",
                kind: AlertKind::Attention,
            },
            _ => return,
        };
        for sender in &self.senders {
            sender.send(alert);
        }
    }
}

trait CommandRunner: Send {
    fn run(&self, program: &OsStr, args: &[&str]);
}

struct ProcessRunner;

impl CommandRunner for ProcessRunner {
    fn run(&self, program: &OsStr, args: &[&str]) {
        let _ = Command::new(program)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}

pub struct HerdrSender {
    program: PathBuf,
    runner: Box<dyn CommandRunner>,
}

impl HerdrSender {
    pub fn from_env() -> Option<Self> {
        Self::from_environment(|name| std::env::var_os(name))
    }

    fn from_environment(get_var: impl Fn(&str) -> Option<OsString>) -> Option<Self> {
        if get_var("HERDR_ENV").as_deref() != Some(OsStr::new("1")) {
            return None;
        }

        let program = get_var("HERDR_BIN_PATH")
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| OsString::from("herdr"));
        Some(Self {
            program: PathBuf::from(program),
            runner: Box::new(ProcessRunner),
        })
    }

    #[cfg(test)]
    fn with_runner(program: PathBuf, runner: Box<dyn CommandRunner>) -> Self {
        Self { program, runner }
    }
}

impl AlertSender for HerdrSender {
    fn send(&self, alert: Alert) {
        let sound = match alert.kind {
            AlertKind::Completed => "done",
            AlertKind::Attention => "request",
        };
        self.runner.run(
            self.program.as_os_str(),
            &[
                "notification",
                "show",
                alert.title,
                "--body",
                alert.body,
                "--sound",
                sound,
            ],
        );
    }
}

pub struct BellSender {
    writer: Mutex<Box<dyn Write + Send>>,
}

impl BellSender {
    pub fn new(writer: Box<dyn Write + Send>) -> Self {
        Self {
            writer: Mutex::new(writer),
        }
    }

    pub fn stdout() -> Self {
        Self::new(Box::new(std::io::stdout()))
    }
}

impl AlertSender for BellSender {
    fn send(&self, _alert: Alert) {
        if let Ok(mut w) = self.writer.lock() {
            let _ = w.write_all(b"\x07");
            let _ = w.flush();
        }
    }
}

pub struct DesktopSender;

impl AlertSender for DesktopSender {
    fn send(&self, alert: Alert) {
        let _ = notify_rust::Notification::new()
            .summary(alert.title)
            .body(alert.body)
            .show();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    struct MockSender {
        alerts: Arc<Mutex<Vec<Alert>>>,
    }

    impl AlertSender for MockSender {
        fn send(&self, alert: Alert) {
            self.alerts.lock().unwrap().push(alert);
        }
    }

    fn mock_dispatcher() -> (AlertDispatcher, Arc<Mutex<Vec<Alert>>>) {
        let alerts = Arc::new(Mutex::new(Vec::new()));
        let sender = MockSender {
            alerts: Arc::clone(&alerts),
        };
        (AlertDispatcher::new(vec![Box::new(sender)]), alerts)
    }

    #[test]
    fn alerts_on_work_to_short_break() {
        let (dispatcher, alerts) = mock_dispatcher();
        dispatcher.on_transition(Transition {
            from: State::Work,
            to: State::ShortBreak,
        });
        let alerts = alerts.lock().unwrap();
        assert_eq!(alerts.len(), 1);
        assert!(alerts[0].body.contains("Pomodoro complete"));
        assert_eq!(alerts[0].kind, AlertKind::Completed);
    }

    #[test]
    fn alerts_on_work_to_long_break() {
        let (dispatcher, alerts) = mock_dispatcher();
        dispatcher.on_transition(Transition {
            from: State::Work,
            to: State::LongBreak,
        });
        let alerts = alerts.lock().unwrap();
        assert_eq!(alerts.len(), 1);
        assert!(alerts[0].body.contains("Long break"));
        assert_eq!(alerts[0].kind, AlertKind::Completed);
    }

    #[test]
    fn alerts_on_break_to_work() {
        let (dispatcher, alerts) = mock_dispatcher();
        dispatcher.on_transition(Transition {
            from: State::ShortBreak,
            to: State::Work,
        });
        let alerts = alerts.lock().unwrap();
        assert_eq!(alerts.len(), 1);
        assert!(alerts[0].body.contains("Back to work"));
        assert_eq!(alerts[0].kind, AlertKind::Attention);
    }

    #[test]
    fn alerts_on_long_break_to_work() {
        let (dispatcher, alerts) = mock_dispatcher();
        dispatcher.on_transition(Transition {
            from: State::LongBreak,
            to: State::Work,
        });
        let alerts = alerts.lock().unwrap();
        assert_eq!(alerts.len(), 1);
        assert!(alerts[0].body.contains("Back to work"));
    }

    #[test]
    fn message_for_work_complete_includes_tomato() {
        let (dispatcher, alerts) = mock_dispatcher();
        dispatcher.on_transition(Transition {
            from: State::Work,
            to: State::ShortBreak,
        });
        assert!(alerts.lock().unwrap()[0].body.contains("🍅"));
    }

    #[test]
    fn message_for_long_break_includes_palm() {
        let (dispatcher, alerts) = mock_dispatcher();
        dispatcher.on_transition(Transition {
            from: State::Work,
            to: State::LongBreak,
        });
        assert!(alerts.lock().unwrap()[0].body.contains("🌴"));
    }

    #[test]
    fn dispatcher_sends_to_multiple_senders() {
        let alerts_a = Arc::new(Mutex::new(Vec::new()));
        let alerts_b = Arc::new(Mutex::new(Vec::new()));
        let sender_a = MockSender {
            alerts: Arc::clone(&alerts_a),
        };
        let sender_b = MockSender {
            alerts: Arc::clone(&alerts_b),
        };
        let dispatcher = AlertDispatcher::new(vec![Box::new(sender_a), Box::new(sender_b)]);
        dispatcher.on_transition(Transition {
            from: State::Work,
            to: State::ShortBreak,
        });
        assert_eq!(alerts_a.lock().unwrap().len(), 1);
        assert_eq!(alerts_b.lock().unwrap().len(), 1);
    }

    #[test]
    fn dispatcher_with_no_senders_does_not_panic() {
        let dispatcher = AlertDispatcher::new(vec![]);
        dispatcher.on_transition(Transition {
            from: State::Work,
            to: State::ShortBreak,
        });
    }

    #[test]
    fn bell_sender_produces_bell_character() {
        let output = Arc::new(Mutex::new(Vec::new()));
        let sender = BellSender::new(Box::new(MockWriter(Arc::clone(&output))));
        sender.send(short_break_alert());
        let bytes = output.lock().unwrap();
        assert_eq!(&*bytes, b"\x07");
    }

    #[test]
    fn herdr_sender_uses_done_sound_for_completed_work() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let sender = HerdrSender::with_runner(
            PathBuf::from("/opt/herdr"),
            Box::new(MockRunner(Arc::clone(&calls))),
        );

        sender.send(short_break_alert());

        let calls = calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, PathBuf::from("/opt/herdr"));
        assert_eq!(
            calls[0].1,
            [
                "notification",
                "show",
                "Mija",
                "--body",
                "🍅 Pomodoro complete! Time for a short break.",
                "--sound",
                "done"
            ]
        );
    }

    #[test]
    fn herdr_sender_uses_request_sound_when_break_ends() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let sender = HerdrSender::with_runner(
            PathBuf::from("herdr"),
            Box::new(MockRunner(Arc::clone(&calls))),
        );

        sender.send(Alert {
            title: "Mija",
            body: "🍅 Back to work!",
            kind: AlertKind::Attention,
        });

        assert_eq!(calls.lock().unwrap()[0].1[6], "request");
    }

    #[test]
    fn herdr_sender_is_enabled_only_inside_herdr() {
        assert!(HerdrSender::from_environment(|_| None).is_none());
        assert!(
            HerdrSender::from_environment(|name| {
                (name == "HERDR_ENV").then(|| OsString::from("0"))
            })
            .is_none()
        );
        assert!(
            HerdrSender::from_environment(|name| {
                (name == "HERDR_ENV").then(|| OsString::from("1"))
            })
            .is_some()
        );
    }

    #[test]
    fn herdr_sender_prefers_injected_binary_path() {
        let sender = HerdrSender::from_environment(|name| match name {
            "HERDR_ENV" => Some(OsString::from("1")),
            "HERDR_BIN_PATH" => Some(OsString::from("/custom/herdr")),
            _ => None,
        })
        .unwrap();

        assert_eq!(sender.program, PathBuf::from("/custom/herdr"));
    }

    fn short_break_alert() -> Alert {
        Alert {
            title: "Mija",
            body: "🍅 Pomodoro complete! Time for a short break.",
            kind: AlertKind::Completed,
        }
    }

    type Calls = Arc<Mutex<Vec<(PathBuf, Vec<String>)>>>;

    struct MockRunner(Calls);

    impl CommandRunner for MockRunner {
        fn run(&self, program: &OsStr, args: &[&str]) {
            self.0.lock().unwrap().push((
                PathBuf::from(program),
                args.iter().map(|arg| (*arg).to_string()).collect(),
            ));
        }
    }

    struct MockWriter(Arc<Mutex<Vec<u8>>>);

    impl std::io::Write for MockWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
}

use std::io::Write;
use std::sync::Mutex;

use crate::timer::{State, Transition};

pub struct AlertDispatcher {
    senders: Vec<Box<dyn AlertSender>>,
}

pub trait AlertSender: Send {
    fn send(&self, message: &str);
}

impl AlertDispatcher {
    pub fn new(sender: Box<dyn AlertSender>) -> Self {
        Self {
            senders: vec![sender],
        }
    }

    pub fn with_senders(senders: Vec<Box<dyn AlertSender>>) -> Self {
        Self { senders }
    }

    pub fn on_transition(&self, transition: Transition) {
        let message = match (transition.from, transition.to) {
            (State::Work, State::ShortBreak) => "🍅 Pomodoro complete! Time for a short break.",
            (State::Work, State::LongBreak) => "🌴 Long break! You've earned it.",
            (State::ShortBreak | State::LongBreak, State::Work) => "🍅 Back to work!",
            _ => return,
        };
        for sender in &self.senders {
            sender.send(message);
        }
    }
}

pub struct TmuxSender;

impl AlertSender for TmuxSender {
    fn send(&self, message: &str) {
        let _ = std::process::Command::new("tmux")
            .args(["display-message", message])
            .spawn();
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
    fn send(&self, _message: &str) {
        if let Ok(mut w) = self.writer.lock() {
            let _ = w.write_all(b"\x07");
            let _ = w.flush();
        }
    }
}

pub struct DesktopSender;

impl AlertSender for DesktopSender {
    fn send(&self, message: &str) {
        let _ = notify_rust::Notification::new()
            .summary("Linda")
            .body(message)
            .show();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    struct MockSender {
        messages: Arc<Mutex<Vec<String>>>,
    }

    impl AlertSender for MockSender {
        fn send(&self, message: &str) {
            self.messages.lock().unwrap().push(message.to_string());
        }
    }

    fn mock_dispatcher() -> (AlertDispatcher, Arc<Mutex<Vec<String>>>) {
        let messages = Arc::new(Mutex::new(Vec::new()));
        let sender = MockSender {
            messages: Arc::clone(&messages),
        };
        (AlertDispatcher::new(Box::new(sender)), messages)
    }

    #[test]
    fn alerts_on_work_to_short_break() {
        let (dispatcher, messages) = mock_dispatcher();
        dispatcher.on_transition(Transition {
            from: State::Work,
            to: State::ShortBreak,
        });
        let msgs = messages.lock().unwrap();
        assert_eq!(msgs.len(), 1);
        assert!(msgs[0].contains("Pomodoro complete"));
    }

    #[test]
    fn alerts_on_work_to_long_break() {
        let (dispatcher, messages) = mock_dispatcher();
        dispatcher.on_transition(Transition {
            from: State::Work,
            to: State::LongBreak,
        });
        let msgs = messages.lock().unwrap();
        assert_eq!(msgs.len(), 1);
        assert!(msgs[0].contains("Long break"));
    }

    #[test]
    fn alerts_on_break_to_work() {
        let (dispatcher, messages) = mock_dispatcher();
        dispatcher.on_transition(Transition {
            from: State::ShortBreak,
            to: State::Work,
        });
        let msgs = messages.lock().unwrap();
        assert_eq!(msgs.len(), 1);
        assert!(msgs[0].contains("Back to work"));
    }

    #[test]
    fn alerts_on_long_break_to_work() {
        let (dispatcher, messages) = mock_dispatcher();
        dispatcher.on_transition(Transition {
            from: State::LongBreak,
            to: State::Work,
        });
        let msgs = messages.lock().unwrap();
        assert_eq!(msgs.len(), 1);
        assert!(msgs[0].contains("Back to work"));
    }

    #[test]
    fn message_for_work_complete_includes_tomato() {
        let (dispatcher, messages) = mock_dispatcher();
        dispatcher.on_transition(Transition {
            from: State::Work,
            to: State::ShortBreak,
        });
        let msgs = messages.lock().unwrap();
        assert!(msgs[0].contains("🍅"));
    }

    #[test]
    fn message_for_long_break_includes_palm() {
        let (dispatcher, messages) = mock_dispatcher();
        dispatcher.on_transition(Transition {
            from: State::Work,
            to: State::LongBreak,
        });
        let msgs = messages.lock().unwrap();
        assert!(msgs[0].contains("🌴"));
    }

    #[test]
    fn dispatcher_sends_to_multiple_senders() {
        let messages_a = Arc::new(Mutex::new(Vec::new()));
        let messages_b = Arc::new(Mutex::new(Vec::new()));
        let sender_a = MockSender {
            messages: Arc::clone(&messages_a),
        };
        let sender_b = MockSender {
            messages: Arc::clone(&messages_b),
        };
        let dispatcher = AlertDispatcher::with_senders(vec![
            Box::new(sender_a),
            Box::new(sender_b),
        ]);
        dispatcher.on_transition(Transition {
            from: State::Work,
            to: State::ShortBreak,
        });
        assert_eq!(messages_a.lock().unwrap().len(), 1);
        assert_eq!(messages_b.lock().unwrap().len(), 1);
    }

    #[test]
    fn dispatcher_with_no_senders_does_not_panic() {
        let dispatcher = AlertDispatcher::with_senders(vec![]);
        dispatcher.on_transition(Transition {
            from: State::Work,
            to: State::ShortBreak,
        });
    }

    #[test]
    fn bell_sender_produces_bell_character() {
        let output = Arc::new(Mutex::new(Vec::new()));
        let sender = BellSender::new(Box::new(MockWriter(Arc::clone(&output))));
        sender.send("ignored message");
        let bytes = output.lock().unwrap();
        assert_eq!(&*bytes, b"\x07");
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

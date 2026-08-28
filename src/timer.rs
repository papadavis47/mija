use crate::config::Config;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Transition {
    pub from: State,
    pub to: State,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Idle,
    Work,
    ShortBreak,
    LongBreak,
    Paused,
}

pub struct Timer {
    config: Config,
    state: State,
    previous_state: State,
    remaining_secs: u32,
    completed_pomodoros: u32,
    rounds_in_cycle: u32,
}

impl Timer {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            state: State::Idle,
            previous_state: State::Idle,
            remaining_secs: 0,
            completed_pomodoros: 0,
            rounds_in_cycle: 0,
        }
    }

    pub fn state(&self) -> State {
        self.state
    }

    pub fn remaining_secs(&self) -> u32 {
        self.remaining_secs
    }

    pub fn completed_pomodoros(&self) -> u32 {
        self.completed_pomodoros
    }

    pub fn start(&mut self) {
        self.state = State::Work;
        self.remaining_secs = self.config.work_duration_secs;
    }

    pub fn tick(&mut self) -> Option<Transition> {
        match self.state {
            State::Idle | State::Paused => None,
            State::Work | State::ShortBreak | State::LongBreak => {
                if self.remaining_secs > 1 {
                    self.remaining_secs -= 1;
                    None
                } else {
                    let from = self.state;
                    self.do_transition();
                    Some(Transition {
                        from,
                        to: self.state,
                    })
                }
            }
        }
    }

    pub fn pause(&mut self) {
        match self.state {
            State::Work | State::ShortBreak | State::LongBreak => {
                self.previous_state = self.state;
                self.state = State::Paused;
            }
            _ => {}
        }
    }

    pub fn resume(&mut self) {
        if self.state == State::Paused {
            self.state = self.previous_state;
        }
    }

    pub fn format_remaining(&self) -> String {
        let remaining_secs = self.remaining_secs();
        let minutes = remaining_secs / 60;
        let seconds = remaining_secs % 60;
        format!("{minutes:02}:{seconds:02}")
    }

    pub fn status_label(&self) -> &'static str {
        match self.state {
            State::Idle => "⏹ idle",
            State::Work => "🍅 work",
            State::ShortBreak => "☕ short break",
            State::LongBreak => "🌴 long break",
            State::Paused => "⏸ paused",
        }
    }

    pub fn format_status(&self) -> String {
        format!("{} {}", self.status_label(), self.format_remaining())
    }

    pub fn skip(&mut self) -> Option<Transition> {
        match self.state {
            State::Work | State::ShortBreak | State::LongBreak => {
                let from = self.state;
                self.do_transition();
                Some(Transition {
                    from,
                    to: self.state,
                })
            }
            _ => None,
        }
    }

    pub fn progress(&self) -> f64 {
        let total = match self.state {
            State::Idle => return 0.0,
            State::Work => self.config.work_duration_secs,
            State::ShortBreak => self.config.short_break_duration_secs,
            State::LongBreak => self.config.long_break_duration_secs,
            State::Paused => match self.previous_state {
                State::Work => self.config.work_duration_secs,
                State::ShortBreak => self.config.short_break_duration_secs,
                State::LongBreak => self.config.long_break_duration_secs,
                _ => return 0.0,
            },
        };
        if total == 0 {
            return 0.0;
        }
        self.remaining_secs as f64 / total as f64
    }

    pub fn current_round(&self) -> u32 {
        self.rounds_in_cycle + if self.state == State::Work { 1 } else { 0 }
    }

    pub fn total_rounds(&self) -> u32 {
        self.config.rounds_before_long_break
    }

    fn do_transition(&mut self) {
        match self.state {
            State::Work => {
                self.completed_pomodoros += 1;
                self.rounds_in_cycle += 1;
                if self.rounds_in_cycle >= self.config.rounds_before_long_break {
                    self.state = State::LongBreak;
                    self.remaining_secs = self.config.long_break_duration_secs;
                    self.rounds_in_cycle = 0;
                } else {
                    self.state = State::ShortBreak;
                    self.remaining_secs = self.config.short_break_duration_secs;
                }
            }
            State::ShortBreak | State::LongBreak => {
                self.state = State::Work;
                self.remaining_secs = self.config.work_duration_secs;
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_timer_starts_idle() {
        let timer = Timer::new(Config::default());
        assert_eq!(timer.state(), State::Idle);
    }

    #[test]
    fn new_timer_has_zero_completed_pomodoros() {
        let timer = Timer::new(Config::default());
        assert_eq!(timer.completed_pomodoros(), 0);
    }

    #[test]
    fn new_timer_remaining_is_zero_when_idle() {
        let timer = Timer::new(Config::default());
        assert_eq!(timer.remaining_secs(), 0);
    }

    #[test]
    fn start_transitions_to_work() {
        let mut timer = Timer::new(Config::default());
        timer.start();
        assert_eq!(timer.state(), State::Work);
    }

    #[test]
    fn start_sets_remaining_to_work_duration() {
        let mut timer = Timer::new(Config::default());
        timer.start();
        assert_eq!(timer.remaining_secs(), 25 * 60);
    }

    #[test]
    fn tick_decrements_remaining() {
        let mut timer = Timer::new(Config::default());
        timer.start();
        timer.tick();
        assert_eq!(timer.remaining_secs(), 25 * 60 - 1);
    }

    #[test]
    fn tick_does_nothing_when_idle() {
        let mut timer = Timer::new(Config::default());
        timer.tick();
        assert_eq!(timer.remaining_secs(), 0);
        assert_eq!(timer.state(), State::Idle);
    }

    #[test]
    fn work_session_ends_with_short_break() {
        let config = Config {
            work_duration_secs: 3,
            ..Config::default()
        };
        let mut timer = Timer::new(config);
        timer.start();
        timer.tick(); // 2
        timer.tick(); // 1
        timer.tick(); // 0 → transition
        assert_eq!(timer.state(), State::ShortBreak);
        assert_eq!(timer.remaining_secs(), 5 * 60);
    }

    #[test]
    fn completing_work_increments_pomodoro_count() {
        let config = Config {
            work_duration_secs: 1,
            ..Config::default()
        };
        let mut timer = Timer::new(config);
        timer.start();
        timer.tick(); // 0 → transition
        assert_eq!(timer.completed_pomodoros(), 1);
    }

    #[test]
    fn short_break_ends_with_work() {
        let config = Config {
            work_duration_secs: 1,
            short_break_duration_secs: 1,
            ..Config::default()
        };
        let mut timer = Timer::new(config);
        timer.start();
        timer.tick(); // work done → short break
        timer.tick(); // short break done → work
        assert_eq!(timer.state(), State::Work);
        assert_eq!(timer.remaining_secs(), 1);
    }

    #[test]
    fn fourth_work_session_followed_by_long_break() {
        let config = Config {
            work_duration_secs: 1,
            short_break_duration_secs: 1,
            long_break_duration_secs: 900,
            rounds_before_long_break: 4,
        };
        let mut timer = Timer::new(config);
        timer.start();

        // Pomodoro 1: work → short break
        timer.tick();
        timer.tick();
        // Pomodoro 2: work → short break
        timer.tick();
        timer.tick();
        // Pomodoro 3: work → short break
        timer.tick();
        timer.tick();
        // Pomodoro 4: work → should be long break
        timer.tick();

        assert_eq!(timer.completed_pomodoros(), 4);
        assert_eq!(timer.state(), State::LongBreak);
        assert_eq!(timer.remaining_secs(), 900);
    }

    #[test]
    fn long_break_ends_with_work_and_resets_round_count() {
        let config = Config {
            work_duration_secs: 1,
            short_break_duration_secs: 1,
            long_break_duration_secs: 1,
            rounds_before_long_break: 4,
        };
        let mut timer = Timer::new(config);
        timer.start();

        // 4 pomodoros: work+break each (7 ticks for 3 short + 1 into long)
        for _ in 0..7 {
            timer.tick();
        }
        assert_eq!(timer.state(), State::LongBreak);

        timer.tick(); // long break done → work
        assert_eq!(timer.state(), State::Work);
        assert_eq!(timer.remaining_secs(), 1);
    }

    #[test]
    fn pause_during_work_preserves_remaining() {
        let mut timer = Timer::new(Config::default());
        timer.start();
        timer.tick(); // 1499
        timer.tick(); // 1498
        timer.pause();
        assert_eq!(timer.state(), State::Paused);
        assert_eq!(timer.remaining_secs(), 25 * 60 - 2);
    }

    #[test]
    fn tick_does_nothing_when_paused() {
        let mut timer = Timer::new(Config::default());
        timer.start();
        timer.tick();
        timer.pause();
        let remaining = timer.remaining_secs();
        timer.tick();
        assert_eq!(timer.remaining_secs(), remaining);
    }

    #[test]
    fn resume_returns_to_previous_state() {
        let mut timer = Timer::new(Config::default());
        timer.start();
        timer.tick();
        timer.pause();
        timer.resume();
        assert_eq!(timer.state(), State::Work);
        assert_eq!(timer.remaining_secs(), 25 * 60 - 1);
    }

    #[test]
    fn pause_during_break_preserves_state() {
        let config = Config {
            work_duration_secs: 1,
            short_break_duration_secs: 60,
            ..Config::default()
        };
        let mut timer = Timer::new(config);
        timer.start();
        timer.tick(); // work done → short break (60s)
        timer.tick(); // 59
        timer.pause();
        timer.resume();
        assert_eq!(timer.state(), State::ShortBreak);
        assert_eq!(timer.remaining_secs(), 59);
    }

    #[test]
    fn pause_when_idle_does_nothing() {
        let mut timer = Timer::new(Config::default());
        timer.pause();
        assert_eq!(timer.state(), State::Idle);
    }

    #[test]
    fn resume_when_not_paused_does_nothing() {
        let mut timer = Timer::new(Config::default());
        timer.start();
        let state = timer.state();
        timer.resume();
        assert_eq!(timer.state(), state);
    }

    #[test]
    fn format_remaining_shows_mm_ss() {
        let config = Config {
            work_duration_secs: 754,
            ..Config::default()
        };
        let mut timer = Timer::new(config);
        timer.start();
        assert_eq!(timer.format_remaining(), "12:34");
    }

    #[test]
    fn format_remaining_zero() {
        let timer = Timer::new(Config::default());
        assert_eq!(timer.format_remaining(), "00:00");
    }

    #[test]
    fn tick_returns_none_when_no_transition() {
        let mut timer = Timer::new(Config::default());
        timer.start();
        assert_eq!(timer.tick(), None);
    }

    #[test]
    fn tick_returns_transition_from_work_to_short_break() {
        let config = Config {
            work_duration_secs: 1,
            ..Config::default()
        };
        let mut timer = Timer::new(config);
        timer.start();
        let result = timer.tick();
        assert_eq!(
            result,
            Some(Transition {
                from: State::Work,
                to: State::ShortBreak,
            })
        );
    }

    #[test]
    fn tick_returns_transition_from_work_to_long_break() {
        let config = Config {
            work_duration_secs: 1,
            short_break_duration_secs: 1,
            rounds_before_long_break: 1,
            ..Config::default()
        };
        let mut timer = Timer::new(config);
        timer.start();
        let result = timer.tick();
        assert_eq!(
            result,
            Some(Transition {
                from: State::Work,
                to: State::LongBreak,
            })
        );
    }

    #[test]
    fn tick_returns_transition_from_break_to_work() {
        let config = Config {
            work_duration_secs: 1,
            short_break_duration_secs: 1,
            ..Config::default()
        };
        let mut timer = Timer::new(config);
        timer.start();
        timer.tick(); // work → short break
        let result = timer.tick(); // short break → work
        assert_eq!(
            result,
            Some(Transition {
                from: State::ShortBreak,
                to: State::Work,
            })
        );
    }

    #[test]
    fn tick_returns_none_when_idle() {
        let mut timer = Timer::new(Config::default());
        assert_eq!(timer.tick(), None);
    }

    #[test]
    fn tick_returns_none_when_paused() {
        let mut timer = Timer::new(Config::default());
        timer.start();
        timer.pause();
        assert_eq!(timer.tick(), None);
    }

    #[test]
    fn status_label_for_work() {
        let mut timer = Timer::new(Config::default());
        timer.start();
        assert_eq!(timer.status_label(), "🍅 work");
    }

    #[test]
    fn status_label_for_short_break() {
        let config = Config {
            work_duration_secs: 1,
            ..Config::default()
        };
        let mut timer = Timer::new(config);
        timer.start();
        timer.tick();
        assert_eq!(timer.status_label(), "☕ short break");
    }

    #[test]
    fn status_label_for_long_break() {
        let config = Config {
            work_duration_secs: 1,
            short_break_duration_secs: 1,
            rounds_before_long_break: 1,
            ..Config::default()
        };
        let mut timer = Timer::new(config);
        timer.start();
        timer.tick();
        assert_eq!(timer.status_label(), "🌴 long break");
    }

    #[test]
    fn status_label_for_idle() {
        let timer = Timer::new(Config::default());
        assert_eq!(timer.status_label(), "⏹ idle");
    }

    #[test]
    fn status_label_for_paused() {
        let mut timer = Timer::new(Config::default());
        timer.start();
        timer.pause();
        assert_eq!(timer.status_label(), "⏸ paused");
    }

    #[test]
    fn format_status_line() {
        let config = Config {
            work_duration_secs: 754,
            ..Config::default()
        };
        let mut timer = Timer::new(config);
        timer.start();
        assert_eq!(timer.format_status(), "🍅 work 12:34");
    }

    #[test]
    fn skip_during_work_transitions_to_break() {
        let mut timer = Timer::new(Config::default());
        timer.start();
        timer.tick();
        let result = timer.skip();
        assert_eq!(timer.state(), State::ShortBreak);
        assert_eq!(
            result,
            Some(Transition {
                from: State::Work,
                to: State::ShortBreak,
            })
        );
    }

    #[test]
    fn skip_during_work_increments_pomodoro_count() {
        let mut timer = Timer::new(Config::default());
        timer.start();
        timer.skip();
        assert_eq!(timer.completed_pomodoros(), 1);
    }

    #[test]
    fn skip_during_break_transitions_to_work() {
        let config = Config {
            work_duration_secs: 1,
            ..Config::default()
        };
        let mut timer = Timer::new(config);
        timer.start();
        timer.tick(); // work → short break
        let result = timer.skip();
        assert_eq!(timer.state(), State::Work);
        assert_eq!(
            result,
            Some(Transition {
                from: State::ShortBreak,
                to: State::Work,
            })
        );
    }

    #[test]
    fn skip_when_idle_does_nothing() {
        let mut timer = Timer::new(Config::default());
        assert_eq!(timer.skip(), None);
        assert_eq!(timer.state(), State::Idle);
    }

    #[test]
    fn skip_when_paused_does_nothing() {
        let mut timer = Timer::new(Config::default());
        timer.start();
        timer.pause();
        assert_eq!(timer.skip(), None);
        assert_eq!(timer.state(), State::Paused);
    }

    #[test]
    fn progress_ratio_at_start_is_one() {
        let mut timer = Timer::new(Config::default());
        timer.start();
        assert!((timer.progress() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn progress_ratio_halfway() {
        let config = Config {
            work_duration_secs: 100,
            ..Config::default()
        };
        let mut timer = Timer::new(config);
        timer.start();
        for _ in 0..50 {
            timer.tick();
        }
        assert!((timer.progress() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn progress_when_idle_is_zero() {
        let timer = Timer::new(Config::default());
        assert!((timer.progress() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn rounds_in_cycle_accessor() {
        let config = Config {
            work_duration_secs: 1,
            short_break_duration_secs: 1,
            ..Config::default()
        };
        let mut timer = Timer::new(config);
        timer.start();
        assert_eq!(timer.current_round(), 1);
        timer.tick(); // work done
        timer.tick(); // break done
        assert_eq!(timer.current_round(), 2);
    }

    #[test]
    fn total_rounds_accessor() {
        let timer = Timer::new(Config::default());
        assert_eq!(timer.total_rounds(), 4);
    }
}

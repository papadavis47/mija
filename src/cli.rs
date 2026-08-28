use clap::Parser;

use crate::config::Config;

#[derive(Parser)]
#[command(name = "mija", about = "A pomodoro timer for the terminal")]
pub struct Args {
    /// Work duration in minutes
    #[arg(long, default_value_t = 25)]
    pub work: u32,

    /// Short break duration in minutes
    #[arg(long, default_value_t = 5)]
    pub short_break: u32,

    /// Long break duration in minutes
    #[arg(long, default_value_t = 15)]
    pub long_break: u32,

    /// Number of work rounds before a long break
    #[arg(long, default_value_t = 4)]
    pub rounds: u32,

    /// Send a terminal bell on state transitions
    #[arg(long)]
    pub bell: bool,

    /// Send desktop notifications on state transitions
    #[arg(long)]
    pub notify: bool,
}

impl Args {
    pub fn to_config(&self) -> Config {
        Config {
            work_duration_secs: self.work * 60,
            short_break_duration_secs: self.short_break * 60,
            long_break_duration_secs: self.long_break * 60,
            rounds_before_long_break: self.rounds,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_work_duration() {
        let args = Args::parse_from(["mija", "--work", "30"]);
        let config = args.to_config();
        assert_eq!(config.work_duration_secs, 30 * 60);
    }

    #[test]
    fn custom_short_break() {
        let args = Args::parse_from(["mija", "--short-break", "10"]);
        let config = args.to_config();
        assert_eq!(config.short_break_duration_secs, 10 * 60);
    }

    #[test]
    fn custom_long_break() {
        let args = Args::parse_from(["mija", "--long-break", "20"]);
        let config = args.to_config();
        assert_eq!(config.long_break_duration_secs, 20 * 60);
    }

    #[test]
    fn custom_rounds() {
        let args = Args::parse_from(["mija", "--rounds", "6"]);
        let config = args.to_config();
        assert_eq!(config.rounds_before_long_break, 6);
    }

    #[test]
    fn default_no_bell() {
        let args = Args::parse_from(["mija"]);
        assert!(!args.bell);
    }

    #[test]
    fn bell_flag() {
        let args = Args::parse_from(["mija", "--bell"]);
        assert!(args.bell);
    }

    #[test]
    fn default_no_notify() {
        let args = Args::parse_from(["mija"]);
        assert!(!args.notify);
    }

    #[test]
    fn notify_flag() {
        let args = Args::parse_from(["mija", "--notify"]);
        assert!(args.notify);
    }

    #[test]
    fn defaults_match_config_defaults() {
        let args = Args::parse_from(["mija"]);
        let config = args.to_config();
        let default = Config::default();
        assert_eq!(config.work_duration_secs, default.work_duration_secs);
        assert_eq!(
            config.short_break_duration_secs,
            default.short_break_duration_secs
        );
        assert_eq!(
            config.long_break_duration_secs,
            default.long_break_duration_secs
        );
        assert_eq!(
            config.rounds_before_long_break,
            default.rounds_before_long_break
        );
    }
}

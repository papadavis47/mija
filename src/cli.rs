use clap::Parser;

use crate::config::{Config, FileConfig};

#[derive(Parser)]
#[command(name = "mija", version, about = "A pomodoro timer for the terminal")]
pub struct Args {
    /// Work duration in minutes [default: 25]
    #[arg(long)]
    pub work: Option<u32>,

    /// Short break duration in minutes [default: 5]
    #[arg(long)]
    pub short_break: Option<u32>,

    /// Long break duration in minutes [default: 15]
    #[arg(long)]
    pub long_break: Option<u32>,

    /// Number of work rounds before a long break [default: 4]
    #[arg(long)]
    pub rounds: Option<u32>,

    /// Send a terminal bell on state transitions
    #[arg(long)]
    pub bell: bool,

    /// Send desktop notifications on state transitions
    #[arg(long)]
    pub notify: bool,
}

/// Pick a duration from the flag, then the config file, then the built-in
/// default. The first two are minutes; the fallback is already seconds.
fn resolve_minutes(flag: Option<u32>, file: Option<u32>, default_secs: u32) -> u32 {
    flag.or(file).map(|min| min * 60).unwrap_or(default_secs)
}

impl Args {
    pub fn to_config(&self, file: &FileConfig) -> Config {
        let defaults = Config::default();
        Config {
            work_duration_secs: resolve_minutes(self.work, file.work, defaults.work_duration_secs),
            short_break_duration_secs: resolve_minutes(
                self.short_break,
                file.short_break,
                defaults.short_break_duration_secs,
            ),
            long_break_duration_secs: resolve_minutes(
                self.long_break,
                file.long_break,
                defaults.long_break_duration_secs,
            ),
            rounds_before_long_break: self
                .rounds
                .or(file.rounds)
                .unwrap_or(defaults.rounds_before_long_break),
        }
    }

    /// A flag can only switch an alert on, so the flag and the file are OR'd.
    pub fn bell_enabled(&self, file: &FileConfig) -> bool {
        self.bell || file.bell.unwrap_or(false)
    }

    pub fn notify_enabled(&self, file: &FileConfig) -> bool {
        self.notify || file.notify.unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_file() -> FileConfig {
        FileConfig::default()
    }

    #[test]
    fn custom_work_duration() {
        let args = Args::parse_from(["mija", "--work", "30"]);
        let config = args.to_config(&no_file());
        assert_eq!(config.work_duration_secs, 30 * 60);
    }

    #[test]
    fn custom_short_break() {
        let args = Args::parse_from(["mija", "--short-break", "10"]);
        let config = args.to_config(&no_file());
        assert_eq!(config.short_break_duration_secs, 10 * 60);
    }

    #[test]
    fn custom_long_break() {
        let args = Args::parse_from(["mija", "--long-break", "20"]);
        let config = args.to_config(&no_file());
        assert_eq!(config.long_break_duration_secs, 20 * 60);
    }

    #[test]
    fn custom_rounds() {
        let args = Args::parse_from(["mija", "--rounds", "6"]);
        let config = args.to_config(&no_file());
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
        let config = args.to_config(&no_file());
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

    #[test]
    fn the_file_beats_the_built_in_default() {
        let args = Args::parse_from(["mija"]);
        let file = FileConfig {
            work: Some(30),
            rounds: Some(3),
            ..FileConfig::default()
        };
        let config = args.to_config(&file);
        assert_eq!(config.work_duration_secs, 30 * 60);
        assert_eq!(config.rounds_before_long_break, 3);
    }

    #[test]
    fn a_flag_beats_the_file() {
        let args = Args::parse_from(["mija", "--work", "5"]);
        let file = FileConfig {
            work: Some(30),
            ..FileConfig::default()
        };
        assert_eq!(args.to_config(&file).work_duration_secs, 5 * 60);
    }

    #[test]
    fn the_file_only_fills_the_keys_it_sets() {
        let args = Args::parse_from(["mija"]);
        let file = FileConfig {
            work: Some(30),
            ..FileConfig::default()
        };
        let config = args.to_config(&file);
        let default = Config::default();
        assert_eq!(config.work_duration_secs, 30 * 60);
        assert_eq!(
            config.short_break_duration_secs,
            default.short_break_duration_secs
        );
    }

    #[test]
    fn the_file_can_turn_on_alerts() {
        let args = Args::parse_from(["mija"]);
        let file = FileConfig {
            bell: Some(true),
            notify: Some(true),
            ..FileConfig::default()
        };
        assert!(args.bell_enabled(&file));
        assert!(args.notify_enabled(&file));
    }

    #[test]
    fn a_flag_turns_on_alerts_the_file_leaves_off() {
        let args = Args::parse_from(["mija", "--bell", "--notify"]);
        let file = FileConfig {
            bell: Some(false),
            notify: Some(false),
            ..FileConfig::default()
        };
        assert!(args.bell_enabled(&file));
        assert!(args.notify_enabled(&file));
    }

    #[test]
    fn alerts_stay_off_when_nothing_asks_for_them() {
        let args = Args::parse_from(["mija"]);
        assert!(!args.bell_enabled(&no_file()));
        assert!(!args.notify_enabled(&no_file()));
    }

    #[test]
    fn version_flag_reports_the_package_version() {
        let err = match Args::try_parse_from(["mija", "--version"]) {
            Ok(_) => panic!("--version should exit early"),
            Err(err) => err,
        };
        assert_eq!(err.kind(), clap::error::ErrorKind::DisplayVersion);
        assert!(err.to_string().contains(env!("CARGO_PKG_VERSION")));
    }
}

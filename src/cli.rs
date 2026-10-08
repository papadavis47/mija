use clap::{CommandFactory, FromArgMatches, Parser};
use std::ffi::OsString;
use std::path::Path;

use crate::config::{self, Config, FileConfig};

#[derive(Parser)]
#[command(name = "mija", version, about = "A pomodoro timer for the terminal")]
pub struct Args {
    /// Work duration in minutes, 1–1440 [default: 25]
    #[arg(long, value_parser = minutes)]
    pub work: Option<u32>,

    /// Short break duration in minutes, 1–1440 [default: 5]
    #[arg(long, value_parser = minutes)]
    pub short_break: Option<u32>,

    /// Long break duration in minutes, 1–1440 [default: 15]
    #[arg(long, value_parser = minutes)]
    pub long_break: Option<u32>,

    /// Number of work rounds before a long break, 1–24 [default: 4]
    #[arg(long, value_parser = rounds)]
    pub rounds: Option<u32>,

    /// Send a terminal bell on state transitions
    #[arg(long, overrides_with = "no_bell")]
    pub bell: bool,

    /// No terminal bell, even if the config file turns it on
    #[arg(long, overrides_with = "bell")]
    pub no_bell: bool,

    /// Send desktop notifications on state transitions
    #[arg(long, overrides_with = "no_notify")]
    pub notify: bool,

    /// No desktop notifications, even if the config file turns them on
    #[arg(long, overrides_with = "notify")]
    pub no_notify: bool,
}

fn parse_within(value: &str, max: u32) -> Result<u32, String> {
    let value = value.parse::<u32>().map_err(|err| err.to_string())?;
    config::check(value, max)
}

fn minutes(value: &str) -> Result<u32, String> {
    parse_within(value, config::MAX_MINUTES)
}

fn rounds(value: &str) -> Result<u32, String> {
    parse_within(value, config::MAX_ROUNDS)
}

/// A flag either way beats the file; with no flag the file decides.
fn resolve_switch(on: bool, off: bool, file: Option<bool>) -> bool {
    if off {
        false
    } else {
        on || file.unwrap_or(false)
    }
}

/// The clap command with the resolved config path in `--help`. Built at
/// runtime because the path depends on the environment.
pub fn command(config: Option<&Path>) -> clap::Command {
    let location = match config {
        Some(path) => path.display().to_string(),
        None => "none (set HOME, XDG_CONFIG_HOME or MIJA_CONFIG)".to_string(),
    };
    Args::command().after_help(format!("Config file: {location}"))
}

pub fn try_parse_with<I, T>(config: Option<&Path>, argv: I) -> Result<Args, clap::Error>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let matches = command(config).try_get_matches_from(argv)?;
    Args::from_arg_matches(&matches)
}

/// Parse the process arguments, exiting on `--help`, `--version` or bad input.
pub fn parse(config: Option<&Path>) -> Args {
    try_parse_with(config, std::env::args_os()).unwrap_or_else(|err| err.exit())
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

    pub fn bell_enabled(&self, file: &FileConfig) -> bool {
        resolve_switch(self.bell, self.no_bell, file.bell)
    }

    pub fn notify_enabled(&self, file: &FileConfig) -> bool {
        resolve_switch(self.notify, self.no_notify, file.notify)
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

    #[test]
    fn help_shows_the_config_path() {
        let help = command(Some(Path::new("/home/me/.config/mija/config.toml")))
            .render_help()
            .to_string();
        assert!(
            help.contains("Config file: /home/me/.config/mija/config.toml"),
            "{help}"
        );
    }

    #[test]
    fn help_admits_when_there_is_no_config_path() {
        let help = command(None).render_help().to_string();
        assert!(help.contains("Config file: none"), "{help}");
    }

    #[test]
    fn try_parse_with_reads_flags() {
        let args = try_parse_with(None, ["mija", "--work", "10"]).expect("valid flags");
        assert_eq!(args.work, Some(10));
    }

    fn alerts_on_in_file() -> FileConfig {
        FileConfig {
            bell: Some(true),
            notify: Some(true),
            ..FileConfig::default()
        }
    }

    #[test]
    fn no_flags_turn_off_alerts_the_file_turns_on() {
        let args = Args::parse_from(["mija", "--no-bell", "--no-notify"]);
        assert!(!args.bell_enabled(&alerts_on_in_file()));
        assert!(!args.notify_enabled(&alerts_on_in_file()));
    }

    #[test]
    fn no_bell_leaves_notify_alone() {
        let args = Args::parse_from(["mija", "--no-bell"]);
        assert!(!args.bell_enabled(&alerts_on_in_file()));
        assert!(args.notify_enabled(&alerts_on_in_file()));
    }

    #[test]
    fn the_last_of_bell_and_no_bell_wins() {
        let off = Args::parse_from(["mija", "--bell", "--no-bell"]);
        assert!(!off.bell_enabled(&no_file()));
        let on = Args::parse_from(["mija", "--no-bell", "--bell"]);
        assert!(on.bell_enabled(&alerts_on_in_file()));
    }

    #[test]
    fn the_last_of_notify_and_no_notify_wins() {
        let off = Args::parse_from(["mija", "--notify", "--no-notify"]);
        assert!(!off.notify_enabled(&no_file()));
        let on = Args::parse_from(["mija", "--no-notify", "--notify"]);
        assert!(on.notify_enabled(&alerts_on_in_file()));
    }

    #[test]
    fn zero_is_rejected_for_every_duration_and_rounds() {
        for flag in ["--work", "--short-break", "--long-break", "--rounds"] {
            let err = match Args::try_parse_from(["mija", flag, "0"]) {
                Ok(_) => panic!("{flag} 0 should be rejected"),
                Err(err) => err,
            };
            assert_eq!(
                err.kind(),
                clap::error::ErrorKind::ValueValidation,
                "{flag}"
            );
            assert!(
                err.to_string().contains("must be at least 1"),
                "{flag}: {err}"
            );
        }
    }

    #[test]
    fn one_is_the_smallest_accepted_value() {
        for flag in ["--work", "--short-break", "--long-break", "--rounds"] {
            assert!(
                Args::try_parse_from(["mija", flag, "1"]).is_ok(),
                "{flag} 1 should be accepted"
            );
        }
    }

    #[test]
    fn values_over_the_cap_are_rejected() {
        use crate::config::{MAX_MINUTES, MAX_ROUNDS};
        for (flag, max) in [
            ("--work", MAX_MINUTES),
            ("--short-break", MAX_MINUTES),
            ("--long-break", MAX_MINUTES),
            ("--rounds", MAX_ROUNDS),
        ] {
            let over = (max + 1).to_string();
            let err = match Args::try_parse_from(["mija", flag, over.as_str()]) {
                Ok(_) => panic!("{flag} {over} should be rejected"),
                Err(err) => err,
            };
            assert!(err.to_string().contains("must be at most"), "{flag}: {err}");
            let at_max = max.to_string();
            assert!(Args::try_parse_from(["mija", flag, at_max.as_str()]).is_ok());
        }
    }

    #[test]
    fn a_huge_value_is_rejected_rather_than_overflowing() {
        assert!(Args::try_parse_from(["mija", "--work", "100000000"]).is_err());
    }
}

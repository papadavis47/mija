use serde::Deserialize;
use std::path::{Path, PathBuf};

pub struct Config {
    pub work_duration_secs: u32,
    pub short_break_duration_secs: u32,
    pub long_break_duration_secs: u32,
    pub rounds_before_long_break: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            work_duration_secs: 25 * 60,
            short_break_duration_secs: 5 * 60,
            long_break_duration_secs: 15 * 60,
            rounds_before_long_break: 4,
        }
    }
}

/// Durations as they appear in `config.toml`, in minutes. Every field is
/// optional so an absent key can fall through to the built-in default rather
/// than overriding it with a zero.
#[derive(Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileConfig {
    pub work: Option<u32>,
    pub short_break: Option<u32>,
    pub long_break: Option<u32>,
    pub rounds: Option<u32>,
    pub bell: Option<bool>,
    pub notify: Option<bool>,
}

/// Parse config text. Kept separate from the filesystem so the parsing rules
/// can be tested without touching disk.
pub fn parse(contents: &str) -> Result<FileConfig, toml::de::Error> {
    toml::from_str(contents)
}

/// Resolve the config location from environment values. Takes them as
/// arguments rather than reading the environment so the precedence rules stay
/// testable — `std::env::set_var` is unsafe in edition 2024 and races the
/// parallel test harness.
pub fn config_path_from(
    override_path: Option<&str>,
    xdg_config_home: Option<&str>,
    home: Option<&str>,
) -> Option<PathBuf> {
    if let Some(path) = override_path {
        return Some(PathBuf::from(path));
    }
    if let Some(xdg) = xdg_config_home {
        return Some(Path::new(xdg).join("mija").join("config.toml"));
    }
    home.map(|home| {
        Path::new(home)
            .join(".config")
            .join("mija")
            .join("config.toml")
    })
}

/// The config location for this process.
pub fn config_path() -> Option<PathBuf> {
    let override_path = std::env::var("MIJA_CONFIG").ok();
    let xdg = std::env::var("XDG_CONFIG_HOME").ok();
    let home = std::env::var("HOME").ok();
    config_path_from(override_path.as_deref(), xdg.as_deref(), home.as_deref())
}

/// Read and parse the config at `path`. No path or no file means "use the
/// defaults"; a file that exists but cannot be read or parsed is an error, so a
/// typo is reported rather than silently ignored.
pub fn load_from_path(path: Option<&Path>) -> Result<FileConfig, String> {
    let Some(path) = path else {
        return Ok(FileConfig::default());
    };
    let contents = match std::fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(FileConfig::default());
        }
        Err(err) => return Err(format!("could not read {}: {err}", path.display())),
    };
    parse(&contents).map_err(|err| format!("could not parse {}: {err}", path.display()))
}

/// Load the config for this process.
pub fn load() -> Result<FileConfig, String> {
    load_from_path(config_path().as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_work_duration_is_25_minutes() {
        let config = Config::default();
        assert_eq!(config.work_duration_secs, 25 * 60);
    }

    #[test]
    fn default_short_break_is_5_minutes() {
        let config = Config::default();
        assert_eq!(config.short_break_duration_secs, 5 * 60);
    }

    #[test]
    fn default_long_break_is_15_minutes() {
        let config = Config::default();
        assert_eq!(config.long_break_duration_secs, 15 * 60);
    }

    #[test]
    fn default_rounds_before_long_break_is_4() {
        let config = Config::default();
        assert_eq!(config.rounds_before_long_break, 4);
    }

    #[test]
    fn parse_reads_every_key() {
        let file = parse(
            r#"
work = 30
short_break = 7
long_break = 20
rounds = 3
bell = true
notify = false
"#,
        )
        .expect("valid config");
        assert_eq!(file.work, Some(30));
        assert_eq!(file.short_break, Some(7));
        assert_eq!(file.long_break, Some(20));
        assert_eq!(file.rounds, Some(3));
        assert_eq!(file.bell, Some(true));
        assert_eq!(file.notify, Some(false));
    }

    #[test]
    fn parse_leaves_absent_keys_unset() {
        let file = parse("work = 30").expect("valid config");
        assert_eq!(file.work, Some(30));
        assert_eq!(file.short_break, None);
        assert_eq!(file.bell, None);
    }

    #[test]
    fn parse_accepts_an_empty_file() {
        let file = parse("").expect("empty is valid");
        assert_eq!(file.work, None);
        assert_eq!(file.rounds, None);
    }

    #[test]
    fn parse_rejects_an_unknown_key() {
        assert!(parse("wrok = 30").is_err());
    }

    #[test]
    fn parse_rejects_a_wrong_type() {
        assert!(parse("work = \"thirty\"").is_err());
    }

    #[test]
    fn config_path_prefers_the_override() {
        let path = config_path_from(Some("/tmp/custom.toml"), Some("/xdg"), Some("/home/me"));
        assert_eq!(path, Some(PathBuf::from("/tmp/custom.toml")));
    }

    #[test]
    fn config_path_uses_xdg_when_there_is_no_override() {
        let path = config_path_from(None, Some("/xdg"), Some("/home/me"));
        assert_eq!(path, Some(PathBuf::from("/xdg/mija/config.toml")));
    }

    #[test]
    fn config_path_falls_back_to_home() {
        let path = config_path_from(None, None, Some("/home/me"));
        assert_eq!(
            path,
            Some(PathBuf::from("/home/me/.config/mija/config.toml"))
        );
    }

    #[test]
    fn config_path_gives_up_without_any_env() {
        assert_eq!(config_path_from(None, None, None), None);
    }

    #[test]
    fn load_without_a_path_is_all_defaults() {
        let file = load_from_path(None).expect("no path is fine");
        assert_eq!(file.work, None);
    }

    #[test]
    fn load_of_a_missing_file_is_all_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("config.toml");
        let file = load_from_path(Some(&missing)).expect("missing file is fine");
        assert_eq!(file.work, None);
    }

    #[test]
    fn load_reads_a_real_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "work = 45\nbell = true\n").unwrap();
        let file = load_from_path(Some(&path)).expect("valid config");
        assert_eq!(file.work, Some(45));
        assert_eq!(file.bell, Some(true));
    }

    #[test]
    fn load_of_a_malformed_file_names_the_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "work = = 30").unwrap();
        let err = load_from_path(Some(&path)).expect_err("malformed config must fail");
        assert!(
            err.contains("config.toml"),
            "error should name the path: {err}"
        );
    }
}

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

/// Where the config lives, and whether the user chose that spot themselves via
/// `MIJA_CONFIG`. Only a default location is ever created for them.
#[derive(Debug, PartialEq, Eq)]
pub struct ConfigLocation {
    pub path: PathBuf,
    pub explicit: bool,
}

/// Resolve the config location from environment values. Takes them as
/// arguments rather than reading the environment so the precedence rules stay
/// testable — `std::env::set_var` is unsafe in edition 2024 and races the
/// parallel test harness.
pub fn config_path_from(
    override_path: Option<&str>,
    xdg_config_home: Option<&str>,
    home: Option<&str>,
) -> Option<ConfigLocation> {
    if let Some(path) = override_path {
        return Some(ConfigLocation {
            path: PathBuf::from(path),
            explicit: true,
        });
    }
    let path = if let Some(xdg) = xdg_config_home {
        Path::new(xdg).join("mija").join("config.toml")
    } else {
        Path::new(home?)
            .join(".config")
            .join("mija")
            .join("config.toml")
    };
    Some(ConfigLocation {
        path,
        explicit: false,
    })
}

/// The config location for this process.
pub fn config_path() -> Option<ConfigLocation> {
    let override_path = std::env::var("MIJA_CONFIG").ok();
    let xdg = std::env::var("XDG_CONFIG_HOME").ok();
    let home = std::env::var("HOME").ok();
    config_path_from(override_path.as_deref(), xdg.as_deref(), home.as_deref())
}

/// Written on first run. Every key is commented out so the built-in defaults
/// still apply — and keep tracking the code if a default ever changes.
pub const TEMPLATE: &str = "\
# Mija configuration. Uncomment a line to override its default.
# Command-line flags take precedence over this file.

# work = 25          # work period, minutes
# short_break = 5    # short break, minutes
# long_break = 15    # long break, minutes
# rounds = 4         # work periods before a long break
# bell = false       # terminal bell on transitions
# notify = false     # desktop notifications on transitions
";

/// Write the template to `path`, creating parent directories. Returns
/// `Ok(false)` without touching anything if the file already exists.
pub fn create_default(path: &Path) -> std::io::Result<bool> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
    {
        Ok(mut file) => {
            std::io::Write::write_all(&mut file, TEMPLATE.as_bytes())?;
            Ok(true)
        }
        Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => Ok(false),
        Err(err) => Err(err),
    }
}

/// Something worth telling the user before the TUI starts.
#[derive(Debug, PartialEq, Eq)]
pub enum Notice {
    Created(PathBuf),
    MissingOverride(PathBuf),
}

impl std::fmt::Display for Notice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Notice::Created(path) => write!(f, "created config at {}", path.display()),
            Notice::MissingOverride(path) => write!(
                f,
                "MIJA_CONFIG points to {}, which does not exist; using defaults",
                path.display()
            ),
        }
    }
}

/// Make sure a default config exists before loading. A failed write is
/// ignored — a missing config must never stop the timer from starting.
pub fn prepare(location: Option<&ConfigLocation>) -> Option<Notice> {
    let location = location?;
    if location.path.exists() {
        return None;
    }
    if location.explicit {
        return Some(Notice::MissingOverride(location.path.clone()));
    }
    match create_default(&location.path) {
        Ok(true) => Some(Notice::Created(location.path.clone())),
        _ => None,
    }
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
pub fn load(location: Option<&ConfigLocation>) -> Result<FileConfig, String> {
    load_from_path(location.map(|location| location.path.as_path()))
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
        let location = config_path_from(Some("/tmp/custom.toml"), Some("/xdg"), Some("/home/me"))
            .expect("override gives a path");
        assert_eq!(location.path, PathBuf::from("/tmp/custom.toml"));
        assert!(location.explicit);
    }

    #[test]
    fn config_path_uses_xdg_when_there_is_no_override() {
        let location = config_path_from(None, Some("/xdg"), Some("/home/me")).expect("xdg path");
        assert_eq!(location.path, PathBuf::from("/xdg/mija/config.toml"));
        assert!(!location.explicit);
    }

    #[test]
    fn config_path_falls_back_to_home() {
        let location = config_path_from(None, None, Some("/home/me")).expect("home path");
        assert_eq!(
            location.path,
            PathBuf::from("/home/me/.config/mija/config.toml")
        );
        assert!(!location.explicit);
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

    #[test]
    fn template_parses_to_all_defaults() {
        let file = parse(TEMPLATE).expect("template must be valid toml");
        assert_eq!(file, FileConfig::default());
    }

    #[test]
    fn template_documents_every_key() {
        for key in [
            "work",
            "short_break",
            "long_break",
            "rounds",
            "bell",
            "notify",
        ] {
            assert!(
                TEMPLATE.contains(&format!("# {key} = ")),
                "template should document {key}"
            );
        }
    }

    #[test]
    fn create_default_writes_the_template_and_parent_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mija").join("config.toml");
        assert!(create_default(&path).expect("write succeeds"));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), TEMPLATE);
    }

    #[test]
    fn create_default_never_overwrites_an_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "work = 45\n").unwrap();
        assert!(!create_default(&path).expect("existing file is fine"));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "work = 45\n");
    }

    fn location(path: PathBuf, explicit: bool) -> ConfigLocation {
        ConfigLocation { path, explicit }
    }

    #[test]
    fn prepare_creates_a_missing_default_config() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mija").join("config.toml");
        let notice = prepare(Some(&location(path.clone(), false)));
        assert_eq!(notice, Some(Notice::Created(path.clone())));
        assert!(path.exists());
    }

    #[test]
    fn prepare_is_silent_when_the_config_exists() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "").unwrap();
        assert_eq!(prepare(Some(&location(path.clone(), false))), None);
        assert_eq!(prepare(Some(&location(path, true))), None);
    }

    #[test]
    fn prepare_reports_a_missing_override_without_creating_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("custom.toml");
        let notice = prepare(Some(&location(path.clone(), true)));
        assert_eq!(notice, Some(Notice::MissingOverride(path.clone())));
        assert!(!path.exists());
    }

    #[test]
    fn prepare_without_a_location_does_nothing() {
        assert_eq!(prepare(None), None);
    }

    #[test]
    fn prepare_swallows_a_failed_write() {
        let dir = tempfile::tempdir().unwrap();
        // A file where the parent directory should be makes the write fail.
        let blocker = dir.path().join("mija");
        std::fs::write(&blocker, "").unwrap();
        let path = blocker.join("config.toml");
        assert_eq!(prepare(Some(&location(path, false))), None);
    }

    #[test]
    fn notices_name_the_path() {
        let path = PathBuf::from("/x/config.toml");
        let created = Notice::Created(path.clone()).to_string();
        assert!(
            created.contains("created config at /x/config.toml"),
            "{created}"
        );
        let missing = Notice::MissingOverride(path).to_string();
        assert!(missing.contains("/x/config.toml"), "{missing}");
        assert!(missing.contains("MIJA_CONFIG"), "{missing}");
        assert!(missing.contains("defaults"), "{missing}");
    }
}

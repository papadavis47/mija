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
}

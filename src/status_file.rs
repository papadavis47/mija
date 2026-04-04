use std::fs;
use std::io;
use std::path::PathBuf;

pub struct StatusFile {
    path: PathBuf,
}

impl StatusFile {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn write(&self, status: &str) -> io::Result<()> {
        fs::write(&self.path, status)
    }

    pub fn cleanup(&self) {
        let _ = fs::remove_file(&self.path);
    }
}

impl Drop for StatusFile {
    fn drop(&mut self) {
        self.cleanup();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn write_creates_file_with_status() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("pomodoro_status");
        let writer = StatusFile::new(path.clone());
        writer.write("🍅 work 24:59").unwrap();
        let contents = fs::read_to_string(&path).unwrap();
        assert_eq!(contents, "🍅 work 24:59");
    }

    #[test]
    fn write_overwrites_previous_content() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("pomodoro_status");
        let writer = StatusFile::new(path.clone());
        writer.write("🍅 work 24:59").unwrap();
        writer.write("☕ short break 04:59").unwrap();
        let contents = fs::read_to_string(&path).unwrap();
        assert_eq!(contents, "☕ short break 04:59");
    }

    #[test]
    fn cleanup_removes_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("pomodoro_status");
        let writer = StatusFile::new(path.clone());
        writer.write("🍅 work 24:59").unwrap();
        writer.cleanup();
        assert!(!path.exists());
    }

    #[test]
    fn cleanup_is_noop_if_file_missing() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("pomodoro_status");
        let writer = StatusFile::new(path);
        writer.cleanup(); // should not panic
    }

    #[test]
    fn drop_removes_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("pomodoro_status");
        {
            let writer = StatusFile::new(path.clone());
            writer.write("🍅 work 24:59").unwrap();
        }
        assert!(!path.exists());
    }
}

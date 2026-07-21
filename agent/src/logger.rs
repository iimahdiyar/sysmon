use common::error::MonitorResult;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct FileLogger {
    path: String,
}

impl FileLogger {
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }

    pub fn log(&self, message: &str) -> MonitorResult<()> {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(Path::new(&self.path))?;

        writeln!(file, "[{ts}] {message}")?;
        Ok(())
    }
}

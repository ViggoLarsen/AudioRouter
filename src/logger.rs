use anyhow::Result;
use log::{Level, LevelFilter, Log, Metadata, Record};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct FileLogger {
    file: Mutex<File>,
}

impl FileLogger {
    pub fn new(log_path: PathBuf) -> Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&log_path)?;

        Ok(FileLogger {
            file: Mutex::new(file),
        })
    }

    pub fn init(log_path: PathBuf, level: &str) -> Result<()> {
        let logger = Box::new(FileLogger::new(log_path)?);

        let level_filter = match level.to_lowercase().as_str() {
            "trace" => LevelFilter::Trace,
            "debug" => LevelFilter::Debug,
            "info" => LevelFilter::Info,
            "warn" => LevelFilter::Warn,
            "error" => LevelFilter::Error,
            _ => LevelFilter::Info,
        };

        log::set_boxed_logger(logger)
            .map(|()| log::set_max_level(level_filter))
            .map_err(|e| anyhow::anyhow!("Failed to initialize logger: {}", e))?;

        Ok(())
    }
}

impl Log for FileLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Trace
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");

            let log_message = format!(
                "[{}] {} - {}: {}\n",
                timestamp,
                record.level(),
                record.target(),
                record.args()
            );

            if let Ok(mut file) = self.file.lock() {
                let _ = file.write_all(log_message.as_bytes());
                let _ = file.flush();
            }

            println!("{}", log_message.trim_end());
        }
    }

    fn flush(&self) {
        if let Ok(mut file) = self.file.lock() {
            let _ = file.flush();
        }
    }
}

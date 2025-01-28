use std::fs::{self, File};

use env_logger::Builder;
use log::LevelFilter;

pub struct Logger {}

impl Logger {
    pub fn new() {
        // Create a log file.
        let mut log_path = dirs::data_dir().expect("Failed to get data directory");
        log_path.push("FastForward");
        fs::create_dir_all(&log_path).expect("Failed to create log directory");
        log_path.push("app.log");
        let log_file = File::create(log_path).expect("Failed to create log file");

        // Initialize the logger.
        Builder::new()
            .filter(None, LevelFilter::Info)
            .write_style(env_logger::WriteStyle::Always)
            .target(env_logger::Target::Pipe(Box::new(log_file)))
            .init();
    }
}

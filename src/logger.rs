use std::{fs::{self, File}, panic, thread};

use env_logger::Builder;
use log::{error, LevelFilter};

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

        // Set up panic hook to log panics.
        panic::set_hook(Box::new(move |info| {
            let thread = thread::current();
            let thread = thread.name().unwrap_or("<unnamed>");

            let msg = match info.payload().downcast_ref::<&'static str>() {
                Some(s) => *s,
                None => match info.payload().downcast_ref::<String>() {
                    Some(s) => &**s,
                    None => "Box<Any>",
                },
            };

            match info.location() {
                Some(location) => {
                    error!(
                        target: "panic", "thread '{}' panicked at '{}': {}:{}",
                        thread,
                        msg,
                        location.file(),
                        location.line(),
                    );
                }
                None => error!(
                    target: "panic",
                    "thread '{}' panicked at '{}'",
                    thread,
                    msg,
                ),
            }
        }));
    }
}

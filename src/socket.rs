use std::fs;
use std::path::PathBuf;
use std::process::Stdio;
use gpui::AppContext;
use tokio::io::{AsyncReadExt, AsyncBufReadExt, BufReader};
use tokio::net::UnixStream;
use tokio::sync::watch;
use serde_json;
use tokio::process::{Child, Command};
use anyhow::Context;
use std::io::prelude::*;

use crate::applications::{App, Applications, IndexType};

const SWIFT_BINARY: &[u8] = include_bytes!("../.build/fast-forward-monitor");

pub struct Socket {
    stream: UnixStream,
    swift_monitor: Child,
    tx: watch::Sender<SocketEvent>,
}

pub enum SocketEvent {
    List(Vec<App>),
    Launch(App),
    Close(App),
    Activate(App),
    None
}

impl Socket {
    pub fn new(cx: &mut AppContext) {
        let (tx, mut rx) = watch::channel(SocketEvent::None);
        cx.spawn(|cx| async move {
            while rx.changed().await.is_ok() {
                match *rx.borrow() {
                    SocketEvent::List(ref list) => {
                        let _ = cx.update(|cx| {
                            Applications::update_list(cx, list.clone());
                        });
                    }
                    SocketEvent::Launch(ref app) | SocketEvent::Activate(ref app) => {
                        let _ = cx.update(|cx| {
                            Applications::update_list_entry(cx, Some(app), Some(IndexType::Start));
                        });
                    }
                    SocketEvent::Close(ref app) => {
                        let _ = cx.update(|cx| {
                            Applications::update_list_entry(cx, Some(app), None);
                        });
                    },
                    SocketEvent::None => {
                        println!("Unknown event received")
                    },
                }
            }
        }).detach();

        Self::listen_for_unix_socket_events(tx);
    }

    fn listen_for_unix_socket_events(tx: watch::Sender<SocketEvent>) {
        tokio::spawn(async move {
            let socket_path = "/tmp/swift_monitor.sock";
            let mut swift_monitor = match Self::run_swift_monitor().await {
                Ok(process) => process,
                Err(e) => {
                    panic!("Failed to start Swift monitor: {}", e);
                }
            };

            Self::wait_for_message(&mut swift_monitor, "Socket bound successfully").await.ok();
            let stream = match UnixStream::connect(socket_path).await {
                Ok(stream) => stream,
                Err(e) => {
                    panic!("Failed to connect to socket: {}", e);
                }
            };

            let mut buffer = vec![0u8; 4096];
            let mut connection = Socket {
                stream,
                swift_monitor,
                tx,
            };

            loop {
                tokio::select! {
                    result = connection.stream.read(&mut buffer) => {
                        match result {
                            Ok(0) => {
                                println!("No more data received.");
                                break;
                            }
                            Ok(bytes_read) => {
                                let message = Self::parse_message(bytes_read, &buffer);
                                connection.handle_message(message);
                            }
                            Err(e) => {
                                eprintln!("Error while reading data from the socket: {}", e);
                                break;
                            }
                        }
                    }
                }
            }
        });
    }

    // TODO: Refactor with protobuf
    fn handle_message(&mut self, message: serde_json::Value) {
        if let Some(message_type) = message["type"].as_str() {
            match message_type {
                "list" => {
                    if let Some(list) = message["list"].as_array() {
                        let mut apps = Vec::new();
                        for app in list {
                            if let Some(name) = app["name"].as_str() {
                                let pid = app["pid"].as_i64().unwrap_or(0) as isize;
                                let active = app["active"].as_bool().unwrap_or(false);
                                let icon = app["icon"].as_str().unwrap_or("").into();
                                apps.push(App {
                                    name: name.into(),
                                    pid,
                                    active,
                                    icon,
                                });
                            }
                        }
                        let event = SocketEvent::List(apps);
                        self.tx.send(event).expect("Failed to send event");
                    } else {
                        eprintln!("List field is missing or not an array");
                    }
                },
                "launch" | "close" | "activate" => {
                    if let Some(app_info) = message["app"].as_object() {
                        let name = app_info["name"].as_str().unwrap_or("").to_string();
                        let pid = app_info["pid"].as_i64().unwrap_or(0) as isize;
                        let active = app_info["active"].as_bool().unwrap_or(false);
                        let icon = app_info["icon"].as_str().unwrap_or("").into();
                        let app = App {
                            name,
                            pid,
                            active,
                            icon,
                        };
                        let event = match message_type {
                            "launch" => SocketEvent::Launch(app),
                            "close" => SocketEvent::Close(app),
                            "activate" => SocketEvent::Activate(app),
                            _ => SocketEvent::None,
                        };
                        self.tx.send(event).expect("Failed to send event");
                    } else {
                        eprintln!("App field is missing or not an object");
                    }
                },
                _ => {}
            }
        } else {
            eprintln!("Message does not contain a type field");
        }
    }

    fn parse_message(bytes_read: usize, buffer: &Vec<u8>) -> serde_json::Value {
        let message = String::from_utf8_lossy(&buffer[..bytes_read]);
        let parsed_message: serde_json::Value = serde_json::from_str(&message).unwrap_or_else(|_| {
            eprintln!("Failed to parse message as JSON");
            serde_json::Value::Null
        });

        parsed_message
    }

    async fn wait_for_message(child: &mut Child, message: &str) -> std::io::Result<()> {
        if let Some(stdout) = child.stdout.take() {
            let mut reader = BufReader::new(stdout).lines();

            while let Some(line) = reader.next_line().await? {
                if line.contains(message) {
                    return Ok(());
                }
            }
        }

        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Swift process terminated or message not found",
        ))
    }

    fn get_swif_binary_path() -> anyhow::Result<PathBuf> {
        let app_support_dir = dirs::config_dir()
            .context("Failed to get application config directory")?
            .join("FastForward");

        Ok(app_support_dir.join("fast-forward-monitor"))
    }

    fn save_swift_binary() -> anyhow::Result<PathBuf> {
        let binary_path = Self::get_swif_binary_path()?;
        if binary_path.exists() {
            fs::remove_file(&binary_path)?;
        }

        let mut file = fs::File::create(&binary_path)?;
        file.write_all(SWIFT_BINARY)?;

        // Make the temporary file executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&binary_path)
                .expect("Failed to get file metadata")
                .permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&binary_path, perms).expect("Failed to set file permissions");
        }

        Ok(binary_path)
    }

    async fn run_swift_monitor() -> std::io::Result<Child> {
        let binary_path = Self::save_swift_binary();
        match &binary_path {
            Ok(path) => {
                let process =Command::new(path)
                    .env_remove("DYLD_LIBRARY_PATH")
                    .stdout(Stdio::piped())
                    .spawn()?;

                Ok(process)
            },
            Err(_) => {
                Err(std::io::Error::new(std::io::ErrorKind::Other, "Failed to save Swift binary"))
            },
        }
    }
}

impl Drop for Socket {
    fn drop(&mut self) {
        let _ = self.swift_monitor.kill();
    }
}

use std::fs;
use std::path::PathBuf;
use std::process::Stdio;
use gpui::AppContext;
use tokio::io::{AsyncReadExt, AsyncBufReadExt, BufReader};
use tokio::net::UnixStream;
use tokio::sync::watch;
use tokio::process::{Child, Command};
use anyhow::Context;
use std::io::prelude::*;
use prost::Message;

use crate::applications::{Applications, IndexType};
use crate::socket_message::{SocketMessage, socket_message::Event};

const SWIFT_BINARY: &[u8] = include_bytes!("../swift-lib/.build/release/swift-lib");

pub struct Socket {
    stream: UnixStream,
    swift_monitor: Child,
}

impl Socket {
    pub fn new(cx: &mut AppContext) {
        let (tx, mut rx) = watch::channel(SocketMessage::default());
        cx.spawn(|cx| async move {
            while rx.changed().await.is_ok() {
                if let Some(event) = &rx.borrow().event {
                    match event {
                        Event::List(ref event) => {
                            let _ = cx.update(|cx| {
                                Applications::update_list(cx, event.apps.clone());
                            });
                        }
                        Event::Launch(ref event) => {
                            let _ = cx.update(|cx| {
                                Applications::update_list_entry(cx, event.app.as_ref(), Some(IndexType::Start));
                            });
                        }
                        Event::Close(ref event) => {
                            let _ = cx.update(|cx| {
                                Applications::update_list_entry(cx, event.app.as_ref(), None);
                            });
                        }
                        Event::Activate(ref event) => {
                            let _ = cx.update(|cx| {
                                Applications::update_list_entry(cx, event.app.as_ref(), Some(IndexType::Start));
                            });
                        }
                    };
                }
            }
        }).detach();

        Self::listen_for_unix_socket_events(tx);
    }

    fn listen_for_unix_socket_events(tx: watch::Sender<SocketMessage>) {
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
                                let message = SocketMessage::decode(&buffer[..bytes_read]).unwrap();
                                println!("Received message: {:?}", message);
                                tx.send(message).expect("Failed to send event");
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

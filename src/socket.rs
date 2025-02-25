use std::fs;
use std::path::PathBuf;
use std::process::Stdio;
use gpui::App;
use tokio::io::{AsyncReadExt, AsyncBufReadExt, BufReader};
use tokio::net::UnixStream;
use tokio::sync::mpsc::UnboundedSender;
use tokio::process::{Child, Command};
use tokio::time::{sleep, Duration};
use anyhow::Context;
use std::io::prelude::*;
use prost::Message;
use log::error;

use crate::commander::{Commander, EventType};
use crate::socket_message::SocketMessage;

const SWIFT_BINARY: &[u8] = include_bytes!("../swift-lib/.build/release/swift-lib");
const RECONNECT_DELAY: Duration = Duration::from_secs(5);

pub struct Socket {
    stream: UnixStream,
    swift_monitor: Child,
}

impl Socket {
    pub fn new(cx: &mut App) {
        let tx = cx.global::<Commander>().tx.clone();
        Self::listen_for_unix_socket_events(tx);
    }

    fn listen_for_unix_socket_events(tx: UnboundedSender<EventType>) {
        tokio::spawn(async move {
            loop {
                match Self::handle_connection(&tx).await {
                    Ok(()) => (),
                    Err(e) => {
                        error!("Connection error: {e}");
                        sleep(RECONNECT_DELAY).await;
                    }
                }
            }
        });
    }

    async fn handle_connection(tx: &UnboundedSender<EventType>) -> std::io::Result<()> {
        let (stream, swift_monitor) = Self::establish_connection().await?;
        let mut connection = Socket { stream, swift_monitor };

        loop {
            if let Err(e) = Self::handle_message(&mut connection, tx).await {
                error!("Message handling error: {e}");
                return Err(e);
            }
        }
    }

    async fn establish_connection() -> std::io::Result<(UnixStream, Child)> {
        let socket_path = "/tmp/swift_monitor.sock";
        let mut swift_monitor = Self::run_swift_monitor()?;

        // TODO: Check if the socket file exists instead of waiting for a message
        Self::wait_for_message(&mut swift_monitor, "Socket bound successfully").await?;
        let stream = UnixStream::connect(socket_path).await?;

        Ok((stream, swift_monitor))
    }

    async fn handle_message(connection: &mut Socket, tx: &UnboundedSender<EventType>) -> std::io::Result<()> {
        let message = Self::read_message(connection).await?;
        Self::process_message(message, tx)?;
        Ok(())
    }

    async fn read_message(connection: &mut Socket) -> std::io::Result<SocketMessage> {
        let mut length_buffer = [0u8; 4];
        connection.stream.read_exact(&mut length_buffer).await?;

        let message_length = u32::from_be_bytes(length_buffer) as usize;
        let mut message_buffer = vec![0u8; message_length];
        connection.stream.read_exact(&mut message_buffer).await?;

        SocketMessage::decode(&*message_buffer)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    fn process_message(message: SocketMessage, tx: &UnboundedSender<EventType>) -> std::io::Result<()> {
        match message.event {
            Some(event) => {
                tx.send(EventType::SocketEvent(event))
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
                Ok(())
            }
            None => {
                Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Message missing event"))
            }
        }
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

    fn run_swift_monitor() -> std::io::Result<Child> {
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
        std::mem::drop(self.swift_monitor.kill());
    }
}

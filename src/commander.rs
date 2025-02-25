use gpui::{App, AsyncApp, Global, Result};
use log::{info, error};
use tokio::sync::mpsc::{self, UnboundedSender};
use tokio::time::Duration;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::{applications::IndexType, socket_message::socket_message::Event as SocketEvent};
use crate::{applications::{ActionType, Applications}, config::Config, window::Window};

static ESCAPE_PRESSED: AtomicBool = AtomicBool::new(false);
static SPACE_PRESSED: AtomicBool = AtomicBool::new(false);

pub struct Commander {
    pub tx: UnboundedSender<EventType>,
}

#[allow(clippy::enum_variant_names)]
pub enum EventType {
    HotkeyEvent(HotkeyEvent),
    TrayEvent(TrayEvent),
    SocketEvent(SocketEvent),
}

#[derive(Debug, Clone, Copy)]
pub enum HotkeyEvent {
    ShowWindow,
    HideWindow,
    HideApplication,
    QuitApplication,
}

#[derive(Debug, Clone, Copy)]
pub enum TrayEvent {
    Settings,
    Logs,
    About,
    Quit,
}

impl Commander {
    pub fn new(cx: &mut App) {
        let (tx, mut rx) = mpsc::unbounded_channel();
        cx.spawn(|cx| async move {
            loop {
                while let Ok(event) = rx.try_recv() {
                    if let Err(e) = handle_event(&cx, event) {
                        error!("Failed to handle event: {:?}", e);
                    }
                }
                cx.background_executor()
                    .timer(Duration::from_millis(50))
                    .await;
            }
        }).detach();

        cx.set_global::<Commander>(Self { tx });
    }
}

fn handle_event(cx: &AsyncApp, event: EventType) -> Result<(), Box<dyn std::error::Error>> {
    match event {
        EventType::HotkeyEvent(event) => handle_hotkey_event(cx, event),
        EventType::TrayEvent(event) => handle_tray_event(cx, event),
        EventType::SocketEvent(event) => handle_socket_event(cx, event),
    }
}

fn handle_hotkey_event(cx: &AsyncApp, event: HotkeyEvent) -> Result<(), Box<dyn std::error::Error>> {
    match event {
        HotkeyEvent::ShowWindow => {
            info!("Received event: HotkeyEvent::ShowWindow");
            cx.update(Window::show)?;
        }
        HotkeyEvent::HideWindow => {
            info!("Received event: HotkeyEvent::HideWindow");
            cx.update(|cx| {
                if !SPACE_PRESSED.load(Ordering::SeqCst) && !ESCAPE_PRESSED.load(Ordering::SeqCst) {
                    Applications::execute_action(cx, ActionType::Activate);
                }
                Window::hide(cx);
            })?;
        }
        HotkeyEvent::HideApplication => {
            info!("Received event: HotkeyEvent::HideApplication");
            cx.update(|cx| Applications::execute_action(cx, ActionType::Hide))?;
        }
        HotkeyEvent::QuitApplication => {
            info!("Received event: HotkeyEvent::QuitApplication");
            cx.update(|cx| Applications::execute_action(cx, ActionType::Quit))?;
        }
    }
    Ok(())
}

fn handle_tray_event(cx: &AsyncApp, event: TrayEvent) -> Result<(), Box<dyn std::error::Error>> {
    match event {
        TrayEvent::Settings => {
            info!("Received event: TrayEvent::Settings");
            let config_path = Config::config_path()?;
            std::process::Command::new("open")
                .arg("-a")
                .arg("TextEdit")
                .arg(&config_path)
                .spawn()?;
        }
        TrayEvent::Logs => {
            info!("Received event: TrayEvent::Logs");
            std::process::Command::new("open")
                .arg("-a")
                .arg("Terminal")
                .arg("tail -f -n100 ~/Library/Application\\ Support/FastForward/app.log")
                .spawn()?;
        }
        TrayEvent::About => {
            info!("Received event: TrayEvent::About");
            cx.update(|cx| cx.open_url("https://github.com/gaauwe/fast-forward"))?;
        }
        TrayEvent::Quit => {
            info!("Received event: TrayEvent::Quit");
            cx.update(|cx| cx.quit())?;
        }
    }
    Ok(())
}

fn handle_socket_event(cx: &AsyncApp, event: SocketEvent) -> Result<(), Box<dyn std::error::Error>> {
    match event {
        SocketEvent::List(event) => {
            info!("Received event: SocketEvent::List");
            cx.update(|cx| Applications::update_list(cx, event.apps.clone()))?;
        }
        SocketEvent::Launch(event) => {
            info!("Received event: SocketEvent::Launch");
            cx.update(|cx| Applications::update_list_entry(cx, event.app.as_ref(), Some(IndexType::Start)))?;
        }
        SocketEvent::Close(event) => {
            info!("Received event: SocketEvent::Close");
            cx.update(|cx| Applications::update_list_entry(cx, event.app.as_ref(), None))?;
        }
        SocketEvent::Activate(event) => {
            info!("Received event: SocketEvent::Activate");
            cx.update(|cx| Applications::update_list_entry(cx, event.app.as_ref(), Some(IndexType::Start)))?;
        }
    }
    Ok(())
}

impl Global for Commander {}

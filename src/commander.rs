use gpui::*;
use log::{info, error};
use tokio::sync::watch;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::{applications::IndexType, socket_message::socket_message::Event as SocketEvent};
use crate::{applications::{ActionType, Applications}, config::Config, window::Window};

static ESCAPE_PRESSED: AtomicBool = AtomicBool::new(false);
static SPACE_PRESSED: AtomicBool = AtomicBool::new(false);

pub struct Commander {
    pub tx: watch::Sender<EventType>,
}

pub enum EventType {
    HotkeyEvent(HotkeyEvent),
    TrayEvent(TrayEvent),
    SocketEvent(SocketEvent),
    None
}

pub enum HotkeyEvent {
    ShowWindow,
    HideWindow,
    HideApplication,
    QuitApplication,
}

pub enum TrayEvent {
    Settings,
    About,
    Quit,
}

impl Commander {
    pub fn new(cx: &mut AppContext) {
        let (tx, mut rx) = watch::channel(EventType::None);
        cx.spawn(|cx| async move {
            loop {
                match *rx.borrow_and_update() {
                    EventType::HotkeyEvent(ref event) => {
                        match event {
                            HotkeyEvent::ShowWindow => {
                                info!("Received event: HotkeyEvent::ShowWindow");
                                if let Err(e) = cx.update(|cx| {
                                    Window::show(cx);
                                }) {
                                    error!("Failed to show window: {:?}", e);
                                }
                            }
                            HotkeyEvent::HideWindow => {
                                info!("Received event: HotkeyEvent::HideWindow");
                                if let Err(e) = cx.update(|cx| {
                                    if !SPACE_PRESSED.load(Ordering::SeqCst) && !ESCAPE_PRESSED.load(Ordering::SeqCst)  {
                                        Applications::execute_action(cx, ActionType::Activate)
                                    }

                                    Window::hide(cx);
                                }) {
                                    error!("Failed to hide window: {:?}", e);
                                }
                            }
                            HotkeyEvent::HideApplication => {
                                info!("Received event: HotkeyEvent::HideApplication");
                                if let Err(e) = cx.update(|cx| {
                                    Applications::execute_action(cx, ActionType::Hide)
                                }) {
                                    error!("Failed to hide application: {:?}", e);
                                }
                            }
                            HotkeyEvent::QuitApplication => {
                                info!("Received event: HotkeyEvent::QuitApplication");
                                if let Err(e) = cx.update(|cx| {
                                    Applications::execute_action(cx, ActionType::Quit)
                                }) {
                                    error!("Failed to quit application: {:?}", e);
                                }
                            }
                        }
                    }
                    EventType::SocketEvent(ref event) => {
                        match event {
                            SocketEvent::List(ref event) => {
                                info!("Received event: SocketEvent::List");
                                if let Err(e) = cx.update(|cx| {
                                    Applications::update_list(cx, event.apps.clone());
                                }) {
                                    error!("Failed to update list: {:?}", e);
                                }
                            }
                            SocketEvent::Launch(ref event) => {
                                info!("Received event: SocketEvent::Launch");
                                if let Err(e) = cx.update(|cx| {
                                    Applications::update_list_entry(cx, event.app.as_ref(), Some(IndexType::Start));
                                }) {
                                    error!("Failed to update list entry on launch: {:?}", e);
                                }
                            }
                            SocketEvent::Close(ref event) => {
                                info!("Received event: SocketEvent::Close");
                                if let Err(e) = cx.update(|cx| {
                                    Applications::update_list_entry(cx, event.app.as_ref(), None);
                                }) {
                                    error!("Failed to update list entry on close: {:?}", e);
                                }
                            }
                            SocketEvent::Activate(ref event) => {
                                info!("Received event: SocketEvent::Activate");
                                if let Err(e) = cx.update(|cx| {
                                    Applications::update_list_entry(cx, event.app.as_ref(), Some(IndexType::Start));
                                }) {
                                    error!("Failed to update list entry on activate: {:?}", e);
                                }
                            }
                        }
                    }
                    EventType::TrayEvent(ref event) => {
                        match event {
                            TrayEvent::Settings => {
                                info!("Received event: Event::Settings");
                                let config_path = Config::config_path().unwrap();
                                if let Err(e) = std::process::Command::new("open")
                                    .arg("-a")
                                    .arg("TextEdit")
                                    .arg(&config_path)
                                    .spawn()
                                {
                                    error!("Failed to open settings: {:?}", e);
                                }
                            }
                            TrayEvent::About => {
                                info!("Received event: Event::About");
                                if let Err(e) = cx.update(|cx| {
                                    cx.open_url("https://github.com/gaauwe/fast-forward")
                                }) {
                                    error!("Failed to open about URL: {:?}", e);
                                }
                            },
                            TrayEvent::Quit => {
                                info!("Received event: Event::Quit");
                                if let Err(e) = cx.update(|cx| {
                                    cx.quit()
                                }) {
                                    error!("Failed to quit application: {:?}", e);
                                }
                            }
                        }
                    }
                    EventType::None => {}
                }

                if let Err(e) = rx.changed().await {
                    error!("Receiving channel failed: {:?}", e);
                    break;
                }
            }

            info!("Event loop terminated");
        }).detach();

        cx.set_global::<Commander>(Self {
            tx,
        });
    }
}

impl Global for Commander {}

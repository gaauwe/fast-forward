mod events;
mod handlers;

use gpui::{App, Global};
use log::error;
use tokio::sync::mpsc::{self, UnboundedSender};
use tokio::time::Duration;

pub use events::{EventType, HotkeyEvent, TrayEvent};
use handlers::handle_event;

pub struct Commander {
    pub tx: UnboundedSender<EventType>,
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

impl Global for Commander {}

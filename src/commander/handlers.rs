use std::sync::atomic::{AtomicBool, Ordering};
use gpui::{AsyncApp, Result};

use crate::{
    applications::{ActionType, Applications, IndexType},
    config::Config,
    window::Window,
};
use super::events::{EventType, HotkeyEvent, TrayEvent};
use crate::socket_message::socket_message::Event as SocketEvent;

static ESCAPE_PRESSED: AtomicBool = AtomicBool::new(false);
static SPACE_PRESSED: AtomicBool = AtomicBool::new(false);

pub(super) fn handle_event(cx: &AsyncApp, event: EventType) -> Result<(), Box<dyn std::error::Error>> {
    match event {
        EventType::HotkeyEvent(event) => handle_hotkey_event(cx, event),
        EventType::TrayEvent(event) => handle_tray_event(cx, event),
        EventType::SocketEvent(event) => handle_socket_event(cx, event),
    }
}

fn handle_hotkey_event(cx: &AsyncApp, event: HotkeyEvent) -> Result<(), Box<dyn std::error::Error>> {
    match event {
        HotkeyEvent::ShowWindow(offset) => {
            cx.update(|cx| {
                    Window::show(cx, offset)
            })?;
        }
        HotkeyEvent::HideWindow => {
            cx.update(|cx| {
                if !SPACE_PRESSED.load(Ordering::SeqCst) && !ESCAPE_PRESSED.load(Ordering::SeqCst) {
                    Applications::execute_action(cx, ActionType::Activate);
                }
                Window::hide(cx);
            })?;
        }
        HotkeyEvent::HideApplication => {
            cx.update(|cx| Applications::execute_action(cx, ActionType::Hide))?;
        }
        HotkeyEvent::QuitApplication => {
            cx.update(|cx| Applications::execute_action(cx, ActionType::Quit))?;
        }
    }
    Ok(())
}

fn handle_tray_event(cx: &AsyncApp, event: TrayEvent) -> Result<(), Box<dyn std::error::Error>> {
    match event {
        TrayEvent::Settings => {
            std::process::Command::new("open")
                .arg("-a")
                .arg("TextEdit")
                .arg(Config::config_path()?)
                .spawn()?;
        },
        TrayEvent::About => {
            cx.update(|cx| cx.open_url("https://github.com/gaauwe/fast-forward"))?;
        }
        TrayEvent::Quit => {
            cx.update(|cx| cx.quit())?;
        }
    }
    Ok(())
}

fn handle_socket_event(cx: &AsyncApp, event: SocketEvent) -> Result<(), Box<dyn std::error::Error>> {
    match event {
        SocketEvent::List(event) => {
            cx.update(|cx| Applications::update_list(cx, event.apps.clone()))?;
        }
        SocketEvent::Launch(event) => {
            cx.update(|cx| Applications::update_list_entry(cx, event.app.as_ref(), Some(IndexType::Start), false))?;
        }
        SocketEvent::Close(event) => {
            cx.update(|cx| Applications::update_list_entry(cx, event.app.as_ref(), None, false))?;
        }
        SocketEvent::Activate(event) => {
            cx.update(|cx| Applications::update_list_entry(cx, event.app.as_ref(), Some(IndexType::Start), false))?;
        }
    }
    Ok(())
}

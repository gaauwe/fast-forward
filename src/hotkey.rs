use gpui::{App, Global};
use log::error;
use tokio::sync::mpsc::UnboundedSender;
use std::sync::atomic::{AtomicBool, Ordering};

use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop, CFRunLoopSource};
use core_graphics::event::{CGEventFlags, CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventType, EventField};

use crate::{commander::{Commander, EventType, HotkeyEvent}, config::Config};

pub static IS_ACTIVE: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    /// Tab key (keycode 48)
    Tab = 48,
    /// Space key (keycode 49)
    Space = 49,
    /// Escape key (keycode 53)
    Escape = 53,
    /// Right Command key (keycode 54)
    RightCommand = 54,
    /// Left Command key (keycode 55)
    LeftCommand = 55,
    /// Any other key
    Other
}

impl From<i64> for Key {
    fn from(value: i64) -> Self {
        match value {
            48 => Key::Tab,
            49 => Key::Space,
            53 => Key::Escape,
            55 => Key::LeftCommand,
            54 => Key::RightCommand,
            _ => Key::Other
        }
    }
}

pub struct Hotkey {
    _tap: CGEventTap<'static>,
    _loop_source: CFRunLoopSource,
}

impl Hotkey {
    pub fn new(cx: &mut App) {
        let handler = EventHandler::new(cx);
        let current = CFRunLoop::get_current();
        let tap = CGEventTap::new(
            CGEventTapLocation::Session,
            CGEventTapPlacement::HeadInsertEventTap,
            CGEventTapOptions::Default,
            vec![CGEventType::FlagsChanged, CGEventType::KeyDown],
            move |_, event_type, event| {
                let keycode = event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE);
                let new_event = event.clone();
                let mut flags = event.get_flags();

                match event_type {
                    CGEventType::FlagsChanged => {
                        if let Some(hotkey_event) = handler.handle_flags_changed(keycode, flags) {
                            handler.send_event(hotkey_event);
                        }
                    },
                    CGEventType::KeyDown => {
                        if let Some(hotkey_event) = handler.handle_flags_changed(keycode, flags) {
                            handler.send_event(hotkey_event);
                        }

                        if let Some((hotkey_event, should_block)) = handler.handle_key_down(keycode, &mut flags) {
                            handler.send_event(hotkey_event);
                            if should_block {
                                new_event.set_type(CGEventType::Null);
                            }
                        }
                        new_event.set_flags(flags);
                    },
                    _ => {}
                }

                Some(new_event)
            },
        ).unwrap_or_else(|e| {
            panic!("Failed to create event tap: {:?}", e);
        });

        let loop_source = tap.mach_port
            .create_runloop_source(0)
            .unwrap_or_else(|e| {
                panic!("Failed to create runloop source: {:?}", e);
            });

        unsafe {
            current.add_source(&loop_source, kCFRunLoopCommonModes);
            tap.enable();
        }

        cx.set_global(Self {
            _tap: tap,
            _loop_source: loop_source,
        });
    }
}

impl Global for Hotkey {}


struct EventHandler {
    tx: UnboundedSender<EventType>,
    enable_left_cmd: bool,
}

impl EventHandler {
    fn new(cx: &mut App) -> Self {
        let tx = cx.global::<Commander>().tx.clone();
        let config = cx.global::<Config>();
        let enable_left_cmd = config.general.enable_left_cmd;

        Self { tx, enable_left_cmd }
    }

    fn handle_flags_changed(&self, keycode: i64, flags: CGEventFlags) -> Option<HotkeyEvent> {
        let is_active = match keycode.into() {
            Key::RightCommand => {
                Some((flags.contains(CGEventFlags::CGEventFlagCommand), 0))
            }
            Key::Tab => {
                if self.enable_left_cmd {
                    Some((flags.contains(CGEventFlags::CGEventFlagCommand), 1))
                } else {
                    None
                }
            }
            Key::LeftCommand => {
                if self.enable_left_cmd && !flags.contains(CGEventFlags::CGEventFlagCommand) {
                    Some((false, 0))
                } else {
                    None
                }
            }
            _ => None,
        };

        is_active.and_then(|(active, offset)| {
            if IS_ACTIVE.load(Ordering::SeqCst) == active {
                None
            } else {
                IS_ACTIVE.store(active, Ordering::SeqCst);
                Some(if active {
                    HotkeyEvent::ShowWindow(offset)
                } else {
                    HotkeyEvent::HideWindow
                })
            }
        })
    }

    fn handle_key_down(&self, keycode: i64, flags: &mut CGEventFlags) -> Option<(HotkeyEvent, bool)> {
        if !IS_ACTIVE.load(Ordering::SeqCst) {
            return None;
        }

        // Remove command flag for normal key press behavior, as they should be handled as normal key events.
        flags.remove(CGEventFlags::CGEventFlagCommand);

        match Key::from(keycode) {
            Key::Escape => {
                Some((HotkeyEvent::QuitApplication, true))
            },
            Key::Space => {
                Some((HotkeyEvent::HideApplication, true))
            },
            _ => {
                None
            }
        }
    }

    fn send_event(&self, event: HotkeyEvent) {
        if let Err(e) = self.tx.send(EventType::HotkeyEvent(event)) {
            error!("Failed to send event {:?}: {:?}", event, e);
        }
    }
}

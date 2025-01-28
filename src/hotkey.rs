use gpui::*;
use log::{info, error};
use std::sync::atomic::{AtomicBool, Ordering};

use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop, CFRunLoopSource};
use core_graphics::event::{CGEventFlags, CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventType, EventField};

use crate::commander::{Commander, EventType, HotkeyEvent};

static RIGHT_CMD_IS_DOWN: AtomicBool = AtomicBool::new(false);
static ESCAPE_PRESSED: AtomicBool = AtomicBool::new(false);
static SPACE_PRESSED: AtomicBool = AtomicBool::new(false);

pub struct Hotkey {
    _tap: CGEventTap<'static>,
    _loop_source: CFRunLoopSource,
}

pub enum Key {
    RightCommand = 54,
    Escape = 53,
    Space = 49,
    Other
}

impl From<i64> for Key {
    fn from(value: i64) -> Self {
        match value {
            54 => Key::RightCommand,
            53 => Key::Escape,
            49 => Key::Space,
            _ => Key::Other
        }
    }
}


impl Hotkey {
    pub fn new(cx: &mut AppContext) {
        let tx = cx.global::<Commander>().tx.clone();
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
                        if keycode == Key::RightCommand as i64 {
                            let right_cmd_is_down = flags.contains(CGEventFlags::CGEventFlagCommand);
                            if right_cmd_is_down {
                                info!("Right Command key pressed");
                                if let Err(e) = tx.send(EventType::HotkeyEvent(HotkeyEvent::ShowWindow)) {
                                    error!("Failed to send ShowWindow event: {:?}", e);
                                }
                            } else {
                                info!("Right Command key released");
                                if let Err(e) = tx.send(EventType::HotkeyEvent(HotkeyEvent::HideWindow)) {
                                    error!("Failed to send HideWindow event: {:?}", e);
                                }
                            }

                            RIGHT_CMD_IS_DOWN.store(right_cmd_is_down, Ordering::SeqCst);
                        }
                    },
                    CGEventType::KeyDown => {
                        // If the event is a right command key press, block it (since that's our trigger).
                        if keycode == Key::RightCommand as i64 {
                            new_event.set_type(CGEventType::Null);
                        }

                        // Early return if command is not down (and therefore our application isn't active).
                        if !RIGHT_CMD_IS_DOWN.load(Ordering::SeqCst) {
                            return Some(new_event);
                        }

                        // Remove the command key from the flags, so that it acts as a normal key press.
                        flags.remove(CGEventFlags::CGEventFlagCommand);
                        new_event.set_flags(flags);

                        match Key::from(keycode) {
                            Key::Escape => {
                                info!("Escape key pressed");
                                if let Err(e) = tx.send(EventType::HotkeyEvent(HotkeyEvent::QuitApplication)) {
                                    error!("Failed to send QuitApplication event: {:?}", e);
                                }

                                new_event.set_type(CGEventType::Null);
                                ESCAPE_PRESSED.store(true, Ordering::SeqCst);
                            },
                            Key::Space => {
                                info!("Space key pressed");
                                if let Err(e) = tx.send(EventType::HotkeyEvent(HotkeyEvent::HideApplication)) {
                                    error!("Failed to send HideApplication event: {:?}", e);
                                }

                                new_event.set_type(CGEventType::Null);
                                SPACE_PRESSED.store(true, Ordering::SeqCst);
                            },
                            _ => {
                                SPACE_PRESSED.store(false, Ordering::SeqCst);
                                ESCAPE_PRESSED.store(false, Ordering::SeqCst);
                            }
                        }
                    },
                    _ => {}
                }

                Some(new_event)
            },
        ).unwrap_or_else(|e| {
            error!("Failed to create event tap: {:?}", e);
            panic!("Failed to create event tap");
        });

        let loop_source = tap.mach_port
            .create_runloop_source(0)
            .unwrap_or_else(|e| {
                error!("Failed to create runloop source: {:?}", e);
                panic!("Failed to create runloop source");
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

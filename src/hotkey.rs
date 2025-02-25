use gpui::{App, Global};
use log::error;
use tokio::sync::mpsc::UnboundedSender;
use std::sync::atomic::{AtomicBool, Ordering};

use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop, CFRunLoopSource};
use core_graphics::event::{CGEventFlags, CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventType, EventField};

use crate::{commander::{Commander, EventType, HotkeyEvent}, window::Window};

static RIGHT_CMD_IS_DOWN: AtomicBool = AtomicBool::new(false);
static ESCAPE_PRESSED: AtomicBool = AtomicBool::new(false);
static SPACE_PRESSED: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    /// Right Command key (keycode 54)
    RightCommand = 54,
    /// Escape key (keycode 53)
    Escape = 53,
    /// Space key (keycode 49)
    Space = 49,
    /// Any other key
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

pub struct Hotkey {
    _tap: CGEventTap<'static>,
    _loop_source: CFRunLoopSource,
}

impl Hotkey {
    pub fn new(cx: &mut App) {
        let handler = EventHandler::new(cx.global::<Commander>().tx.clone());
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
                        // Block right command key events.
                        if keycode == Key::RightCommand as i64 {
                            new_event.set_type(CGEventType::Null);
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

        // Trap focus as long as the command key is pressed.
        let _ = cx.global::<Window>().window.clone().update(cx, |_view, window, cx| {
            cx.observe_window_activation(window, |_input, window, cx| {
                if !window.is_window_active() && RIGHT_CMD_IS_DOWN.load(Ordering::SeqCst) {
                    cx.activate(true);
                }
            }).detach();
        });


        cx.set_global(Self {
            _tap: tap,
            _loop_source: loop_source,
        });
    }
}

impl Global for Hotkey {}


struct EventHandler {
    tx: UnboundedSender<EventType>,
}

impl EventHandler {
    fn new(tx: UnboundedSender<EventType>) -> Self {
        Self { tx }
    }

    fn handle_flags_changed(&self, keycode: i64, flags: CGEventFlags) -> Option<HotkeyEvent> {
        if keycode == Key::RightCommand as i64 {
            let right_cmd_is_down = flags.contains(CGEventFlags::CGEventFlagCommand);
            RIGHT_CMD_IS_DOWN.store(right_cmd_is_down, Ordering::SeqCst);
            Some(if right_cmd_is_down {
                HotkeyEvent::ShowWindow
            } else {
                HotkeyEvent::HideWindow
            })
        } else {
            None
        }
    }

    fn handle_key_down(&self, keycode: i64, flags: &mut CGEventFlags) -> Option<(HotkeyEvent, bool)> {
        // Early return if command is not down.
        if !RIGHT_CMD_IS_DOWN.load(Ordering::SeqCst) {
            return None;
        }

        // Remove command flag for normal key press behavior, as they should be handled as normal key events.
        flags.remove(CGEventFlags::CGEventFlagCommand);

        match Key::from(keycode) {
            Key::Escape => {
                ESCAPE_PRESSED.store(true, Ordering::SeqCst);
                Some((HotkeyEvent::QuitApplication, true))
            },
            Key::Space => {
                SPACE_PRESSED.store(true, Ordering::SeqCst);
                Some((HotkeyEvent::HideApplication, true))
            },
            _ => {
                SPACE_PRESSED.store(false, Ordering::SeqCst);
                ESCAPE_PRESSED.store(false, Ordering::SeqCst);
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

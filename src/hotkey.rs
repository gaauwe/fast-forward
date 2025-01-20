use std::{sync::mpsc::channel, time::Duration};

use gpui::*;
use swift_rs::{swift, Bool};
use std::sync::atomic::{AtomicBool, Ordering};

use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop, CFRunLoopSource};
use core_graphics::event::{CGEventFlags, CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventType, EventField};

use crate::{applications::Applications, window::Window};

swift!(fn enable_accessibility_features() -> Bool);

static RIGHT_CMD_IS_DOWN: AtomicBool = AtomicBool::new(false);
static ESCAPE_PRESSED: AtomicBool = AtomicBool::new(false);
static SPACE_PRESSED: AtomicBool = AtomicBool::new(false);

pub struct HotkeyManager {
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

pub enum EventType {
    ShowWindow,
    HideWindow,
    MinimizeApplication,
    QuitApplication
}

impl HotkeyManager {
    pub fn new(cx: &mut AppContext) {
        // Enable accessibility features.
        unsafe {
            enable_accessibility_features();
        }

        // Create channel for hiding/showing the window.
        let (sender, receiver) = channel::<EventType>();
        cx.spawn(move |cx| async move {
            loop {
                if let Ok(event) = receiver.try_recv() {
                    match event {
                        EventType::ShowWindow => {
                            let _ = cx.update(|cx| {
                                Window::new(cx);
                            });
                        }
                        EventType::HideWindow => {
                            let _ = cx.update(|cx| {
                                if !SPACE_PRESSED.load(Ordering::SeqCst) && !ESCAPE_PRESSED.load(Ordering::SeqCst)  {
                                    Applications::fire_event(cx, "activate");
                                }

                                Window::close(cx);
                            });
                        },
                        EventType::MinimizeApplication => {
                            let _ = cx.update(|cx| {
                                Applications::fire_event(cx, "minimize");
                            });
                        },
                        EventType::QuitApplication => {
                            let _ = cx.update(|cx| {
                                Applications::fire_event(cx, "quit");
                            });
                        }
                    }
                }

                cx.background_executor()
                    .timer(Duration::from_millis(50))
                    .await;
            }
        })
        .detach();

        // Create the event tap.
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
                                let _ = sender.send(EventType::ShowWindow);
                            } else {
                                let _ = sender.send(EventType::HideWindow);
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
                                let _ = sender.send(EventType::QuitApplication);

                                new_event.set_type(CGEventType::Null);
                                ESCAPE_PRESSED.store(true, Ordering::SeqCst);
                            },
                            Key::Space => {
                                let _ = sender.send(EventType::MinimizeApplication);

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
        ).expect("Failed to create event tap");

        let loop_source = tap.mach_port
            .create_runloop_source(0)
            .expect("Failed to create runloop source");

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

impl Global for HotkeyManager {}

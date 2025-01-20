use std::{sync::mpsc::channel, time::Duration};

use gpui::{AppContext, Global};
use swift_rs::{swift, Bool};
use std::sync::atomic::{AtomicBool, Ordering};

use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop, CFRunLoopSource};
use core_graphics::event::{CGEventFlags, CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventType, EventField};

use crate::{applications::Applications, window::Window};

swift!(fn enable_accessibility_features() -> Bool);

static RIGHT_CMD_IS_DOWN: AtomicBool = AtomicBool::new(false);

pub struct HotkeyManager {
    _tap: CGEventTap<'static>,
    _loop_source: CFRunLoopSource,
}

pub enum EventType {
    KeyPress,
    KeyRelease,
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
                        EventType::KeyPress => {
                            let _ = cx.update(|cx| {
                                Window::new(cx);
                            });
                        }
                        EventType::KeyRelease => {
                            let _ = cx.update(|cx| {
                                Applications::focus_active_window(cx);
                                Window::close(cx);
                            });
                        },
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
                        if keycode == 54 {
                            let right_cmd_is_down = flags.contains(CGEventFlags::CGEventFlagCommand);
                            if right_cmd_is_down {
                                let _ = sender.send(EventType::KeyPress);
                            } else {
                                let _ = sender.send(EventType::KeyRelease);
                            }

                            RIGHT_CMD_IS_DOWN.store(right_cmd_is_down, Ordering::SeqCst);
                        }
                    },
                    CGEventType::KeyDown => {
                        // If the event is a right command key press, block it (since that's our trigger).
                        if keycode == 54 {
                            new_event.set_type(CGEventType::Null);
                        }
                        // If the event is combined with the right command key, remove the command key from the flags.
                        else if RIGHT_CMD_IS_DOWN.load(Ordering::SeqCst) {
                            flags.remove(CGEventFlags::CGEventFlagCommand);
                            new_event.set_flags(flags);
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

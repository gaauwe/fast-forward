use std::{sync::mpsc::channel, time::Duration};

use gpui::{AppContext, Global};
use swift_rs::{swift, Bool};
use rdev::{grab, EventType, Key, Event};

use crate::{applications::Applications, window::Window};

swift!(fn enable_accessibility_features() -> Bool);

pub struct HotkeyManager {
}

impl HotkeyManager {
    pub fn new(cx: &mut AppContext) {
        // Enable accessibility features.
        unsafe {
            enable_accessibility_features();
        }

        let (sender, receiver) = channel::<Event>();
        cx.spawn(move |cx| async move {
            loop {
                if let Ok(event) = receiver.try_recv() {
                    match event.event_type {
                        EventType::KeyPress(Key::MetaRight) => {
                            let _ = cx.update(|cx| {
                                Window::new(cx);
                            });
                        }
                        EventType::KeyRelease(Key::MetaRight) => {
                            let _ = cx.update(|cx| {
                                Applications::focus_active_window(cx);
                                Window::close(cx);
                            });
                        },
                        _ => {}
                    }
                }

                cx.background_executor()
                    .timer(Duration::from_millis(50))
                    .await;
            }
        })
        .detach();


        if let Err(error) = grab(move |event| {
            match event.event_type {
                EventType::KeyPress(Key::MetaRight) => {
                    let _ = sender.send(event.clone());
                    None
                }
                EventType::KeyRelease(Key::MetaRight) => {
                    let _ = sender.send(event.clone());
                    None
                }
                _ => Some(event),
            }
        }) {
            eprintln!("Error in grab: {:?}", error)
        }
    }
}

impl Global for HotkeyManager {}

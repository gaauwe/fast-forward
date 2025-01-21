use gpui::{AppContext, Global};
use tokio::sync::watch;
use tray_icon::{menu::{Menu, MenuEvent, MenuItem}, TrayIcon, TrayIconBuilder};

use crate::config::Config;

pub struct Tray {
    _tray: TrayIcon,
}

pub enum EventType {
    Settings,
    About,
    Quit,
    None
}

impl Tray {
    pub fn new(cx: &mut AppContext) {
        let (tx, mut rx) = watch::channel(EventType::None);
        cx.spawn(|cx| async move {
            while rx.changed().await.is_ok() {
                match *rx.borrow() {
                    EventType::Settings => {
                        let config_path = Config::config_path().unwrap();
                        let _ = std::process::Command::new("open")
                            .arg("-a")
                            .arg("TextEdit")
                            .arg(&config_path)
                            .spawn();
                    }
                    EventType::About => {
                        let _ = cx.update(|cx| {
                            cx.open_url("https://github.com/gaauwe/fast-forward")
                        });
                    },
                    EventType::Quit => {
                        let _ = cx.update(|cx| {
                            cx.quit()
                        });
                    },
                    EventType::None => {}
                }
            }
        }).detach();

        let icon = load_icon();
        let menu = Menu::new();

        let settings_action = MenuItem::with_id("settings", "Settings", true, None);
        let about_action = MenuItem::with_id("about", "About Fast Forward", true, None);
        let quit_action = MenuItem::with_id("quite", "Quit...", true, None);

        let _ = menu.append_items(&[
            &settings_action,
            &about_action,
            &quit_action
        ]);

        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_icon(icon)
            .build()
            .unwrap();

        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            match event.id.0.as_str() {
                "settings" => tx.send(EventType::Settings).expect("Failed to forward Settings event"),
                "about" => tx.send(EventType::About).expect("Failed to forward Settings event"),
                "quit" => tx.send(EventType::Quit).expect("Failed to forward Settings event"),
                _ => {}
            }
        }));

        cx.set_global(Self {
            _tray: tray,
        });
    }
}

impl Global for Tray {}

fn load_icon() -> tray_icon::Icon {
    let (icon_rgba, icon_width, icon_height) = {
        let icon_bytes = include_bytes!("../assets/tray_icon.png");
        let image = image::load_from_memory(icon_bytes)
            .expect("Failed to open icon path")
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };
    tray_icon::Icon::from_rgba(icon_rgba, icon_width, icon_height).expect("Failed to open icon")
}

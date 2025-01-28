use gpui::{App, Global};
use log::{info, error};
use tray_icon::{menu::{Menu, MenuEvent, MenuItem}, TrayIcon, TrayIconBuilder};

use crate::commander::{Commander, EventType, TrayEvent};

pub struct Tray {
    _tray: TrayIcon,
}

#[derive(Debug, Clone, Copy)]
pub enum MenuId {
    Settings,
    About,
    Quit,
}

impl MenuId {
    fn as_str(&self) -> &'static str {
        match self {
            MenuId::Settings => "settings",
            MenuId::About => "about",
            MenuId::Quit => "quit",
        }
    }
}


impl Tray {
    pub fn new(cx: &mut App) {
        let tx = cx.global::<Commander>().tx.clone();
        let icon = Self::load_icon();
        let menu = Menu::new();

        let settings_action = MenuItem::with_id(MenuId::Settings.as_str(), "Settings", true, None);
        let about_action = MenuItem::with_id(MenuId::About.as_str(), "About Fast Forward", true, None);
        let quit_action = MenuItem::with_id(MenuId::Quit.as_str(), "Quit...", true, None);

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
            info!("Sending event: {:?}", event);
            match event.id.0.as_str() {
                id if id == MenuId::Settings.as_str() => {
                    if let Err(e) = tx.send(EventType::TrayEvent(TrayEvent::Settings)) {
                        error!("Failed to forward Settings event: {:?}", e);
                    }
                },
                id if id == MenuId::About.as_str() => {
                    if let Err(e) = tx.send(EventType::TrayEvent(TrayEvent::About)) {
                        error!("Failed to forward About event: {:?}", e);
                    }
                },
                id if id == MenuId::Quit.as_str() => {
                    if let Err(e) = tx.send(EventType::TrayEvent(TrayEvent::Quit)) {
                        error!("Failed to forward Quit event: {:?}", e);
                    }
                },
                _ => {}
            }
        }));

        cx.set_global(Self {
            _tray: tray,
        });
    }

    fn load_icon() -> tray_icon::Icon {
        let (icon_rgba, icon_width, icon_height) = {
            let icon_bytes = include_bytes!("../assets/tray_icon.png");
            let image = image::load_from_memory(icon_bytes)
                .unwrap_or_else(|e| {
                    error!("Failed to open icon path: {:?}", e);
                    panic!("Failed to open icon");
                })
                .into_rgba8();
            let (width, height) = image.dimensions();
            let rgba = image.into_raw();
            (rgba, width, height)
        };
        tray_icon::Icon::from_rgba(icon_rgba, icon_width, icon_height)
            .unwrap_or_else(|e| {
                error!("Failed to open icon: {:?}", e);
                panic!("Failed to open icon");
            })
    }
}

impl Global for Tray {}

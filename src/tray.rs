use gpui::{App, Global};
use log::error;
use tokio::sync::mpsc::UnboundedSender;
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

        let _ = menu.append_items(&[
            &MenuItem::with_id(MenuId::Settings.as_str(), "Settings", true, None),
            &MenuItem::with_id(MenuId::About.as_str(), "About Fast Forward", true, None),
            &MenuItem::with_id(MenuId::Quit.as_str(), "Quit...", true, None),
        ]);

        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_icon(icon)
            .build()
            .unwrap();

        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            match event.id.0.as_str() {
                id if id == MenuId::Settings.as_str() => Self::send_tray_event(&tx, TrayEvent::Settings),
                id if id == MenuId::About.as_str() => Self::send_tray_event(&tx, TrayEvent::About),
                id if id == MenuId::Quit.as_str() => Self::send_tray_event(&tx, TrayEvent::Quit),
                _ => {}
            }
        }));

        cx.set_global(Self {
            _tray: tray,
        });
    }

    fn send_tray_event(tx: &UnboundedSender<EventType>, event: TrayEvent) {
        if let Err(e) = tx.send(EventType::TrayEvent(event)) {
            error!("Failed to forward tray event: {:?}", e);
        }
    }

    fn load_icon() -> tray_icon::Icon {
        let icon_bytes = include_bytes!("../assets/tray_icon.png");
        let image = image::load_from_memory(icon_bytes)
            .unwrap_or_else(|e| {
                panic!("Failed to load icon: {:?}", e);
            })
            .into_rgba8();

        let (width, height) = image.dimensions();
        let rgba = image.into_raw();

        tray_icon::Icon::from_rgba(rgba, width, height)
            .unwrap_or_else(|e| {
                panic!("Failed to create icon: {:?}", e);
            })
    }
}

impl Global for Tray {}

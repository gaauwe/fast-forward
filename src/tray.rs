use std::time::Duration;
use gpui::{AppContext, Global};
use tray_icon::{menu::{Menu, MenuEvent, MenuItem}, TrayIcon, TrayIconBuilder};

pub struct Tray {
    _tray: TrayIcon,
}

impl Tray {
    pub fn new(cx: &mut AppContext) {
        let icon = load_icon();
        let menu = Menu::new();

        let about_action = MenuItem::new("About Fast Forward", true, None);
        let quit_action = MenuItem::new("Quit...", true, None);

        let _ = menu.append_items(&[
            &about_action,
            &quit_action
        ]);

        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_icon(icon)
            .build()
            .unwrap();

        cx.spawn(|cx| async move {
            loop {
                if let Ok(event) = MenuEvent::receiver().try_recv() {
                    if event.id == about_action.id() {
                        let _ = cx.update(|cx| {
                            cx.open_url("https://github.com/gaauwe/fast-forward")
                        });
                    }
                    if event.id == quit_action.id() {
                        let _ = cx.update(|cx| {
                            cx.quit();
                        });
                    }
                }

                cx.background_executor()
                    .timer(Duration::from_millis(50))
                    .await;
            }
        })
        .detach();

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

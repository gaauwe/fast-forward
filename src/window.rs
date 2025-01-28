use core_graphics::display::{CGDisplay, CGPoint};
use gpui::*;
use mouse_position::mouse_position::Mouse;

use crate::{applications::{Applications, IndexType}, ui::{input::SearchQuery, Container}};

pub struct Window {
    pub window: WindowHandle<Container>,
    pub display_id: DisplayId,
}

impl Window {
    pub fn new(cx: &mut App) {
        // Calculate the bounds of the active display.
        let display_id = Some(Self::get_active_display_id(cx));
        let bounds = cx.displays().iter().find(|d| Some(d.id()) == display_id).map(|d| d.bounds()).unwrap_or(Bounds {
            origin: Point::new(Pixels::from(0.0), Pixels::from(0.0)),
            size: Size {
                width: Pixels::from(1920.0),
                height: Pixels::from(1080.0),
            },
        });

        // Calculate the height and position of the window.
        let height = Pixels(bounds.size.height.0 * 0.5);
        let width = Pixels::from(544.0);
        let center = bounds.center();
        let x: Pixels = center.x - width / 2.0;
        let y: Pixels = Pixels(bounds.size.height.0 * 0.3);

        // Launch the window.
        let window = cx.open_window(
            WindowOptions {
                titlebar: None,
                window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                    Point { x, y },
                    Size { width, height },
                ))),
                window_background: WindowBackgroundAppearance::Blurred,
                kind: WindowKind::PopUp,
                is_movable: false,
                display_id,
                ..Default::default()
            },
            |window, cx| {
                cx.new(|cx| Container::new(window, cx))
            },
        )
        .unwrap();

        // Auto focus the input field.
        window
            .update(cx, |view, window, cx| {
                cx.focus_view(&view.input, window);
                cx.activate(true);
            })
            .unwrap();

        cx.set_global(Self {
            window,
            display_id: display_id.unwrap(),
        });
    }

    pub fn show(cx: &mut App) {
        let display_id = cx.global::<Self>().display_id;
        let active_display_id = Self::get_active_display_id(cx);

        // If the display has changed, close the current window and open a new one.
        if display_id != active_display_id {
            Self::close(cx);

            // Delay the opening of the window to prevent flickering.
            cx.spawn(|cx| async move {
                cx.background_executor().timer(std::time::Duration::from_millis(0)).await;
                cx.update(|cx| {
                    Self::new(cx);
                })
            }).detach();
        }

        let window = cx.global::<Window>();
        window.window.clone().update(cx, |_view, _window, cx| {
            cx.activate(true);
        }).ok();
    }

    pub fn hide(cx: &mut App) {
        // Hide the window and reset the input field.
        let window = cx.global::<Window>();
        window.window.clone().update(cx, |view, _window, cx| {
            cx.hide();


            view.input.update(cx, |input, _cx| {
                input.value = "".into();
            });
            cx.notify();
        }).ok();

        // Clear the search query when the window is hidden (after a short delay).
        cx.spawn(|cx| async move {
            cx.background_executor().timer(std::time::Duration::from_millis(0)).await;
            cx.update(|cx| {
                cx.set_global(SearchQuery { value: String::new() });
            })
        }).detach();

        // Reset the active index when the window is hidden.
        Applications::update_active_index(cx, IndexType::Start);
    }

    pub fn close(cx: &mut App) {
        let window = cx.global::<Window>();
        window.window.clone().update(cx, |_view, window, _cx| {
            window.remove_window();
        }).ok();
    }

    fn get_active_display_id(cx: &mut App) -> DisplayId {
        let mouse_location = Mouse::get_mouse_position();
        let gpui_displays = cx.displays();
        let fallback_display_id = gpui_displays.first().unwrap().id();

        match mouse_location {
            Mouse::Position { x, y } => {
                // Get all active displays.
                let displays = CGDisplay::active_displays().unwrap_or_default();

                // Check for each display if the mouse is in its bounds.
                for display_id in displays {
                    let display = CGDisplay::new(display_id);
                    let bounds = display.bounds();

                    if bounds.contains(&CGPoint { x: x as f64, y: y as f64 }) {
                        // Find the corresponding GPUI display, since that returns a DisplayId that we can use to open a window.
                        let gpui_display = gpui_displays.iter().find(|d| {
                            // We can't access the private integer, but the struct does implement fmt based on the private integer 🥴.
                            let id = format!("{:?}", d.id());
                            id == format!("DisplayId({})", display_id)
                        });

                        return gpui_display.unwrap().id();
                    }
                }

                fallback_display_id
            },
            Mouse::Error => fallback_display_id,
        }
    }
}

impl Global for Window {}

use core_graphics::display::{CGDisplay, CGPoint};
use gpui::*;
use mouse_position::mouse_position::Mouse;

use crate::{applications::Applications, ui::App};

pub struct Window {}

impl Window {
    pub fn new(cx: &mut AppContext) {
        // Refresh the list of open application windows.
        Applications::new(cx);

        // Calculate the bounds of the active display.
        let display_id = Some(get_active_display_id(cx));
        let bounds = cx.displays().iter().find(|d| Some(d.id()) == display_id).map(|d| d.bounds()).unwrap_or(Bounds {
            origin: Point::new(Pixels::from(0.0), Pixels::from(0.0)),
            size: Size {
                width: Pixels::from(1920.0),
                height: Pixels::from(1080.0),
            },
        });

        // Calculate the height and position of the window.
        let height = Pixels(App::get_height(cx));
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
            |cx| {
                cx.new_view(|cx| App::new(cx))
            },
        )
        .unwrap();



        // Auto focus the input field.
        window
            .update(cx, |view, cx| {
                cx.focus_view(&view.input);
                cx.activate(true);
            })
            .unwrap();
    }

    pub fn close(cx: &mut AppContext) {
        let windows = cx.windows();
        for window in windows {
            cx.update_window(window, |_, cx| {
                cx.remove_window();
            }).ok();
        }
    }
}

impl Global for Window {}

fn get_active_display_id(cx: &mut AppContext) -> DisplayId {
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

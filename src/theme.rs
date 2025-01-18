use gpui::*;

pub struct Theme {
    pub primary: Hsla,
    pub background: Hsla,
    pub surface: Hsla,
    pub border: Hsla,
}

impl Theme {
    pub fn new(cx: &mut AppContext) {
        cx.set_global(Self {
            primary: hsla(211., 100., 50., 1.),
            background: hsla(0., 0., 0., 0.8),
            surface: hsla(0., 0., 100., 0.05),
            border: hsla(0., 0., 100., 0.1),
        });
    }
}

impl Global for Theme {}

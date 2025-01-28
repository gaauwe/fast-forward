use gpui::*;
use serde::Deserialize;

use crate::config::Config;

#[derive(Copy, Clone, Debug, Deserialize)]
pub struct Theme {
    pub primary: Hsla,
    pub background: Hsla,
    pub foreground: Hsla,
    pub muted: Hsla,
    pub muted_foreground: Hsla,
    pub border: Hsla,
}

#[derive(Debug, Deserialize)]
pub struct ThemeConfig {
    pub primary: Option<Color>,
    pub background: Option<Color>,
    pub foreground: Option<Color>,
    pub muted: Option<Color>,
    pub muted_foreground: Option<Color>,
    pub border: Option<Color>,
}

#[derive(Debug, Deserialize)]
pub struct Color {
    pub h: f32,
    pub s: f32,
    pub l: f32,
    pub a: f32,
}

impl From<Color> for Hsla {
    fn from(color: Color) -> Self {
        hsla(color.h, color.s, color.l, color.a)
    }
}

impl Theme {
    pub fn new(cx: &mut App) {
        let config = cx.global::<Config>();
        let theme = config.theme;

        cx.set_global(theme);
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            primary: hsla(0.508, 0.33, 0.38, 1.),
            background: hsla(0., 0., 15. / 100., 0.9),
            foreground: hsla(0., 0., 100. / 100., 0.8),
            muted: hsla(0., 0., 31. / 100., 1.),
            muted_foreground: hsla(0., 0., 100. / 100., 0.4),
            border: hsla(0., 0., 31. / 100., 1.),
        }
    }
}

impl Global for Theme {}

use gpui::*;
use prelude::FluentBuilder;

use crate::{applications::Applications, theme::Theme};

pub struct List {
}

impl List {
    pub fn new() -> Self {
        Self { }
    }
}

impl Render for List {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let applications = cx.global::<Applications>();
        let active_index = applications.active_index;

        if applications.filtered_windows.len() > 0 {
            div().child(uniform_list(cx.view().clone(), "entries", applications.filtered_windows.len(), {
                let windows = applications.filtered_windows.clone();
                move |_this, range, cx| {
                    let theme = cx.global::<Theme>();

                    range.map(|i| {
                        let name = windows[i].name.to_string();
                        let icon_path = windows[i].icon_path.clone();

                        div()
                            .id(i)
                            .cursor_pointer()
                            .flex()
                            .h(px(40.0))
                            .w_full()
                            .items_center()
                            .gap_2()
                            .rounded(px(4.0))
                            .p_1()
                            .text_sm()
                            .text_color(theme.foreground)
                            .when(i == active_index, |cx| cx.bg(theme.muted))
                            .child(img(icon_path).h(px(32.0)).w(px(32.0)))
                            .child(div().child(name))
                    }).collect::<Vec<_>>()
                }
            }).h_full()).h_full()
        } else {
            div()
                .flex()
                .h_full()
                .items_center()
                .justify_center()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child("No windows found")
        }.into_any()
    }
}

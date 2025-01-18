use gpui::*;
use prelude::FluentBuilder;

use crate::applications::Applications;

use super::input::TextInput;

pub struct List {
    pub input: View<TextInput>,
}

impl List {
    pub fn new(input: View<TextInput>) -> Self {
        Self { input }
    }

    pub fn get_height(cx: &AppContext) -> f32 {
        let applications = cx.global::<Applications>();
        let max_items = applications.filtered_windows.len();
        (max_items as f32 * 40.0) + 51.
    }
}

impl Render for List {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let applications = cx.global::<Applications>();
        let active_index = applications.active_index;

        // Default height if no windows are found.
        let height = if applications.filtered_windows.len() > 0 {
            Self::get_height(cx)
        } else {
            100. + 51.
        };

        div()
            .flex()
            .flex_col()
            .h(px(height))
            .w_full()
            .gap_1()
            .rounded_xl()
            .border_1()
            .border_color(rgb(0x505050))
            .bg(rgba(0x262626E6))
            .p(px(5.0))
            .mx(px(10.))
            .child(self.input.clone())
            .child(
                if applications.filtered_windows.len() > 0 {
                    div().child(uniform_list(cx.view().clone(), "entries", applications.filtered_windows.len(), {
                        let windows = applications.filtered_windows.clone();
                        move |_this, range, _cx| {
                            let mut items = Vec::new();
                            for i in range {
                                let name = windows[i].name.to_string();
                                let icon_path = windows[i].icon_path.clone();

                                items.push(
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
                                        .text_color(rgba(0xFFFFFFCC))
                                        .when(i == active_index, |cx| cx.bg(rgb(0x505050)))
                                        .child(img(icon_path).h(px(32.0)).w(px(32.0)))
                                        .child(div().child(name)),
                                );
                            }
                            items
                        }
                    }).h_full()).h_full()
                } else {
                    div()
                        .flex()
                        .h_full()
                        .items_center()
                        .justify_center()
                        .text_sm()
                        .text_color(rgba(0xFFFFFF77))
                        .child("No windows found")
                }.into_any()
            )
    }
}

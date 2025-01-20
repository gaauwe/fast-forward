pub mod input;
pub mod list;

use gpui::*;
use input::TextInput;

use crate::{applications::Applications, theme::Theme, ui::list::List};

pub struct App {
    pub input: View<TextInput>,
    pub list: View<List>,
}

impl App {
    pub fn new(cx: &mut ViewContext<Self>) -> Self {
        let input = cx.new_view(|cx| TextInput::new(cx));
        let list = cx.new_view(|_cx| List::new());

        Self { input, list }
    }

    pub fn get_height(cx: &AppContext) -> f32 {
        let applications = cx.global::<Applications>();
        let max_items = applications.filtered_windows.len();
        (max_items as f32 * 40.0) + 51. + 36.
    }
}

impl Render for App {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let applications = cx.global::<Applications>();

        // Default height if no windows are found.
        let height = if applications.filtered_windows.len() > 0 {
            Self::get_height(cx)
        } else {
            100. + 51. + 32.
        };

        div()
            .flex()
            .flex_col()
            .h(px(height))
            .w_full()
            .gap_1()
            .rounded_xl()
            .border_1()
            .border_color(theme.border)
            .bg(theme.background)
            .p(px(5.0))
            .mx(px(10.))
            .child(self.input.clone())
            .child(self.list.clone())
            .child(
                div()
                    .flex()
                    .gap_4()
                    .h(px(32.))
                    .items_center()
                    .justify_end()
                    .text_sm()
                    .pt_2()
                    .pr_2()
                    .text_color(theme.muted_foreground)
                    .border_t_1()
                    .border_color(theme.border)
                    .child(
                        div()
                            .flex()
                            .gap_1()
                            .items_center()
                            .text_sm()
                            .text_color(theme.foreground)
                            .child("Minimize")
                            .child(
                                div().text_color(theme.muted_foreground).mb_1().child("␣")
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_1()
                            .items_center()
                            .text_sm()
                            .text_color(theme.foreground)
                            .child("Quit")
                            .child(
                                div().text_color(theme.muted_foreground).mb_0p5().child("⎋")
                            ),
                    ),
            )
    }
}

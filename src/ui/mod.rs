pub mod icon;
pub mod input;
pub mod list;

use gpui::{App, AppContext, ClickEvent, Context, Entity, InteractiveElement, IntoElement, ParentElement, Render, StatefulInteractiveElement, Styled, Window, div, prelude, px};
use input::{SearchQuery, TextInput};
use prelude::FluentBuilder;
use macos_accessibility_client::accessibility::application_is_trusted_with_prompt;

use crate::{applications::Applications, theme::Theme, ui::list::List};

pub struct Container {
    pub input: Entity<TextInput>,
    pub list: Entity<List>,
    pub trusted: bool
}

pub static LIST_ITEM_HEIGHT: f32 = 40.;
pub static INPUT_HEIGHT: f32 = 51.;
pub static ACTION_BAR_HEIGHT: f32 = 36.;
pub static EMPTY_PLACEHOLDER_HEIGHT: f32 = 100.;

impl Container {
    pub fn new(window: &mut Window, cx: &mut App) -> Self {
        let input = cx.new(|cx| TextInput::new(window, cx));
        let list = cx.new(|cx| List::new(cx));

        let trusted = application_is_trusted_with_prompt();

        Self { input, list, trusted }
    }

    fn update_accessibility_permission(&mut self, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        if application_is_trusted_with_prompt() {
            if let Ok(current_exe) = std::env::current_exe() {
                cx.restart(Some(current_exe));
            } else {
                cx.restart(None);
            }
        }
    }

    pub fn get_height(&self, cx: &App) -> f32 {
        let applications = cx.global::<Applications>();
        let query = cx.global::<SearchQuery>().value.as_str();

        let list = List::filter(query, applications.list.clone());
        let max_items = list.len();

        if max_items > 0 && self.trusted {
            return (max_items as f32 * LIST_ITEM_HEIGHT) + INPUT_HEIGHT + ACTION_BAR_HEIGHT;
        }

        // Default height if no windows are found.
        EMPTY_PLACEHOLDER_HEIGHT + INPUT_HEIGHT + ACTION_BAR_HEIGHT
    }

    fn render_accessibility_prompt(
        &self,
        theme: &Theme,
        listener: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static
    ) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .flex_1()
            .gap_neg_1()
            .items_center()
            .justify_center()
            .text_sm()
            .text_color(theme.foreground)
            .child("Please grant accessibility permissions in your")
            .child("system settings for the app to function correctly.")
            .child(
                div()
                    .id("permissions")
                    .flex()
                    .text_color(theme.foreground)
                    .bg(theme.primary)
                    .hover(|style| {
                        let mut bg = theme.primary;
                        bg.fade_out(0.15);
                        style.bg(bg)
                    })
                    .active(|style| {
                        let mut bg = theme.primary;
                        bg.fade_out(0.25);
                        style.bg(bg)
                    })
                    .on_click(listener)
                    .text_sm()
                    .rounded_md()
                    .pt_px()
                    .px_2()
                    .mt_3()
                    .cursor_pointer()
                    .child("Check again")
            )
    }

    fn render_action_button(
        &self,
        theme: &Theme,
        label: impl Into<String>,
        shortcut: impl Into<String>
    ) -> impl IntoElement {
        div()
            .flex()
            .gap_1()
            .items_center()
            .text_sm()
            .text_color(theme.foreground)
            .child(label.into())
            .child(
                div()
                    .text_color(theme.muted_foreground)
                    .mb_0p5()
                    .child(shortcut.into())
            )
    }
}

impl Render for Container {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let height = self.get_height(cx);

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
            .when(self.trusted, |cx| {
                cx.child(self.list.clone())
            })
            .when(!self.trusted, |element| {
                element.child(self.render_accessibility_prompt(theme, cx.listener(Self::update_accessibility_permission)))
            })
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
                    .child(self.render_action_button(theme, "Hide", "␣"))
                    .child(self.render_action_button(theme, "Quit", "⎋")),
            )
    }
}

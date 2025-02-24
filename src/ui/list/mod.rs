use std::time::Duration;

use gpui::{div, img, percentage, prelude, px, uniform_list, Animation, AnimationExt, App, Context, Div, Element, InteractiveElement, IntoElement, ParentElement, Render, ScrollStrategy, Styled, Transformation, UniformListScrollHandle, Window};
use prelude::FluentBuilder;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

use crate::{applications::Applications, theme::Theme};
use crate::socket_message::App as Application;
use super::icon::{IconColor, IconSize};
use super::{icon::{Icon, IconName}, input::SearchQuery};

pub struct List {
    pub items: Vec<Application>,
    list: UniformListScrollHandle,
}

impl List {
    pub fn new(cx: &App) -> Self {
        let applications = cx.global::<Applications>();

        Self {
            items: applications.list.clone(),
            list: UniformListScrollHandle::new(),
        }
    }

    pub fn filter(query: &str, mut list: Vec<Application>) -> Vec<Application> {
        let matcher = SkimMatcherV2::default();

        if query.is_empty() {
            list.retain(|item| item.pid != 0);
        } else {
            list.retain(|item| {
                matcher.fuzzy_match(&item.name, query).is_some()
            });

            list.sort_by(|a, b| {
                let score_a = matcher.fuzzy_match(&a.name, query).unwrap_or(0);
                let score_b = matcher.fuzzy_match(&b.name, query).unwrap_or(0);
                score_b.cmp(&score_a)
            });
        }

        list.sort_by(|a, b| {
            if a.pid != 0 && b.pid == 0 {
                std::cmp::Ordering::Less
            } else if a.pid == 0 && b.pid != 0 {
                std::cmp::Ordering::Greater
            } else if a.pid == 0 && b.pid == 0 {
                match (a.path.starts_with("/Applications/"), b.path.starts_with("/Applications/")) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => std::cmp::Ordering::Equal,
                }
            } else {
                std::cmp::Ordering::Equal
            }
        });

        // Limit the list to only have max 3 items with no pid
        let mut no_pid_count = 0;
        list.retain(|item| {
            if item.pid == 0 {
                no_pid_count += 1;
                no_pid_count <= 3
            } else {
                true
            }
        });

        list
    }

    fn render_empty_state(&self, theme: &Theme, child:  impl IntoElement) -> Div {
        div()
            .flex()
            .h_full()
            .items_center()
            .justify_center()
            .text_sm()
            .text_color(theme.muted_foreground)
            .child(child)
    }
}

impl Render for List {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let applications = cx.global::<Applications>();
        let query = cx.global::<SearchQuery>().value.as_str();

        let index = applications.index;
        let loading = applications.loading;
        self.list.scroll_to_item(index, ScrollStrategy::Top);

        if loading {
            return self.render_empty_state(
                theme,
                Icon::new(IconName::ArrowCircle, IconSize::Default, IconColor::Default).with_animation(
                    "arrow-circle",
                    Animation::new(Duration::from_secs(2)).repeat(),
                    |icon, delta| icon.transform(Transformation::rotate(percentage(delta))),
                )
            ).into_any();
        }

        // Update the list with the filtered applications.
        self.items = Self::filter(query, applications.list.clone());
        let scroll_handle = self.list.clone();

        if !self.items.is_empty() {
            div().child(uniform_list(cx.entity().clone(), "entries", self.items.len(), {
                let list = self.items.clone();
                move |_this, range, _window, cx| {
                    let theme = cx.global::<Theme>();

                    range.map(|i| {
                        let name = list[i].name.to_string();
                        let icon = list[i].icon.to_string();
                        let pid = list[i].pid;

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
                            .when(i == index, |cx| cx.bg(theme.muted))
                            .child(img(icon).h(px(32.0)).w(px(32.0)))
                            .child(div().child(name).flex_1())
                            .when(pid == 0, |cx| cx.child(div().mr_0p5().child(Icon::new(IconName::ExternalLink, IconSize::Small, IconColor::Muted))))
                            .when(pid == 0, |cx| cx.opacity(0.6))
                    }).collect::<Vec<_>>()
                }
            }).h_full().track_scroll(scroll_handle)).h_full()
        } else {
            self.render_empty_state(theme, "No applications found")
        }.into_any()
    }
}

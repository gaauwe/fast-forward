use std::time::Duration;

use gpui::*;
use prelude::FluentBuilder;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

use crate::{applications::{Applications, App}, theme::Theme};
use super::{icon::{Icon, IconName}, input::SearchQuery};

pub struct List {
    matcher: SkimMatcherV2,
    pub items: Vec<App>,
}

impl List {
    pub fn new(cx: &AppContext) -> Self {
        let matcher = SkimMatcherV2::default();
        let applications = cx.global::<Applications>();

        Self {
            matcher,
            items: applications.list.clone()
        }
    }

    pub fn filter(&self, query: &str, mut list: Vec<App>) -> Vec<App> {
        list.retain(|item| {
            self.matcher.fuzzy_match(&item.name, query).is_some()
        });

        list.sort_by(|a, b| {
            let score_a = self.matcher.fuzzy_match(&a.name, query).unwrap_or(0);
            let score_b = self.matcher.fuzzy_match(&b.name, query).unwrap_or(0);
            score_b.cmp(&score_a)
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
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let applications = cx.global::<Applications>();
        let query = cx.global::<SearchQuery>().value.as_str();

        let index = applications.index;
        let loading = applications.loading;

        if loading {
            return self.render_empty_state(
                theme,
                Icon::new(IconName::ArrowCircle).with_animation(
                    "arrow-circle",
                    Animation::new(Duration::from_secs(2)).repeat(),
                    |icon, delta| icon.transform(Transformation::rotate(percentage(delta))),
                )
            ).into_any();
        }

        // Update the list with the filtered applications.
        self.items = self.filter(query, applications.list.clone());

        if self.items.len() > 0 {
            div().child(uniform_list(cx.view().clone(), "entries", self.items.len(), {
                let list = self.items.clone();
                move |_this, range, cx| {
                    let theme = cx.global::<Theme>();

                    range.map(|i| {
                        let name = list[i].name.to_string();
                        let icon = list[i].icon.clone();

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
                            .child(div().child(name))
                    }).collect::<Vec<_>>()
                }
            }).h_full()).h_full()
        } else {
            self.render_empty_state(theme, "No applications found")
        }.into_any()
    }
}

use gpui::*;
use std::{path::PathBuf, time::Duration};
use swift_rs::{swift, Bool, Int, SRObjectArray, SRString};

use crate::ui::input::TextInput;
use crate::window::Window;

pub struct Applications {
    pub windows: Vec<App>,
    pub filtered_windows: Vec<App>,
    pub active_index: usize,
}

pub struct App {
    pub name: String,
    pub pid: isize,
    pub icon_path: PathBuf,
}

impl Clone for App {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            pid: self.pid,
            icon_path: self.icon_path.clone(),
        }
    }
}

#[repr(C)]
struct Application {
    pub name: SRString,
    pub path: SRString,
    pub pid: Int,
}

pub enum MoveType {
    Top,
    Bottom,
    Away
}

pub enum IndexType {
    Start,
    Next,
    Previous
}

swift!(fn get_active_app() -> SRString);
swift!(fn get_application_windows() -> SRObjectArray<Application>);
swift!(fn fire_window_event(application_name: &Int, action: SRString) -> Bool);

impl Applications {
    pub fn new(cx: &mut AppContext) {
        let windows = Self::get_application_windows();
        let filtered_windows = windows.clone();
        let applications = Self { windows, filtered_windows, active_index: 0 };
        cx.set_global(applications);

        Self::subscribe_to_active_app(cx);
    }

    pub fn set_active_index(cx: &mut AppContext, index_type: IndexType) {
        let this = cx.global::<Applications>();
        let mut active_index = this.active_index;

        match index_type {
            IndexType::Start => active_index = 0,
            IndexType::Next => active_index = (active_index + 1) % this.filtered_windows.len(),
            IndexType::Previous => {
                if active_index == 0 {
                    active_index = this.filtered_windows.len() - 1;
                } else {
                    active_index -= 1;
                }
            }
        }

        let applications = Self {
            windows: this.windows.clone(),
            filtered_windows: this.filtered_windows.clone(),
            active_index
        };
        cx.set_global(applications);
    }

    pub fn get_active_application(cx: &mut AppContext) -> Option<&App> {
        let applications = cx.global::<Applications>();
        applications.filtered_windows.get(applications.active_index)
    }

    pub fn fire_event(cx: &mut AppContext, action: &str) {
        let applications = cx.global::<Applications>();
        if applications.filtered_windows.is_empty() {
            return;
        }

        let app_pid = applications.filtered_windows[applications.active_index].pid;
        unsafe {
            fire_window_event(&app_pid, SRString::from(action));
        }
    }

    pub fn filter_applications(cx: &mut ViewContext<TextInput>, input: &str) {
        let applications = cx.global::<Applications>();
        let mut applications = applications.clone();
        if !input.is_empty() {
            use fuzzy_matcher::skim::SkimMatcherV2;
            use fuzzy_matcher::FuzzyMatcher;
            let matcher = SkimMatcherV2::default();

            applications.filtered_windows = applications.windows.clone();
            applications.filtered_windows.retain(|window| {
                matcher.fuzzy_match(&window.name, input).is_some()
            });

            applications.filtered_windows.sort_by(|a, b| {
                let score_a = matcher.fuzzy_match(&a.name, input).unwrap_or(0);
                let score_b = matcher.fuzzy_match(&b.name, input).unwrap_or(0);
                score_b.cmp(&score_a)
            });
        } else {
            applications.filtered_windows = applications.windows.clone();
        }

        // Reset the active index after filtering.
        applications.active_index = 0;
        cx.set_global(applications);
    }

    fn get_application_windows() -> Vec<App> {
        let applications = unsafe { get_application_windows() };
        applications
            .into_iter()
            .fold(Vec::new(), |mut acc, item| {
                acc.push(App {
                    name: item.name.to_string(),
                    pid: item.pid,
                    icon_path: PathBuf::from(item.path.to_string()),
                });
                acc
            })
    }

    // TODO:
    // - Constantly polling for the active application is not ideal.
    // - When app is minimized, move it down the list.
    // - When app is closed, remove it from the list.
    fn subscribe_to_active_app(cx: &mut AppContext) {
        cx.spawn(|cx| async move {
            let mut last_active_app = String::new();
            loop {
                let windows = cx.update(|cx| cx.windows());
                match windows {
                    Ok(windows) => {
                        // Never re-order the list if our application is in the foreground.
                        if windows.is_empty() {
                            let active_app = unsafe { get_active_app() }.to_string();
                            if active_app != last_active_app {
                                last_active_app = active_app.clone();

                                // Re-order the applications list to put the active application first.
                                let _ = cx.update(|cx| {
                                    Self::move_app(cx, Some(active_app.as_str()), MoveType::Top);
                                });
                            }
                        }
                    },
                    Err(_) => {}
                }

                cx.background_executor()
                    .timer(Duration::from_millis(500))
                    .await;
            }
        }).detach();
    }

    pub fn move_app(cx: &mut AppContext, app_name: Option<&str>, move_type: MoveType) {
        let applications = cx.global::<Applications>();
        let mut applications = applications.clone();

        // If no app name is provided, move the active application.
        let app_name = app_name.unwrap_or_else(|| {
            let active_app = Self::get_active_application(cx);
            active_app.map(|app| app.name.as_str()).unwrap_or("")
        });

        let app_index = applications.windows.iter().position(|window| window.name == app_name);
        if let Some(index) = app_index {
            let app = applications.windows.remove(index);

            match move_type {
                MoveType::Top => applications.windows.insert(0, app),
                MoveType::Bottom => applications.windows.push(app),
                MoveType::Away => {}
            }
        }

        applications.filtered_windows = applications.windows.clone();
        cx.set_global(applications);

        // Re-render the list after the order has changed.
        let window = cx.global::<Window>();
        window.window.clone().update(cx, |_view, cx| cx.notify()).ok();
    }
}

impl Global for Applications {}

impl Clone for Applications {
    fn clone(&self) -> Self {
        Self {
            windows: self.windows.clone(),
            filtered_windows: self.filtered_windows.clone(),
            active_index: self.active_index,
        }
    }
}

use gpui::*;
use std::{path::PathBuf, time::Duration};
use swift_rs::{swift, Bool, Int, SRObjectArray, SRString};

use crate::ui::input::TextInput;

pub struct Applications {
    pub windows: Vec<Window>,
    pub filtered_windows: Vec<Window>,
    pub active_index: usize,
}

pub struct Window {
    pub name: String,
    pub pid: isize,
    pub icon_path: PathBuf,
}

impl Clone for Window {
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

    pub fn next(cx: &mut AppContext) {
        let this = cx.global::<Applications>();
        let mut active_index = this.active_index;

        if active_index < this.windows.len() - 1 {
            active_index += 1;
        } else {
            active_index = 0;
        }

        let applications = Self {
            windows: this.windows.clone(),
            filtered_windows: this.filtered_windows.clone(),
            active_index
        };
        cx.set_global(applications);
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

    fn get_application_windows() -> Vec<Window> {
        let applications = unsafe { get_application_windows() };
        applications
            .into_iter()
            .fold(Vec::new(), |mut acc, item| {
                acc.push(Window {
                    name: item.name.to_string(),
                    pid: item.pid,
                    icon_path: PathBuf::from(item.path.to_string()),
                });
                acc
            })
    }

    // TODO: Constantly polling for the active application is not ideal.
    fn subscribe_to_active_app(cx: &mut AppContext) {
        cx.spawn(|cx| async move {
            let mut last_active_app = String::new();
            loop {
                let windows = cx.update(|cx| cx.windows());
                match windows {
                    Ok(windows) => {
                        if windows.is_empty() {
                            let active_app = unsafe { get_active_app() }.to_string();
                            let test = active_app.clone();
                            if active_app != last_active_app {
                                last_active_app = active_app;

                                // Re-order the applications list to put the active application first.
                                let _ = cx.update(|cx| {
                                    let applications = cx.global::<Applications>();
                                    let mut applications = applications.clone();

                                    applications.windows.sort_by(|a, b| {
                                        if a.name == test {
                                            std::cmp::Ordering::Less
                                        } else if b.name == test {
                                            std::cmp::Ordering::Greater
                                        } else {
                                            std::cmp::Ordering::Equal
                                        }
                                    });

                                    applications.filtered_windows = applications.windows.clone();
                                    applications.active_index = 0;

                                    cx.set_global(applications);
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

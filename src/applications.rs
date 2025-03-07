use std::process::Command;

use gpui::{App, Global};
use log::error;
use objc2::rc::Retained;
use objc2_app_kit::{NSApplicationActivationOptions, NSRunningApplication, NSWorkspace};

use crate::ui::input::SearchQuery;
use crate::window::Window;
use crate::socket_message::App as Application;
use crate::ui::list::List;

#[derive(Debug, Clone)]
pub struct Applications {
    pub list: Vec<Application>,
    pub index: usize,
    pub loading: bool,
}

impl Default for Applications {
    fn default() -> Self {
        Self {
            list: Vec::new(),
            index: 0,
            loading: true
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ActionType {
    Activate,
    Hide,
    Quit
}

#[derive(Debug, Clone, Copy)]
pub enum IndexType {
    Start,
    End,
    Next,
    Previous,
}

impl Applications {
    pub fn new(cx: &mut App) {
        cx.set_global(Self::default());
    }

    pub fn update_list(cx: &mut App, list: Vec<Application>) {
        cx.set_global(Self {
            list,
            index: 0,
            loading: false
        });
    }

    pub fn update_active_index(cx: &mut App, index_type: IndexType) {
        let query = cx.global::<SearchQuery>();
        let applications = cx.global::<Applications>();

        let mut applications = applications.clone();
        let list = List::filter(&query.value, applications.list.clone());

        applications.index = Self::get_index_from_type(&list, applications.index, index_type);
        cx.set_global(applications);
    }

    pub fn update_list_entry(cx: &mut App, app: Option<&Application>, index_type: Option<IndexType>, reset: bool) {
        let applications = cx.global::<Applications>();
        let mut applications = applications.clone();

        if let Some(app) = app {
            if let Some(existing_app_index) = applications.list.iter().position(|a| a.name == app.name) {
                if index_type.is_some() {
                    applications.list.remove(existing_app_index);
                } else {
                    applications.list[existing_app_index].pid = 0;
                }
            }

            if let Some(index_type) = index_type {
                let target_index = Self::get_index_from_type(&applications.list, applications.index, index_type);
                applications.list.insert(target_index, app.clone());
            }
        }

        cx.set_global(applications);

        // Reset the active index and search query.
        if reset {
            Self::update_active_index(cx, IndexType::Start);
            cx.set_global(SearchQuery { value: String::new() });
        }
    }

    pub fn execute_action(cx: &mut App, action_type: ActionType) {
        let applications = cx.global::<Applications>().clone();
        let window = cx.global::<Window>().window;

        let list = window.read(cx).unwrap().list.read(cx).items.clone();
        if list.is_empty() {
            return;
        }

        if let Some(app) = list.get(applications.index) {
            let native_app = Self::get_running_app_instance(app);
            if let Some(native_app) = native_app {
                match action_type {
                    ActionType::Activate => {
                        Self::update_list_entry(cx, Some(app), Some(IndexType::Start), false);

                        unsafe {
                            native_app.activateWithOptions(NSApplicationActivationOptions::empty());
                        }
                    },
                    ActionType::Hide => {
                        Self::update_list_entry(cx, Some(app), Some(IndexType::End), true);

                        unsafe {
                            native_app.hide();
                        }
                    },
                    ActionType::Quit => {
                        Self::update_list_entry(cx, Some(app), None, true);

                        unsafe {
                            native_app.terminate();
                        }
                    }
                }
            } else {
                // Handle events for non-running applications.
                if let ActionType::Activate = action_type {
                    Self::update_list_entry(cx, Some(app), Some(IndexType::Start), true);

                    if let Err(err) = Command::new("open").arg(&app.path).status() {
                        error!("Failed to open application at {}: {}", app.path, err);
                    }
                }
            }
        }
    }

    fn get_index_from_type(list: &[Application], current_index: usize, index_type: IndexType) -> usize {
        match index_type {
            IndexType::Start => 0,
            IndexType::End => list.len(),
            IndexType::Next => (current_index + 1) % list.len(),
            IndexType::Previous => {
                if current_index == 0 {
                    list.len() - 1
                } else {
                    current_index - 1
                }
            },
        }
    }

    fn get_running_app_instance(app: &Application) -> Option<Retained<NSRunningApplication>> {
        unsafe {
            let running_applications = NSWorkspace::sharedWorkspace().runningApplications();
            running_applications.iter().find(|item| item.localizedName().unwrap().to_string() == app.name)
        }
    }
}

impl Global for Applications {}

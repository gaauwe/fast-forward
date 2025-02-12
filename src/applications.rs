use std::process::Command;

use gpui::{App, Global};
use objc2::rc::Retained;
use objc2_app_kit::{NSApplicationActivationOptions, NSRunningApplication, NSWorkspace};

use crate::ui::input::SearchQuery;
use crate::window::Window;
use crate::socket_message::App as Application;
use crate::ui::list::List;

pub struct Applications {
    pub list: Vec<Application>,
    pub index: usize,
    pub loading: bool,
}

pub enum ActionType {
    Activate,
    Hide,
    Quit
}

pub enum IndexType {
    Start,
    End,
    Next,
    Previous,
}

impl Applications {
    pub fn new(cx: &mut App) {
        let list = Vec::new();
        let applications = Self { list, index: 0, loading: true };
        cx.set_global(applications);
    }

    pub fn update_list(cx: &mut App, list: Vec<Application>) {
        let applications = Self { list, index: 0, loading: false };
        cx.set_global(applications);
    }

    pub fn update_active_index(cx: &mut App, index_type: IndexType) {
        let query = cx.global::<SearchQuery>();
        let applications = cx.global::<Applications>();

        let mut applications = applications.clone();
        let list = List::filter(&query.value, applications.list.clone());

        applications.index = Self::get_index_from_type(&list, applications.index, index_type);
        cx.set_global(applications);
    }

    pub fn update_list_entry(cx: &mut App, app: Option<&Application>, index_type: Option<IndexType>) {
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

        // Re-render the list after the order has changed.
        let window = cx.global::<Window>();
        window.window.clone().update(cx, |_view, _window, cx| cx.notify()).ok();
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
                        Self::update_list_entry(cx, Some(app), Some(IndexType::Start));

                        unsafe {
                            native_app.activateWithOptions(NSApplicationActivationOptions::empty());
                        }
                    },
                    ActionType::Hide => {
                        Self::update_list_entry(cx, Some(app), Some(IndexType::End));

                        unsafe {
                            native_app.hide();
                        }
                    },
                    ActionType::Quit => {
                        Self::update_list_entry(cx, Some(app), None);

                        unsafe {
                            native_app.terminate();
                        }
                    }
                }
            }

            // Handle events for non-running applications.
            if let ActionType::Activate = action_type {
                Self::update_list_entry(cx, Some(app), Some(IndexType::Start));

                let _ = Command::new("open").arg(app.path.as_str()).status();
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

impl Clone for Applications {
    fn clone(&self) -> Self {
        Self {
            list: self.list.clone(),
            index: self.index,
            loading: self.loading
        }
    }
}

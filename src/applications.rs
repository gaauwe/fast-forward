use gpui::*;
use objc2::rc::Retained;
use objc2_app_kit::{NSApplicationActivationOptions, NSRunningApplication, NSWorkspace};
use std::path::PathBuf;

use crate::window::Window;

pub struct App {
    pub pid: isize,
    pub name: String,
    pub icon: PathBuf,
    pub active: bool,
}

impl Clone for App {
    fn clone(&self) -> Self {
        Self {
            pid: self.pid,
            name: self.name.clone(),
            icon: self.icon.clone(),
            active: self.active
        }
    }
}

pub struct Applications {
    pub list: Vec<App>,
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
    pub fn new(cx: &mut AppContext) {
        let list = Vec::new();
        let applications = Self { list, index: 0, loading: true };
        cx.set_global(applications);
    }

    pub fn update_list(cx: &mut AppContext, list: Vec<App>) {
        let applications = Self { list, index: 0, loading: false };
        cx.set_global(applications);
    }

    pub fn update_active_index(cx: &mut AppContext, index_type: IndexType) {
        let applications = cx.global::<Applications>();
        let mut applications = applications.clone();

        applications.index = Self::get_index_from_type(&applications.list, applications.index, index_type);
        cx.set_global(applications);
    }

    pub fn update_list_entry(cx: &mut AppContext, app: Option<&App>, index_type: Option<IndexType>) {
        let applications = cx.global::<Applications>();
        let mut applications = applications.clone();

        if let Some(app) = app {
            if let Some(existing_app_index) = applications.list.iter().position(|a| a.name == app.name) {
                applications.list.remove(existing_app_index);
            }

            match index_type {
                Some(index_type) => {
                    let target_index = Self::get_index_from_type(&applications.list, applications.index, index_type);
                    applications.list.insert(target_index, app.clone());
                },
                None => {}
            }
        }

        cx.set_global(applications);

        // Re-render the list after the order has changed.
        let window = cx.global::<Window>();
        window.window.clone().update(cx, |_view, cx| cx.notify()).ok();
    }

    pub fn execute_action(cx: &mut AppContext, action_type: ActionType) {
        let applications = cx.global::<Applications>().clone();
        let window = cx.global::<Window>().window;

        let list = window.read(cx).unwrap().list.read(cx).items.clone();
        if list.is_empty() {
            return;
        }

        if let Some(app) = list.get(applications.index) {
            let native_app = Self::get_native_app_instance(app);
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
        }
    }

    fn get_index_from_type(list: &Vec<App>, current_index: usize, index_type: IndexType) -> usize {
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

    fn get_native_app_instance(app: &App) -> Option<Retained<NSRunningApplication>> {
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

mod applications;
mod commander;
mod config;
mod hotkey;
mod logger;
mod theme;
mod socket;
mod tray;
mod ui;
mod window;

mod socket_message {
    include!(concat!(env!("OUT_DIR"), "/_.rs"));
}

use std::fs;
use std::path::PathBuf;

use applications::Applications;
use commander::Commander;
use config::Config;
use hotkey::Hotkey;
use logger::Logger;
use macos_accessibility_client::accessibility::application_is_trusted_with_prompt;
use theme::Theme;
use socket::Socket;
use tray::Tray;
use window::Window;
use cocoa::appkit::NSApplication;
use cocoa::appkit::NSApplicationActivationPolicy;
use cocoa::base::nil;
use gpui::*;

#[tokio::main]
async fn main() {
    App::new()
        .with_assets(Assets {
            base: if cfg!(debug_assertions) {
                PathBuf::from("assets")
            } else {
                std::env::current_exe().unwrap().parent().unwrap().join("assets")
            },
        })
        .run(|cx: &mut AppContext| {
        // Start the application in accessory mode, which means it won't appear in the dock.
        // - https://developer.apple.com/documentation/appkit/nsapplication/activationpolicy-swift.enum/accessory
        unsafe {
            let ns_app = NSApplication::sharedApplication(nil);
            ns_app.setActivationPolicy_(NSApplicationActivationPolicy::NSApplicationActivationPolicyAccessory);
        }

        // Initialize the application components.
        Logger::new();
        Commander::new(cx);
        Tray::new(cx);
        Applications::new(cx);
        Config::new(cx);
        Theme::new(cx);
        Window::new(cx);

        // Check if the application is trusted to access the accessibility API.
        if application_is_trusted_with_prompt() {
            Socket::new(cx);
            Hotkey::new(cx);
        }
    });
}

struct Assets {
    base: PathBuf,
}

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<std::borrow::Cow<'static, [u8]>>> {
        fs::read(self.base.join(path))
            .map(|data| Some(std::borrow::Cow::Owned(data)))
            .map_err(|err| err.into())
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        fs::read_dir(self.base.join(path))
            .map(|entries| {
                entries
                    .filter_map(|entry| {
                        entry
                            .ok()
                            .and_then(|entry| entry.file_name().into_string().ok())
                            .map(SharedString::from)
                    })
                    .collect()
            })
            .map_err(|err| err.into())
    }
}

#![allow(clippy::new_ret_no_self)]
mod applications;
mod assets;
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

use applications::Applications;
use assets::Assets;
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
use gpui::{App, Application};

#[tokio::main]
async fn main() {
    Application::new()
        .with_assets(Assets)
        .run(|cx: &mut App| {
        // Start the application in accessory mode, which means it won't appear in the dock.
        // - https://developer.apple.com/documentation/appkit/nsapplication/activationpolicy-swift.enum/accessory
        unsafe {
            let ns_app = NSApplication::sharedApplication(nil);
            ns_app.setActivationPolicy_(NSApplicationActivationPolicy::NSApplicationActivationPolicyAccessory);
        }

        // Initialize the application components.
        Logger::new();
        Commander::new(cx);

        // Load the configuration (and initialize the tray).
        let config = Config::new(cx);
        if config.general.show_tray {
            Tray::new(cx);
        }

        // Start the application.
        Applications::new(cx);
        Theme::new(cx);
        Window::new(cx);

        // Check if the application is trusted to access the accessibility API.
        if application_is_trusted_with_prompt() {
            Socket::new(cx);
            Hotkey::new(cx);
        }
    });
}

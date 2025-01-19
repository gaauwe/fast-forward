mod applications;
mod components;
mod config;
mod hotkey;
mod theme;
mod tray;
mod window;

use applications::Applications;
use config::Config;
use hotkey::HotkeyManager;
use theme::Theme;
use tray::Tray;
use window::Window;

use cocoa::appkit::NSApplication;
use cocoa::appkit::NSApplicationActivationPolicy;
use cocoa::base::nil;

use gpui::*;

fn main() {
    App::new().run(|cx: &mut AppContext| {
        // Start the application in accessory mode, which means it won't appear in the dock.
        // - https://developer.apple.com/documentation/appkit/nsapplication/activationpolicy-swift.enum/accessory
        unsafe {
            let ns_app = NSApplication::sharedApplication(nil);
            ns_app.setActivationPolicy_(NSApplicationActivationPolicy::NSApplicationActivationPolicyAccessory);
        }

        // Create tray icon.
        Tray::new(cx);

        // Create the global hotkey manager.
        HotkeyManager::new(cx);

        // Load configuration.
        Config::new(cx);

        // Initialize the theme.
        Theme::new(cx);

        // Initialize the list of open application windows.
        Applications::new(cx);

        // Initialize the window.
        Window::new(cx);
    });
}

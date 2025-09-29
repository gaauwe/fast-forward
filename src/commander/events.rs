use crate::socket_message::socket_message::Event as SocketEvent;

#[allow(clippy::enum_variant_names)]
pub enum EventType {
    HotkeyEvent(HotkeyEvent),
    TrayEvent(TrayEvent),
    SocketEvent(SocketEvent),
}

#[derive(Debug, Clone, Copy)]
pub enum HotkeyEvent {
    ShowWindow(usize),
    HideWindow,
    HideApplication,
    QuitApplication,
}

#[derive(Debug, Clone, Copy)]
pub enum TrayEvent {
    Settings,
    About,
    Quit,
}

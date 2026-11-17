use std::sync::{mpsc::Sender, Arc, Mutex};

use windows_sys::Win32::{
    Foundation::{HMODULE, HWND},
    UI::{
        Shell::{NIF_ICON, NIF_TIP, NOTIFYICONDATAW},
        WindowsAndMessaging::{HICON, HMENU},
    },
};

#[derive(Clone)]
pub(crate) struct WindowInfo {
    pub hwnd: HWND,
    pub hmodule: HMODULE,
    pub hmenu: HMENU,
    /// Complete desired appearance, shared with the window message thread.
    pub notification: Arc<Mutex<NOTIFYICONDATAW>>,
    pub taskbar_created: u32,
}

unsafe impl Send for WindowInfo {}
unsafe impl Sync for WindowInfo {}

impl WindowInfo {
    pub fn with_tooltip(&self, tooltip: [u16; 128]) -> NOTIFYICONDATAW {
        let mut saved = self.notification.lock().unwrap();
        saved.szTip = tooltip;
        saved.uFlags |= NIF_TIP;
        *saved
    }

    pub fn with_icon(&self, icon: HICON) -> NOTIFYICONDATAW {
        let mut saved = self.notification.lock().unwrap();
        saved.hIcon = icon;
        saved.uFlags |= NIF_ICON;
        *saved
    }

    pub fn notification_for_message(&self, message: u32) -> Option<NOTIFYICONDATAW> {
        (self.taskbar_created != 0 && message == self.taskbar_created)
            .then(|| *self.notification.lock().unwrap())
    }
}

#[derive(Clone)]
pub(crate) struct WindowsLoopData {
    pub info: WindowInfo,
    pub tx: Sender<WindowsTrayEvent>,
}

pub(crate) struct WindowsTrayEvent(pub(crate) u32);

#[cfg(test)]
mod tests {
    use super::*;
    use windows_sys::Win32::UI::{Shell::NIF_MESSAGE, WindowsAndMessaging::WM_USER};

    fn info() -> WindowInfo {
        let mut notification: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
        notification.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        notification.hWnd = 123;
        notification.uID = 1;
        notification.uFlags = NIF_MESSAGE;
        notification.uCallbackMessage = WM_USER + 1;
        WindowInfo {
            hwnd: 123,
            hmodule: 0,
            hmenu: 0,
            notification: Arc::new(Mutex::new(notification)),
            taskbar_created: 0xC001,
        }
    }

    #[test]
    fn restores_configured_icon_tooltip_and_callback_repeatedly() {
        let info = info();
        let mut tip = [0u16; 128];
        let text: Vec<u16> = "Корзина 🗑".encode_utf16().collect();
        tip[..text.len()].copy_from_slice(&text);
        info.with_tooltip(tip);
        info.with_icon(456); // Caller-owned custom icon; no OS API call in this test.
        for _ in 0..3 {
            let restored = info.notification_for_message(0xC001).unwrap();
            assert_eq!(restored.hIcon, 456);
            assert_eq!(restored.szTip, tip);
            assert_eq!(restored.uFlags, NIF_MESSAGE | NIF_ICON | NIF_TIP);
            assert_eq!(restored.hWnd, 123);
            assert_eq!(restored.uID, 1);
            assert_eq!(restored.uCallbackMessage, WM_USER + 1);
        }
    }

    #[test]
    fn restores_latest_icon_and_clears_old_tooltip_suffix() {
        let info = info();
        info.with_icon(456);
        info.with_tooltip([65; 128]);
        let mut shorter = [0; 128];
        shorter[0] = 66;
        info.with_tooltip(shorter);
        info.with_icon(789); // Also covers the embedded fallback's cached handle.
        let restored = info.notification_for_message(0xC001).unwrap();
        assert_eq!(restored.hIcon, 789);
        assert_eq!(restored.szTip, shorter);
    }

    #[test]
    fn ignores_unrelated_and_unregistered_messages() {
        let mut info = info();
        assert!(info.notification_for_message(0).is_none());
        assert!(info.notification_for_message(WM_USER + 1).is_none());
        info.taskbar_created = 0;
        assert!(info.notification_for_message(0).is_none());
    }
}

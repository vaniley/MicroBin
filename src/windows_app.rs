use microbin::{load_config, Config};
use std::{os::windows::ffi::OsStrExt, path::Path, process::Command, sync::mpsc};
use tray_item::{IconSource, TrayItem};
use windows_sys::Win32::{
    UI::Shell::SHEmptyRecycleBinW,
    UI::WindowsAndMessaging::{
        DestroyIcon, LoadImageW, MessageBoxW, HICON, IMAGE_ICON, LR_LOADFROMFILE, MB_ICONERROR,
        MB_OK,
    },
};

fn wide(value: &std::ffi::OsStr) -> Vec<u16> {
    value.encode_wide().chain(Some(0)).collect()
}

pub fn report_error(message: &str) {
    let message = wide(std::ffi::OsStr::new(message));
    let title = wide(std::ffi::OsStr::new("MicroBin"));
    // Both buffers are NUL-terminated and live through the synchronous call.
    unsafe {
        MessageBoxW(0, message.as_ptr(), title.as_ptr(), MB_OK | MB_ICONERROR);
    }
}

struct CustomIcon(HICON);

impl CustomIcon {
    fn load(path: &Path) -> Result<Self, String> {
        let name = wide(path.as_os_str());
        // LR_LOADFROMFILE returns an owned icon; CustomIcon releases it after the tray.
        let handle = unsafe { LoadImageW(0, name.as_ptr(), IMAGE_ICON, 64, 64, LR_LOADFROMFILE) };
        if handle == 0 {
            Err(format!(
                "Cannot load icon {}: {}",
                path.display(),
                std::io::Error::last_os_error()
            ))
        } else {
            Ok(Self(handle))
        }
    }
}

impl Drop for CustomIcon {
    fn drop(&mut self) {
        unsafe {
            DestroyIcon(self.0);
        }
    }
}

#[derive(Clone, Copy)]
enum Message {
    Open,
    Empty,
    Quit,
}

pub fn run() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|error| error.to_string())?;
    let config_path = exe
        .parent()
        .ok_or("Cannot locate the executable directory")?
        .join("config.toml");
    let config = match load_config(&config_path) {
        Ok(config) => config,
        Err(error) => {
            report_error(&format!("Cannot load {}:\n{error}\n\nMicroBin will use defaults. The existing file is unchanged. Fix the file and restart MicroBin.", config_path.display()));
            Config::default()
        }
    };
    let custom_icon = config.icon_file(&config_path).and_then(|path| {
        match CustomIcon::load(&path) {
            Ok(icon) => Some(icon),
            Err(error) => {
                report_error(&format!("{error}\n\nMicroBin will use its built-in icon. Fix icon_path and restart MicroBin."));
                None
            }
        }
    });
    // Declare the tray after its icon so the tray is destroyed first.
    let icon = custom_icon
        .as_ref()
        .map_or(IconSource::Resource("empty"), |icon| {
            IconSource::RawIcon(icon.0)
        });
    let mut tray = TrayItem::new(&config.tray_label, icon).map_err(|error| error.to_string())?;
    // Never block the native menu callback thread while a Shell dialog is open.
    let (tx, rx) = mpsc::channel();
    for (label, message) in [
        (&config.open_label, Message::Open),
        (&config.empty_label, Message::Empty),
    ] {
        let tx = tx.clone();
        tray.add_menu_item(label, move || {
            let _ = tx.send(message);
        })
        .map_err(|error| error.to_string())?;
    }
    tray.inner_mut()
        .add_separator()
        .map_err(|error| error.to_string())?;
    tray.add_menu_item(&config.quit_label, move || {
        let _ = tx.send(Message::Quit);
    })
    .map_err(|error| error.to_string())?;
    while let Ok(message) = rx.recv() {
        match message {
            Message::Quit => break,
            Message::Open => {
                if let Err(error) = Command::new("explorer.exe")
                    .arg("shell:RecycleBinFolder")
                    .spawn()
                {
                    report_error(&format!("Cannot open the Recycle Bin: {error}"));
                }
            }
            Message::Empty => {
                // A null root path selects all drives. The Shell owns confirmation and progress.
                let result =
                    unsafe { SHEmptyRecycleBinW(0, std::ptr::null(), config.empty_flags()) };
                // User cancellation is not an application failure.
                if result < 0 && result != 0x800704C7u32 as i32 && result != 0x80004004u32 as i32 {
                    report_error(&format!(
                        "Cannot empty the Recycle Bin (HRESULT 0x{:08X}).",
                        result as u32
                    ));
                }
            }
        }
    }
    Ok(())
}

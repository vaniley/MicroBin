#[cfg(not(target_os = "windows"))]
compile_error!("MicroBin's local tray-item patch supports Windows only");

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "windows")]
pub type TrayItemImpl = windows::TrayItemWindows;

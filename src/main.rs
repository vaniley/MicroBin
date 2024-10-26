#![cfg_attr(windows, windows_subsystem = "windows")]

use std::fs;
use std::process::Command;
use std::sync::mpsc;
use serde::{Deserialize, Serialize};
use tray_item::{IconSource, TrayItem};
use windows::Win32::UI::Shell::SHEmptyRecycleBinW;
use windows::Win32::Foundation::HWND;
use windows::core::PCWSTR;

#[derive(Serialize, Deserialize, Debug)]
struct Config {
    tray_label: String,
    open_label: String,
    empty_label: String,
    quit_label: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            tray_label: "MicroBin".to_string(),
            open_label: "Open".to_string(),
            empty_label: "Empty".to_string(),
            quit_label: "Quit".to_string(),
        }
    }
}

enum Message {
    Quit,
    Open,
    Empty,
}

fn load_config() -> Config {
    let config_path = get_config_path();
    if let Ok(config_data) = fs::read_to_string(&config_path) {
        toml::from_str(&config_data).unwrap_or_default()
    } else {
        let default_config = Config::default();
        if let Ok(_) = fs::create_dir_all(config_path.parent().unwrap()) {
            let _ = fs::write(&config_path, toml::to_string_pretty(&default_config).unwrap());
        }
        default_config
    }
}

fn get_config_path() -> std::path::PathBuf {
    let exe_path = std::env::current_exe().unwrap();
    exe_path.parent().unwrap().join("config.toml")
}

fn open_recycle_bin() {
    let _ = Command::new("explorer.exe").arg(r"shell:RecycleBinFolder").spawn();
}

fn empty_recycle_bin() {
    unsafe {
        // Flags for silent empty recycle bin
        // SHERB_NOCONFIRMATION | SHERB_NOPROGRESSUI | SHERB_NOSOUND = 0x000000A4
        const SHERB_NOCONFIRMATION: u32 = 0x00000001;
        const SHERB_NOPROGRESSUI: u32 = 0x00000002;
        const SHERB_NOSOUND: u32 = 0x00000004;

        let flags = SHERB_NOCONFIRMATION | SHERB_NOPROGRESSUI | SHERB_NOSOUND;

        // Call the WinAPI function to empty the recycle bin
        let result = SHEmptyRecycleBinW(
            HWND(0),           // Null HWND for no parent window
            PCWSTR::null(),    // Null PCWSTR to empty all drives
            flags
        );
        
        if let Err(e) = result {
            eprintln!("Failed to empty recycle bin: {:?}", e);
        }
    }
}

fn main() {
    let config = load_config();
    let mut tray = TrayItem::new(&config.tray_label, IconSource::Resource("empty")).unwrap();

    let (tx, rx) = mpsc::sync_channel(1);

    let open_tx = tx.clone();
    let _ = tray.add_menu_item(&config.open_label, move || {
        open_tx.send(Message::Open).unwrap();
    });

    let empty_tx = tx.clone();
    let _ = tray.add_menu_item(&config.empty_label, move || {
        empty_tx.send(Message::Empty).unwrap();
    });

    tray.inner_mut().add_separator().unwrap();

    let quit_tx = tx.clone();
    let _ = tray.add_menu_item(&config.quit_label, move || {
        quit_tx.send(Message::Quit).unwrap();
    });

    loop {
        match rx.recv() {
            Ok(Message::Quit) => break,
            Ok(Message::Open) => open_recycle_bin(),
            Ok(Message::Empty) => empty_recycle_bin(),
            _ => {}
        }
    }
}

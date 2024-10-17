// src/main.rs
use std::process::Command;
use std::sync::mpsc;
use tray_item::{IconSource, TrayItem};

enum Message {
    Quit,
    Open,
    Empty,
}

fn open_recycle_bin() {
    Command::new("explorer.exe")
        .arg(r"shell:RecycleBinFolder")
        .spawn()
        .unwrap();
}

fn empty_recycle_bin() {
    Command::new("powershell.exe")
        .args(["-Command", "$RecycleBin = New-Object -ComObject Shell.Application; $RecycleBin.Namespace(0xA).Items() | Remove-Item -Force"])
        .spawn()
        .unwrap();
}

fn main() {
    let mut tray = TrayItem::new("Recycle Bin", IconSource::Resource("empty"))
        .unwrap();

    let (tx, rx) = mpsc::sync_channel(1);

    let open_tx = tx.clone();
    tray.add_menu_item("Open", move || {
        open_tx.send(Message::Open).unwrap();
    })
    .unwrap();

    let empty_tx = tx.clone();
    tray.add_menu_item("Empty", move || {
        empty_tx.send(Message::Empty).unwrap();
    })
    .unwrap();

    tray.inner_mut().add_separator().unwrap();

    let quit_tx = tx.clone();
    tray.add_menu_item("Quit", move || {
        quit_tx.send(Message::Quit).unwrap();
    })
    .unwrap();

    loop {
        match rx.recv() {
            Ok(Message::Quit) => {
                break;
            }
            Ok(Message::Open) => {
                open_recycle_bin();
            }
            Ok(Message::Empty) => {
                empty_recycle_bin();
            }
            _ => {}
        }
    }
}

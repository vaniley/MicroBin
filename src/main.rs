#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
mod windows_app;

fn main() {
    #[cfg(windows)]
    if let Err(error) = windows_app::run() {
        windows_app::report_error(&error);
        std::process::exit(1);
    }
    #[cfg(not(windows))]
    {
        eprintln!("MicroBin requires Windows. Configuration tests can run on this platform.");
        std::process::exit(1);
    }
}

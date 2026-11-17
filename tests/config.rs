use microbin::{load_config, Config};
use std::{
    fs,
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};

#[test]
fn defaults_preserve_existing_behavior() {
    let config = Config::parse("").unwrap();
    assert_eq!(config, Config::default());
    assert_eq!(config.empty_flags(), 7);
    assert!(config.icon_file(Path::new("config.toml")).is_none());
}

#[test]
fn legacy_and_partial_files_work() {
    let config = Config::parse("tray_label = 'My Bin'\nopen_label = 'Open bin'\nempty_label = 'Clean'\nquit_label = 'Exit'").unwrap();
    assert_eq!(config.tray_label, "My Bin");
    assert_eq!(config.empty_flags(), 7);
    let partial = Config::parse("open_label = 'Открыть'").unwrap();
    assert_eq!(partial.open_label, "Открыть");
    assert_eq!(partial.quit_label, "Quit");
}

#[test]
fn every_flag_combination_matches_shell_constants() {
    for bits in 0..8 {
        let config = Config {
            confirm_empty: bits & 1 != 0,
            show_progress: bits & 2 != 0,
            play_sound: bits & 4 != 0,
            ..Config::default()
        };
        assert_eq!(config.empty_flags(), (!bits) & 7);
    }
}

#[test]
fn invalid_configuration_has_actionable_errors() {
    for text in [
        "tray_label = 10",
        "not toml",
        "confirm_emty = true",
        "open_label = ''",
        "quit_label = '  '",
        "icon_path = \"\\u0000\"",
        "empty_label = \"a\\u0000b\"",
    ] {
        assert!(Config::parse(text).is_err(), "unexpectedly accepted {text}");
    }
    assert!(Config::parse(&format!("tray_label = '{}'", "😀".repeat(64))).is_err());
    assert!(Config::parse(&format!("tray_label = '{}'", "a".repeat(127))).is_ok());
}

#[test]
fn icon_paths_use_config_directory_not_working_directory() {
    let config = Config::parse("icon_path = 'icons/custom.ico'").unwrap();
    assert_eq!(
        config.icon_file(Path::new("settings/config.toml")).unwrap(),
        Path::new("settings/icons/custom.ico")
    );
    let absolute = std::env::current_dir().unwrap().join("custom.ico");
    let config = Config {
        icon_path: absolute.to_str().unwrap().into(),
        ..Config::default()
    };
    assert_eq!(
        config.icon_file(Path::new("settings/config.toml")).unwrap(),
        absolute
    );
}

struct Fixture(std::path::PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        // Avoid the OS temp directory: keep test fixtures inside the build directory.
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join(format!(
                "config-test-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
        fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
    fn path(&self) -> std::path::PathBuf {
        self.0.join("config.toml")
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn missing_config_is_created_and_round_trips() {
    let fixture = Fixture::new();
    assert_eq!(load_config(&fixture.path()).unwrap(), Config::default());
    assert_eq!(load_config(&fixture.path()).unwrap(), Config::default());
}

#[test]
fn bad_existing_file_is_not_overwritten() {
    let fixture = Fixture::new();
    for contents in [b"invalid = [".as_slice(), &[0xff, 0xfe]] {
        fs::write(fixture.path(), contents).unwrap();
        assert!(load_config(&fixture.path()).is_err());
        assert_eq!(fs::read(fixture.path()).unwrap(), contents);
    }
}

#[test]
fn inaccessible_path_returns_error_without_panicking() {
    let fixture = Fixture::new();
    assert!(load_config(&fixture.0).is_err());
    assert!(load_config(&fixture.0.join("missing/config.toml"))
        .unwrap_err()
        .starts_with("Cannot create configuration:"));
}

#[test]
fn empty_bin_and_cancel_are_not_failures() {
    for result in [
        0,
        1,
        0x800704C7u32 as i32,
        0x80004004u32 as i32,
        0x8000FFFFu32 as i32,
    ] {
        assert!(!microbin::empty_bin_failed(result));
    }
    assert!(microbin::empty_bin_failed(0x80070005u32 as i32));
    assert!(microbin::empty_bin_failed(0x80004005u32 as i32));
}

#[test]
fn example_is_valid_and_enables_confirmation() {
    let config = Config::parse(include_str!("../config.example.toml")).unwrap();
    assert!(config.confirm_empty);
}

#[cfg(unix)]
#[test]
fn missing_config_in_read_only_folder_uses_defaults_without_writing() {
    use std::os::unix::fs::PermissionsExt;
    let fixture = Fixture::new();
    fs::set_permissions(&fixture.0, fs::Permissions::from_mode(0o555)).unwrap();
    let result = load_config(&fixture.path());
    // Restore permissions before assertions so the fixture remains removable.
    fs::set_permissions(&fixture.0, fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(result.unwrap(), Config::default());
    assert!(!fixture.path().exists());
}

#[cfg(unix)]
#[test]
fn unreadable_existing_config_still_reports_error() {
    use std::os::unix::fs::PermissionsExt;
    let fixture = Fixture::new();
    let contents = "confirm_empty = true";
    fs::write(fixture.path(), contents).unwrap();
    fs::set_permissions(fixture.path(), fs::Permissions::from_mode(0o000)).unwrap();
    let result = load_config(&fixture.path());
    fs::set_permissions(fixture.path(), fs::Permissions::from_mode(0o600)).unwrap();
    assert!(result
        .unwrap_err()
        .starts_with("Cannot read configuration:"));
    assert_eq!(fs::read_to_string(fixture.path()).unwrap(), contents);
}

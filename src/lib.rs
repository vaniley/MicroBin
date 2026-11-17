use serde::{Deserialize, Serialize};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub tray_label: String,
    pub open_label: String,
    pub empty_label: String,
    pub quit_label: String,
    /// Empty uses the embedded icon. Relative paths use the configuration directory.
    pub icon_path: String,
    pub confirm_empty: bool,
    pub show_progress: bool,
    pub play_sound: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            tray_label: "MicroBin".into(),
            open_label: "Open".into(),
            empty_label: "Empty".into(),
            quit_label: "Quit".into(),
            icon_path: String::new(),
            confirm_empty: false,
            show_progress: false,
            play_sound: false,
        }
    }
}

impl Config {
    pub fn parse(text: &str) -> Result<Self, String> {
        let config: Self = toml::from_str(text).map_err(|error| error.to_string())?;
        for (name, value) in [
            ("tray_label", &config.tray_label),
            ("open_label", &config.open_label),
            ("empty_label", &config.empty_label),
            ("quit_label", &config.quit_label),
        ] {
            if value.trim().is_empty() || value.contains('\0') {
                return Err(format!(
                    "{name} must not be empty or contain a NUL character"
                ));
            }
        }
        // The tray library copies the tooltip into a fixed Windows buffer.
        if config.tray_label.encode_utf16().count() > 127 {
            return Err("tray_label must fit in 127 UTF-16 code units".into());
        }
        if config.icon_path.contains('\0') {
            return Err("icon_path must not contain a NUL character".into());
        }
        Ok(config)
    }

    pub fn icon_file(&self, config_path: &Path) -> Option<PathBuf> {
        if self.icon_path.is_empty() {
            return None;
        }
        let path = PathBuf::from(&self.icon_path);
        Some(if path.is_absolute() {
            path
        } else {
            config_path.parent().unwrap_or(Path::new(".")).join(path)
        })
    }

    /// SHERB_NOCONFIRMATION, SHERB_NOPROGRESSUI, SHERB_NOSOUND.
    pub fn empty_flags(&self) -> u32 {
        u32::from(!self.confirm_empty)
            | (u32::from(!self.show_progress) << 1)
            | (u32::from(!self.play_sound) << 2)
    }
}

/// Never replace unreadable or invalid user configuration.
pub fn load_config(path: &Path) -> Result<Config, String> {
    match fs::read_to_string(path) {
        Ok(text) => Config::parse(&text),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            let config = Config::default();
            let text = toml::to_string_pretty(&config).map_err(|error| error.to_string())?;
            // create_new prevents a concurrent instance from overwriting a newly created file.
            use std::io::Write;
            match fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
            {
                Ok(mut file) => file
                    .write_all(text.as_bytes())
                    .map_err(|error| error.to_string())?,
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                    return fs::read_to_string(path)
                        .map_err(|error| error.to_string())
                        .and_then(|text| Config::parse(&text));
                }
                Err(error) => return Err(error.to_string()),
            }
            Ok(config)
        }
        Err(error) => Err(error.to_string()),
    }
}

# MicroBin

[Документация на русском](README.md)

MicroBin is a small Windows app that opens and empties the Recycle Bin from the system tray.

## Features

- Open the Recycle Bin in File Explorer.
- Empty the Recycle Bin on all drives.
- Customize the tray tooltip and menu labels.
- Use a custom `.ico` tray icon.
- Configure Windows confirmation, progress, and sound for empty operations.
- Show configuration and Windows operation errors.

## Run and customize

Build the app or download the `MicroBin-windows-x64` artifact from a successful GitHub Actions run.
Extract the app into a writable directory. Run `MicroBin.exe`.
The app creates `config.toml` beside the executable on first launch.

Edit `config.toml`. Restart MicroBin to apply changes.
You can also copy [config.example.toml](config.example.toml) to `config.toml`.
The example enables confirmation and progress.

| Option | Default | Purpose |
| --- | --- | --- |
| `tray_label` | `"MicroBin"` | Tray tooltip; at most 127 UTF-16 code units |
| `open_label` | `"Open"` | Open menu label |
| `empty_label` | `"Empty"` | Empty menu label |
| `quit_label` | `"Quit"` | Quit menu label |
| `icon_path` | `""` | `.ico` path; empty uses the embedded icon |
| `confirm_empty` | `false` | Ask Windows to confirm deletion |
| `show_progress` | `false` | Allow Windows progress UI |
| `play_sound` | `false` | Allow the Windows empty sound |

**Warning:** defaults preserve the original silent empty behavior without confirmation, progress, or sound.
MicroBin cannot undo deletion. Set `confirm_empty = true` to request confirmation.
Windows controls whether progress appears and whether sound plays.

Relative icon paths use the `config.toml` directory, not the working directory.
Use TOML literal strings for Windows paths: `icon_path = 'C:\Icons\bin.ico'`.
The icon is static. It does not track whether the Recycle Bin is full.
MicroBin restores the configured icon and tooltip after Explorer restarts.

Existing four-label configurations still work. Missing options use defaults.
Unknown options, wrong types, blank labels, and oversized tooltips produce an error dialog.
The app uses defaults and leaves existing configuration files unchanged.
If a custom icon fails to load, the app uses its embedded icon.
Fix the file and restart MicroBin.

If configuration is missing and the folder denies creation, MicroBin uses defaults without a startup warning.
Use a writable directory to save custom settings. Existing unreadable or invalid files still produce an error.
MicroBin never replaces existing configuration files. Updates do not rewrite existing settings.

## Development

Windows builds require stable Rust with Cargo and MSVC build tools (Visual Studio Build Tools with C++ support).

```powershell
cargo test --locked
cargo test --locked -p tray-item --lib
cargo clippy --locked --all-targets -- -D warnings
cargo build --locked --release
```

The executable is `target/release/MicroBin.exe`.

Configuration tests run on Linux without desktop libraries:

```sh
cargo fmt --all -- --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```

For Linux cross-compilation, install MinGW-w64 (GCC and windres) and the Rust target:

```sh
rustup target add x86_64-pc-windows-gnu
cargo build --locked --release --target x86_64-pc-windows-gnu
```

Cross-compilation does not verify tray behavior. Check menus, icons, confirmation, cancellation, and errors on Windows.
CI runs tests on Linux and Windows and uploads a Windows build. CI does not automate native UI checks.

The build uses a local tray-item patch for tray recovery. See [vendor/tray-item/PATCHES.md](vendor/tray-item/PATCHES.md).

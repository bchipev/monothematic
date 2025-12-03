monothematic
================

A pre-defined theme generator for the Noctalia shell.

What it does:
- Reads your current wallpaper from Noctalia's settings.
- Extracts the dominant color from the wallpaper and generates a 20-color OKLCH palette from almost white to not quite black.
- Generates a Noctalia shell colorscheme at `~/.config/quickshell/noctalia-shell/Assets/ColorScheme/Monothematic/Monothematic.json`.

Requirements:
- Noctalia shell configured and a wallpaper set in `~/.config/noctalia/settings.json`.

Build (from source):
- Ensure Rust is installed (https://rustup.rs/)
- Run: cargo build --release
- Binary: target/release/monothematic

Usage
-----

  # Generate from monitor 0 (default)
  monothematic set

  # Generate from a specific monitor id
  monothematic set 1

Commands
- set: Read current Noctalia wallpaper for a monitor and generate a theme file.

Options for `set`
- <MONITOR_ID> (optional)  Integer id of the monitor to read from Noctalia settings. Defaults to 0.

Noctalia integration
- Monothematic reads the wallpaper image path for the selected monitor from
  `~/.config/noctalia/settings.json`, more specifically from `wallpaper.monitors[MONITOR_ID].wallpaper` where `MONITOR_ID` defaults to `0` when not provided.

Noctalia colorscheme
- A custom Noctalia colorscheme JSON is generated at:
  ~/.config/quickshell/noctalia-shell/Assets/ColorScheme/Monothematic/Monothematic.json
- Colors are mapped from a predefined template to your generated palette by closest OKLCH lightness.

Template placeholders
When using --templates-dir, the following placeholders are available inside .tpl files:
- {{dominant}}            Hex color (#RRGGBB) of the dominant color
- {{color0}}..{{color19}}  20-color palette from near-white to near-black (hex)
- {{name}}                Image file stem (without extension)

Systemd (user) service (example):
Create ~/.config/systemd/user/monothematic.service with:

  [Unit]
  Description=Monothematic Theme Generator Service
  After=graphical-session.target
  Wants=graphical-session.target

  [Service]
  Type=simple
  ExecStart=%h/.cargo/bin/monothematic set
  Restart=on-failure
  RestartSec=2s

  [Install]
  WantedBy=default.target

Then enable and start:
- systemctl --user daemon-reload
- systemctl --user enable --now monothematic.service

Packaging:
- Coming soon to popular package managers (AUR/Nix/Debian/Fedora, etc.). For now, build from source as above.

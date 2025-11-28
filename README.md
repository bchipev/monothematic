monothematic
================

A small Wayland-friendly wallpaper switcher and theme generator.

What it does:
- Picks a random image from a user-defined directory (or per-output directories) and sets it as your wallpaper using swaybg (hard requirement).
- Extracts the dominant color from the wallpaper and generates a 20-color OKLCH palette from almost white to not quite black.
- Writes a theme file next to the image, named `'<image-filename>.theme.toml'`.
- Optionally renders simple templates (Matugen-like) by replacing placeholders in .tpl files.

Install prerequisite (required):
- Install swaybg and ensure it is on your PATH. This application requires swaybg to set wallpapers.
  - Arch: pacman -S swaybg
  - Debian/Ubuntu: apt install swaybg
  - Fedora: dnf install swaybg
  - Nix: nix-env -iA nixpkgs.swaybg (or add to your configuration)

Build (from source):
- Ensure Rust is installed (https://rustup.rs/)
- Run: cargo build --release
- Binary: target/release/monothematic

Usage examples:
- All outputs from one directory:
  monothematic set --dir /path/to/wallpapers
- Per-output directories:
  monothematic set --map eDP-1=/path/laptop --map HDMI-A-1=/path/external
- With template rendering:
  monothematic set --dir /path/walls --templates-dir ~/.config/monothematic/templates

Requirements:
- swaybg must be installed and available on PATH.

Systemd (user) service (example):
Create ~/.config/systemd/user/monothematic.service with:

  [Unit]
  Description=Monothematic Wallpaper + Theme Service
  After=graphical-session.target
  Wants=graphical-session.target

  [Service]
  Type=simple
  ExecStart=%h/.cargo/bin/monothematic set --dir /path/to/wallpapers --templates-dir %h/.config/monothematic/templates
  Restart=on-failure
  RestartSec=2s

  [Install]
  WantedBy=default.target

Then enable and start:
- systemctl --user daemon-reload
- systemctl --user enable --now monothematic.service

Packaging:
- Coming soon to popular package managers (AUR/Nix/Debian/Fedora, etc.). When packaged, swaybg will be listed as a dependency. For now, install swaybg and build from source as above.

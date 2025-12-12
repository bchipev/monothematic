# Monothematic

Generate a cohesive, monochromatic theme from your current wallpaper and apply it across apps via simple text templates.

Monothematic integrates with Noctalia to detect wallpaper changes, extracts a dominant color in OKLCH, builds a full palette, saves it to `~/.config/Monothematic/colors.json`, and rewrites your template files (e.g. Niri/GTK configs) with nearest-matching colors. It keeps watching your Noctalia config and regenerates automatically when the wallpaper changes.


## Features
- Wallpaper-driven, auto-updating color scheme (via Noctalia config)
- OKLCH-based palette with consistent lightness steps: `base-98` … `base-02`
- Additional signal colors: `error`, `warning`, `success` (light/dark variants)
- Lightweight templating: replace colors (hex or `oklch(...)`) in any text file by nearest lightness from the palette
- Safe: if a template has no colors, Monothematic writes it through unchanged


## How it works (high level)
1. Reads your Noctalia config to find the current wallpaper path.
2. Uses `sharp` to analyze the image and `culori` to convert to OKLCH.
3. Generates a palette around the dominant color with fixed lightness stops.
4. Writes the palette as JSON to `~/.config/Monothematic/colors.json`.
5. For each configured mapping, rewrites the destination file by replacing any colors found in the source template:
   - Supported in templates: `#rgb`, `#rrggbb`, and `oklch(l c h)` forms
   - For each color, finds the nearest palette color by lightness only
   - Preserves the format: hex in → hex out, `oklch(...)` in → `oklch(...)` out
6. Continues watching the Noctalia config for changes and repeats the process.


## Requirements
- Bun (for building/running from source). Install from https://bun.sh
- A Noctalia config with a readable wallpaper path (default path below)


## Installation
Clone and build a single-file binary using Bun:

```bash
git clone https://github.com/yourname/monothematic.git
cd monothematic
bun install
bun run build   # produces ./monothematic
```

Optionally move the binary into your PATH, e.g.:

```bash
install -m 0755 ./monothematic ~/.local/bin/
```


## Usage
Run Monothematic (it runs once, then watches for changes):

```bash
monothematic
```

Expected log messages will indicate the Noctalia config being watched and when regeneration occurs.

Tip: to just regenerate once (e.g., after editing templates), run and then Ctrl+C after the first "Scheme and themes generated." message.


## Configuration & paths
Monothematic follows an XDG-like layout under your home directory.

- Config directory: `~/.config/Monothematic`
- Main config file: `~/.config/Monothematic/config.json`
- User templates directory: `~/.config/Monothematic/templates`
- Output directory (auxiliary): `~/.config/Monothematic/themes`
- Generated palette JSON: `~/.config/Monothematic/colors.json`

On first run:
- A default `config.json` is created if missing.
- If your user templates directory is empty, bundled example templates are copied there to get you started.

Default config (conceptually):

```json
{
  "noctaliaConfigPath": "/home/<you>/.config/Noctalia/config.json",
  "templatesDir": "/home/<you>/.config/Monothematic/templates",
  "outputDir": "/home/<you>/.config/Monothematic/themes",
  "mappings": [
    { "name": "noctalia", "source": "<templates>/noctalia-theme.json", "destination": "/home/<you>/.config/Noctalia/theme.json" },
    { "name": "niri",     "source": "<templates>/niri.conf",           "destination": "/home/<you>/.config/niri/theme.conf" },
    { "name": "gtk3",     "source": "<templates>/gtk3.css",            "destination": "/home/<you>/.config/gtk-3.0/gtk.css" },
    { "name": "gtk4",     "source": "<templates>/gtk4.css",            "destination": "/home/<you>/.config/gtk-4.0/gtk.css" }
  ]
}
```

Notes:
- Sources are your template files under `templatesDir`.
- Destinations are the files Monothematic will overwrite with themed content.
- If a mapping's `source` does not exist, it is skipped without error.


## Templates
Templates are just text files. Monothematic scans them for colors and replaces them. You can use either hex or OKLCH notation.

Example Niri snippet (bundled example):

```ini
# Example Niri theme snippet. Colors will be replaced by Monothematic.
background = oklch(0.1 0.02 250)
foreground = #c8c8c8
border = oklch(0.6 0.03 200)
accent = #88c0d0
error = oklch(0.7 0.1 29)
warning = oklch(0.7 0.1 109)
success = oklch(0.7 0.1 142)
```

Tips for effective templates:
- Use multiple occurrences of colors you want to be tied together by perceived lightness.
- Prefer `oklch(l c h)` when you want to preserve format or tweak chroma/hue by hand; Monothematic will only match by lightness.
- When in doubt, start with simple hex colors and iterate.


## Noctalia integration
By default, Monothematic reads `~/.config/Noctalia/config.json` to find your wallpaper path. If you use a different location, update `noctaliaConfigPath` in `~/.config/Monothematic/config.json`.


## Development

Prerequisites:
- Bun

Install dependencies and run in development mode:

```bash
bun install
bun run dev
```

Build a static binary:

```bash
bun run build
```

Project layout:
- `src/index.ts` – program entrypoint: ensures config, generates once, then watches Noctalia config and regenerates
- `src/lib/config.ts` – config paths, default config, user config/bootstrap
- `src/lib/wallpaper.ts` – locates wallpaper path inside Noctalia-like JSON
- `src/lib/colors.ts` – image analysis, OKLCH conversion, palette generation, JSON export
- `src/lib/templates.ts` – color finding and replacement engine for templates
- `templates/` – bundled starter templates copied to your user templates if empty

Contributions: Issues and PRs welcome. Please keep the code style consistent with the existing files and add concise comments where needed.


## Troubleshooting
- No colors generated? Check logs for the detected Noctalia config path and ensure it exists and contains a valid wallpaper path.
- Nothing changes in output files? Make sure your template exists and contains colors in hex or `oklch(...)` notation.
- `sharp` install issues during build? Ensure your platform has compatible prebuilt binaries or the necessary build tooling. Re-run `bun install`.


## License
MIT
// monothematic: A small Wayland-friendly wallpaper + theme generator service
//
// What this application does (high level):
// 1) Picks a random image from a user-provided directory (or per-output mapping of directories)
//    and sets it as the wallpaper on Wayland compositors using `swaybg` (hard requirement).
// 2) Extracts the dominant color from the chosen wallpaper.
// 3) Builds a 20-color theme spanning "almost white" to "not quite black" in OKLCH space while
//    keeping the dominant hue/chroma. Writes a theme file next to the image.
// 4) Optionally renders application-specific config files from templates, similar to matugen.
//
// Notes:
// - Wayland itself doesn’t define a standard API for setting wallpapers. Compositors differ.
//   We use `swaybg` as the single, required wallpaper helper and spawn one process per output.
// - Multi-monitor: we try to detect outputs and set images per output.
// - Theme file naming: We place the theme file alongside the image using the exact image file name
//   plus the suffix ".theme.toml" (e.g., sunset.jpg.theme.toml). This avoids overwriting the image
//   while keeping a strict 1:1 mapping to the exact file name.
//
// This file contains many comments, explaining “what” and “why”, for beginners.

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use image::GenericImageView;
use palette::{convert::FromColorUnclamped, Oklab, Oklch, Srgb};
use image::Pixel; // brings to_rgb() into scope for image pixels
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;
use which::which;

// ----------------------------- CLI definitions -----------------------------

#[derive(Parser, Debug)]
#[command(author, version, about = "Wayland wallpaper + theme generator service", long_about = None)]
struct Cli {
    /// Templates directory holding files that will be rendered with the generated theme.
    /// Each file ending with .tpl will be rendered into a sibling file with the .tpl removed.
    #[arg(long)]
    templates_dir: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Set a random wallpaper (optionally per-output) and ensure theme files exist for it/them.
    Set {
        /// Single directory used for all outputs (ignored if --map is used)
        #[arg(long)]
        dir: Option<PathBuf>,

        /// Map of OUTPUT=DIR allowing different directories per display output.
        /// Example: --map eDP-1=/path/laptop --map HDMI-A-1=/path/external
        #[arg(long = "map")] 
        maps: Vec<String>,
    },
}

// ----------------------------- Data structures -----------------------------

/// The theme we generate and store next to the image.
#[derive(Debug, Serialize, Deserialize)]
struct ThemeFile {
    // The dominant color in hex sRGB like #RRGGBB
    dominant_hex: String,
    // The dominant color in OKLCH broken out for readability/portability
    dominant_oklch: DominantOKLCH,
    // The 20-color palette from near-white to near-black (hex sRGB)
    palette_hex: Vec<String>,
    // Original image this theme is derived from
    image_path: String,
    // Generation timestamp (for traceability)
    generated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
struct DominantOKLCH {
    // Lightness 0..1
    l: f32,
    // Chroma ~0..0.4 typical
    c: f32,
    // Hue in turns (0..1)
    h: f32,
}

// ----------------------------- Utilities -----------------------------

fn is_image(path: &Path) -> bool {
    matches!(path.extension().and_then(OsStr::to_str).map(|s| s.to_lowercase()),
        Some(ext) if ["png","jpg","jpeg","gif","bmp","ico","tiff","webp"].contains(&ext.as_str())
    )
}

fn list_images(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = vec![];
    for entry in WalkDir::new(dir).min_depth(1).max_depth(1) {
        let entry = entry?;
        if entry.file_type().is_file() {
            let p = entry.path().to_path_buf();
            if is_image(&p) { files.push(p); }
        }
    }
    if files.is_empty() {
        Err(anyhow!("No image files found in {}", dir.display()))
    } else {
        Ok(files)
    }
}

fn random_image(dir: &Path) -> Result<PathBuf> {
    let mut imgs = list_images(dir)?;
    let mut rng = rand::thread_rng();
    imgs.shuffle(&mut rng);
    imgs.into_iter().next().ok_or_else(|| anyhow!("no images"))
}

fn file_stem(p: &Path) -> Result<String> {
    p.file_stem()
        .and_then(OsStr::to_str)
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("Could not extract file stem for {}", p.display()))
}

fn theme_path_for_image(img: &Path) -> Result<PathBuf> {
    // We write alongside the image and keep the image file name intact, appending a suffix.
    // Example: sunset.jpg -> sunset.jpg.theme.toml (clearly tied to the exact image file)
    let fname = img.file_name()
        .and_then(OsStr::to_str)
        .ok_or_else(|| anyhow!("Could not get file name for {}", img.display()))?;
    Ok(img.parent().unwrap_or(Path::new(".")).join(format!("{}.theme.toml", fname)))
}

fn now_iso() -> String {
    // Simple, dependency-free-ish timestamp; precision is not critical here
    // Using chrono would add another dependency; current approach is sufficient
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    format!("{}", secs)
}

fn srgb_to_hex(c: Srgb<f32>) -> String {
    // Clamp to [0,1] and convert to 0..255
    let r = (c.red.clamp(0.0, 1.0) * 255.0).round() as u8;
    let g = (c.green.clamp(0.0, 1.0) * 255.0).round() as u8;
    let b = (c.blue.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!("#{:02X}{:02X}{:02X}", r, g, b)
}

fn hex_to_srgb(hex: &str) -> Option<Srgb<f32>> {
    let s = hex.strip_prefix('#').unwrap_or(hex);
    if s.len() != 6 { return None; }
    let r = u8::from_str_radix(&s[0..2], 16).ok()? as f32 / 255.0;
    let g = u8::from_str_radix(&s[2..4], 16).ok()? as f32 / 255.0;
    let b = u8::from_str_radix(&s[4..6], 16).ok()? as f32 / 255.0;
    Some(Srgb::new(r, g, b))
}

// ----------------------------- Color logic -----------------------------

fn dominant_color_oklch(img_path: &Path) -> Result<Oklch<f32>> {
    // Load the image using the `image` crate
    let img = image::open(img_path)
        .with_context(|| format!("Failed to open image {}", img_path.display()))?;

    // Downscale heavily to reduce computation; we only need an approximate dominant color.
    // 64x64 is sufficient for a decent estimate while being fast.
    let thumb = img.thumbnail(64, 64);

    // Compute average color in linearized space for simplicity. For “dominant” we could
    // use k-means or median-cut, but average works surprisingly well on wallpapers.
    let mut acc = [0.0f64, 0.0, 0.0];
    let mut count = 0.0f64;
    for (_x, _y, p) in thumb.pixels() {
        let rgb = p.to_rgb();
        // Convert from 8-bit sRGB to linear f32 Srgb
        let srgb = Srgb::new(
            rgb[0] as f32 / 255.0,
            rgb[1] as f32 / 255.0,
            rgb[2] as f32 / 255.0,
        );
        // Accumulate in f64 for numerical stability
        acc[0] += srgb.red as f64;
        acc[1] += srgb.green as f64;
        acc[2] += srgb.blue as f64;
        count += 1.0;
    }
    if count == 0.0 { return Err(anyhow!("No pixels read")); }
    let avg = Srgb::new((acc[0]/count) as f32, (acc[1]/count) as f32, (acc[2]/count) as f32);
    // Convert to OKLCH for perceptual adjustments
    let oklch = Oklch::from_color_unclamped(avg.into_linear());
    Ok(oklch)
}

fn build_palette_from_dominant(ok: Oklch<f32>) -> Vec<Oklch<f32>> {
    // We maintain hue (ok.h) and chroma (ok.c), and vary lightness (ok.l)
    // across 20 steps from near-white (0.98) to near-black (0.12).
    let steps = 20;
    let l_start = 0.98f32; // almost white
    let l_end = 0.12f32;   // not quite black
    let mut out = Vec::with_capacity(steps);
    for i in 0..steps {
        let t = i as f32 / (steps - 1) as f32; // 0..1
        let l = l_start + (l_end - l_start) * t;
        out.push(Oklch::new(l, ok.chroma, ok.hue));
    }
    out
}

fn palette_to_hex(pal: &[Oklch<f32>]) -> Vec<String> {
    pal.iter()
        .map(|c| {
            // Convert Oklch -> Oklab -> sRGB for hex output
            let lab: Oklab<f32> = Oklab::from_color_unclamped(*c);
            let rgb: Srgb<f32> = Srgb::from_color_unclamped(lab);
            srgb_to_hex(rgb)
        })
        .collect()
}

// ----------------------------- Theme file IO -----------------------------

fn ensure_theme_for_image(img: &Path) -> Result<ThemeFile> {
    let theme_path = theme_path_for_image(img)?;
    if theme_path.exists() {
        // If already exists, load and return it to avoid regenerating.
        let txt = fs::read_to_string(&theme_path)
            .with_context(|| format!("Failed reading theme file {}", theme_path.display()))?;
        let tf: ThemeFile = toml::from_str(&txt)
            .with_context(|| format!("Failed parsing theme file {}", theme_path.display()))?;
        return Ok(tf);
    }

    // Generate dominant and palette
    let dom = dominant_color_oklch(img)?;
    let pal_ok = build_palette_from_dominant(dom);
    let pal_hex = palette_to_hex(&pal_ok);
    let dom_hex = {
        let lab: Oklab<f32> = Oklab::from_color_unclamped(dom);
        let rgb: Srgb<f32> = Srgb::from_color_unclamped(lab);
        srgb_to_hex(rgb)
    };

    let tf = ThemeFile {
        dominant_hex: dom_hex,
        dominant_oklch: DominantOKLCH { l: dom.l, c: dom.chroma, h: dom.hue.into() },
        palette_hex: pal_hex,
        image_path: img.to_string_lossy().to_string(),
        generated_at: now_iso(),
    };

    // Serialize as TOML for readability and easy templating
    let toml_txt = toml::to_string_pretty(&tf)?;
    fs::write(&theme_path, toml_txt)?;
    Ok(tf)
}

// ----------------------------- Output detection -----------------------------

fn detect_outputs() -> Result<Vec<String>> {
    // Try Hyprland first (hyprctl monitors -j)
    if which("hyprctl").is_ok() {
        if let Ok(out) = Command::new("hyprctl").args(["monitors", "-j"]).output() {
            if out.status.success() {
                let v: serde_json::Value = serde_json::from_slice(&out.stdout)?;
                let arr = v.as_array().ok_or_else(|| anyhow!("Unexpected hyprctl monitors output"))?;
                let names: Vec<String> = arr.iter()
                    .filter_map(|m| m.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()))
                    .collect();
                if !names.is_empty() { return Ok(names); }
            }
        }
    }
    // Try Sway (swaymsg -t get_outputs)
    if which("swaymsg").is_ok() {
        if let Ok(out) = Command::new("swaymsg").args(["-t", "get_outputs"]).output() {
            if out.status.success() {
                let v: serde_json::Value = serde_json::from_slice(&out.stdout)?;
                let arr = v.as_array().ok_or_else(|| anyhow!("Unexpected sway outputs output"))?;
                let names: Vec<String> = arr.iter()
                    .filter(|m| m.get("active").and_then(|a| a.as_bool()).unwrap_or(false))
                    .filter_map(|m| m.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()))
                    .collect();
                if !names.is_empty() { return Ok(names); }
            }
        }
    }
    // Fallback: single unnamed output
    Ok(vec!["default".to_string()])
}

// ----------------------------- Wallpaper setting -----------------------------

fn set_wallpaper(mapping: &[(String, PathBuf)]) -> Result<()> {
    // Require swaybg; provide a clear error if not installed.
    if which("swaybg").is_err() {
        return Err(anyhow!(
            "swaybg is required but was not found on PATH. Please install swaybg and try again."
        ));
    }
    // We spawn one swaybg per output. In a service, the processes keep running.
    for (output, path) in mapping {
        let _child = Command::new("swaybg")
            .args(["-o", output, "-i", &path.to_string_lossy(), "-m", "fill"]) // fill keeps aspect nicely
            .spawn()?;
    }
    Ok(())
}

// ----------------------------- Templates rendering -----------------------------

fn render_templates(templates_dir: &Path, theme: &ThemeFile, stem: &str) -> Result<()> {
    // For simplicity, we treat any file ending with .tpl as a text template.
    // We support simple placeholders like {{dominant}}, {{color0}}..{{color19}}, {{name}}.
    // The output filename is the same as the template but with .tpl removed; we also
    // replace any literal "{{name}}" in the filename with the image stem.
    for entry in WalkDir::new(templates_dir).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            let p = entry.path();
            if p.extension().and_then(OsStr::to_str) == Some("tpl") {
                let mut txt = fs::read_to_string(p)
                    .with_context(|| format!("Failed reading template {}", p.display()))?;
                // Replace placeholders
                txt = txt.replace("{{dominant}}", &theme.dominant_hex);
                for (idx, col) in theme.palette_hex.iter().enumerate() {
                    let key = format!("{{{{color{}}}}}", idx);
                    txt = txt.replace(&key, col);
                }
                let name_key = "{{name}}";
                let mut out_name = p.file_stem().and_then(OsStr::to_str).unwrap_or("output").to_string();
                if let Some(stripped) = out_name.strip_suffix(".tpl") { out_name = stripped.to_string(); }
                out_name = out_name.replace(name_key, stem);
                // If the stem didn't include removal, ensure we drop .tpl from the path
                let mut out_path = p.with_extension("");
                // Some OSes keep a trailing dot; rebuild explicitly
                out_path = out_path.with_file_name(out_name);
                fs::write(&out_path, txt)
                    .with_context(|| format!("Failed writing rendered template {}", out_path.display()))?;
            }
        }
    }
    Ok(())
}

// ----------------------------- Mapping parse -----------------------------

fn parse_maps(maps: &[String]) -> Result<HashMap<String, PathBuf>> {
    let mut out = HashMap::new();
    for m in maps {
        if let Some((k, v)) = m.split_once('=') {
            out.insert(k.to_string(), PathBuf::from(v));
        } else {
            return Err(anyhow!("--map expects OUTPUT=DIR, got: {}", m));
        }
    }
    Ok(out)
}

// ----------------------------- Main flow -----------------------------

fn handle_set(dir: Option<PathBuf>, maps: Vec<String>, templates_dir: Option<PathBuf>) -> Result<()> {
    let outputs = detect_outputs()?;

    let mapping: Vec<(String, PathBuf)> = if !maps.is_empty() {
        // If user provided explicit mapping, use it and validate outputs
        let m = parse_maps(&maps)?;
        let mut out = vec![];
        for o in &outputs {
            let d = m.get(o).ok_or_else(|| anyhow!("No directory specified for output {} via --map", o))?;
            out.push((o.clone(), d.clone()));
        }
        out
    } else {
        let dir = dir.ok_or_else(|| anyhow!("Either --dir or --map must be provided"))?;
        outputs.into_iter().map(|o| (o, dir.clone())).collect()
    };

    // Select image per output and ensure theme files
    let mut per_output_image: Vec<(String, PathBuf, ThemeFile, String)> = vec![]; // (out, img, theme, stem)
    for (out, d) in &mapping {
        let img = random_image(d)?;
        let stem = file_stem(&img)?;
        let theme = ensure_theme_for_image(&img)?; // creates if absent
        // Render templates if requested
        if let Some(tdir) = templates_dir.as_deref() {
            render_templates(tdir, &theme, &stem)?;
        }
        per_output_image.push((out.clone(), img, theme, stem));
    }

    // Actually set wallpapers now
    let mapping_only: Vec<(String, PathBuf)> = per_output_image.iter().map(|(o, p, _, _)| (o.clone(), p.clone())).collect();
    set_wallpaper(&mapping_only)?;

    Ok(())
}

fn main() -> Result<()> {
    // We use clap to parse arguments and expose two main actions for now: `set`.
    // You can wire this into systemd with a unit file calling `monothematic set ...`.
    let cli = Cli::parse();
    match cli.command {
        Commands::Set { dir, maps, } => {
            handle_set(dir, maps, cli.templates_dir)?;
        }
    }
    Ok(())
}

// ----------------------------- Systemd notes -----------------------------
// Example systemd user service (save as ~/.config/systemd/user/monothematic.service):
//
// [Unit]
// Description=Monothematic Wallpaper + Theme Service
// After=graphical-session.target
// Wants=graphical-session.target
//
// [Service]
// Type=simple
// ExecStart=%h/.cargo/bin/monothematic set --dir /path/to/wallpapers --templates-dir %h/.config/monothematic/templates
// Restart=on-failure
// RestartSec=2s
//
// [Install]
// WantedBy=default.target
//
// For per-output directories, replace ExecStart with e.g.:
// ExecStart=%h/.cargo/bin/monothematic set --map eDP-1=%h/Pictures/walls/laptop --map HDMI-A-1=%h/Pictures/walls/ultrawide --templates-dir %h/.config/monothematic/templates
//
// Then enable & start (user service):
//   systemctl --user daemon-reload
//   systemctl --user enable --now monothematic.service
//
// Make sure swaybg is installed and on your PATH (hard requirement).

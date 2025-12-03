// monothematic: A QuickShell-aware theme generator service

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use image::GenericImageView;
use palette::{convert::FromColorUnclamped, Oklab, Oklch, Srgb};
use image::Pixel;
use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use notify::{RecommendedWatcher, Watcher, RecursiveMode, EventKind};


// ----------------------------- CLI definitions -----------------------------

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Generate themes from your current QuickShell wallpaper(s)",
    long_about = "Monothematic reads your current QuickShell wallpaper configuration and derives theme colors from the\ncurrently used image(s). It generates a 20-color palette from the dominant color and writes a '<image>.theme.toml'\nnext to each wallpaper image it finds. It also generates a Noctalia colorscheme JSON at the QuickShell Noctalia path.",
    after_help = "Quick start:\n  • Generate from current QuickShell wallpaper(s):\n      monothematic set 0\n\nNotes:\n  - This tool targets QuickShell users. It reads your wallpaper paths from your QuickShell configuration.\n  - Theme files are created next to each wallpaper using the pattern '<image-filename>.theme.toml'.\n  - Noctalia colorscheme JSON is written under '~/.config/quickshell/noctalia-shell/Assets/ColorScheme/Monothematic/'."
)]
struct Cli {

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Generate a theme from the wallpaper used on the specified monitor.
    #[command(
        about = "Generate theme file from Noctalia's current wallpaper for a monitor",
        long_about = "Reads the wallpaper path for the given monitor from '~/.config/noctalia/settings.json' and generates a theme file\n'<image>.theme.toml' next to that wallpaper. Also updates the Noctalia colorscheme JSON.\n\nIf no monitor id is provided, it defaults to 0."
    )]
    Set {
        /// Integer monitor id to read from Noctalia settings.json (wallpaper.monitors). Defaults to 0.
        #[arg(value_name = "MONITOR_ID", default_value_t = 0)]
        monitor_id: u32,
    },
    /// Watch Noctalia settings.json and regenerate theme on wallpaper changes
    #[command(
        about = "Continuously watch '~/.config/noctalia/settings.json' and regenerate theme on changes",
        long_about = "Monitors Noctalia's settings.json for changes to the current wallpaper. When a change is detected\nfor the specified monitor index, regenerates the theme file next to that wallpaper and updates the Noctalia colorscheme JSON.\n\nIf no monitor id is provided, it defaults to 0."
    )]
    Watch {
        /// Integer monitor id to read from Noctalia settings.json (wallpaper.monitors). Defaults to 0.
        #[arg(value_name = "MONITOR_ID", default_value_t = 0)]
        monitor_id: u32,
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
    // Error color (mid-red hue with dominant lightness/chroma), hex sRGB like #RRGGBB
    #[serde(default, skip_serializing_if = "Option::is_none")]
    error_hex: Option<String>,
    // OnError color: a brighter version of the error color for legible content on error surfaces
    #[serde(default, skip_serializing_if = "Option::is_none")]
    on_error_hex: Option<String>,
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

// No persistent config remains; feature set has been simplified.

// ----------------------------- Utilities -----------------------------

// Removed legacy image listing/randomization utilities.

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

// ----------------------------- Config IO -----------------------------

// Removed config I/O; no persistent configuration is used anymore.

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
    // across 40 steps from near-white (0.98) to near-black (0.02).
    let steps = 40;
    let l_start = 0.98f32; // almost white
    let l_end = 0.02f32;   // not quite black
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

fn oklch_to_hex(c: Oklch<f32>) -> String {
    let lab: Oklab<f32> = Oklab::from_color_unclamped(c);
    let rgb: Srgb<f32> = Srgb::from_color_unclamped(lab);
    srgb_to_hex(rgb)
}

fn compute_error_oklch_from_theme(theme: &ThemeFile) -> Oklch<f32> {
    // Keep L and C from dominant; set hue to mid-red (derived from sRGB pure red)
    let l = theme.dominant_oklch.l;
    let c = theme.dominant_oklch.c;
    let red_hue = {
        let red_oklch: Oklch<f32> = Oklch::from_color_unclamped(
            Oklab::from_color_unclamped(Srgb::new(1.0, 0.0, 0.0).into_linear()),
        );
        red_oklch.hue
    };
    Oklch::new(l, c, red_hue)
}

fn compute_on_error_oklch_from_theme(theme: &ThemeFile) -> Oklch<f32> {
    // Brighter variant of the error color: increase lightness while keeping chroma and hue
    let err = compute_error_oklch_from_theme(theme);
    let brighter_l = (err.l + 0.30).min(0.98); // clamp near-white ceiling
    Oklch::new(brighter_l, err.chroma, err.hue)
}

// ----------------------------- Theme file IO -----------------------------

fn ensure_theme_for_image(img: &Path) -> Result<ThemeFile> {
    let theme_path = theme_path_for_image(img)?;
    if theme_path.exists() {
        // If already exists, load and return it to avoid regenerating.
        let txt = fs::read_to_string(&theme_path)
            .with_context(|| format!("Failed reading theme file {}", theme_path.display()))?;
        let mut tf: ThemeFile = toml::from_str(&txt)
            .with_context(|| format!("Failed parsing theme file {}", theme_path.display()))?;
        // Backward compatibility: compute error_hex if missing
        if tf.error_hex.is_none() {
            let err = compute_error_oklch_from_theme(&tf);
            tf.error_hex = Some(oklch_to_hex(err));
        }
        // Backward compatibility: compute on_error_hex if missing
        if tf.on_error_hex.is_none() {
            let on_err = compute_on_error_oklch_from_theme(&tf);
            tf.on_error_hex = Some(oklch_to_hex(on_err));
        }
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
    let err_hex = {
        let red_hue = {
            let red_oklch: Oklch<f32> = Oklch::from_color_unclamped(
                Oklab::from_color_unclamped(Srgb::new(1.0, 0.0, 0.0).into_linear()),
            );
            red_oklch.hue
        };
        let err = Oklch::new(dom.l, dom.chroma, red_hue);
        oklch_to_hex(err)
    };
    let on_err_hex = {
        // compute from the error color by brightening lightness
        let err_ok = compute_error_oklch_from_theme(&ThemeFile {
            dominant_hex: String::new(),
            dominant_oklch: DominantOKLCH { l: dom.l, c: dom.chroma, h: dom.hue.into() },
            error_hex: None,
            on_error_hex: None,
            palette_hex: vec![],
            image_path: String::new(),
            generated_at: String::new(),
        });
        let brighter_l = (err_ok.l + 0.30).min(0.98);
        oklch_to_hex(Oklch::new(brighter_l, err_ok.chroma, err_ok.hue))
    };

    let tf = ThemeFile {
        dominant_hex: dom_hex,
        dominant_oklch: DominantOKLCH { l: dom.l, c: dom.chroma, h: dom.hue.into() },
        error_hex: Some(err_hex),
        on_error_hex: Some(on_err_hex),
        palette_hex: pal_hex,
        image_path: img.to_string_lossy().to_string(),
        generated_at: now_iso(),
    };

    // Serialize as TOML for readability and easy templating
    let toml_txt = toml::to_string_pretty(&tf)?;
    fs::write(&theme_path, toml_txt)?;
    Ok(tf)
}

// ----------------------------- Noctalia wallpaper query -----------------------------
const NOCTALIA_SETTINGS_PATH: &str = "~/.config/noctalia/settings.json";

fn read_wallpaper_from_noctalia(monitor_id: u32) -> Result<PathBuf> {
    // Resolve the fixed settings path against $HOME
    let home = std::env::var("HOME").context("$HOME not set")?;
    let settings = Path::new(&home).join(".config").join("noctalia").join("settings.json");

    if !settings.exists() {
        return Err(anyhow!("Noctalia settings not found at {}", NOCTALIA_SETTINGS_PATH));
    }

    let txt = fs::read_to_string(&settings)
        .with_context(|| format!("Failed to read {}", settings.display()))?;
    let json: serde_json::Value = serde_json::from_str(&txt)
        .with_context(|| format!("Invalid JSON in {}", settings.display()))?;

    // Treat monitor_id as an ARRAY INDEX under wallpaper.monitors
    let monitors = json
        .get("wallpaper")
        .and_then(|w| w.get("monitors"))
        .ok_or_else(|| anyhow!("Missing 'wallpaper.monitors' in {}", NOCTALIA_SETTINGS_PATH))?;

    let arr = monitors
        .as_array()
        .ok_or_else(|| anyhow!("'wallpaper.monitors' must be an array in {}", NOCTALIA_SETTINGS_PATH))?;

    let idx = monitor_id as usize;
    let mon = arr
        .get(idx)
        .ok_or_else(|| anyhow!(
            "Monitor index {} is out of bounds for 'wallpaper.monitors' (len = {}) in {}",
            idx,
            arr.len(),
            NOCTALIA_SETTINGS_PATH
        ))?;

    let s = mon
        .get("wallpaper")
        .and_then(|p| p.as_str())
        .ok_or_else(|| anyhow!(
            "Missing string 'wallpaper' at 'wallpaper.monitors[{}]' in {}",
            idx,
            NOCTALIA_SETTINGS_PATH
        ))?;

    let s = s.strip_prefix("file:///").unwrap_or(s);
    let pb = PathBuf::from(s);
    if !pb.exists() {
        return Err(anyhow!("Wallpaper file does not exist: {}", pb.display()));
    }
    Ok(pb)
}

// ----------------------------- Noctalia colorscheme -----------------------------

fn oklch_from_hex(hex: &str) -> Result<Oklch<f32>> {
    let rgb = hex_to_srgb(hex).ok_or_else(|| anyhow!("Invalid hex color: {}", hex))?;
    let lab: Oklab<f32> = Oklab::from_color_unclamped(rgb.into_linear());
    Ok(Oklch::from_color_unclamped(lab))
}

fn palette_oklch(theme: &ThemeFile) -> Vec<(String, Oklch<f32>)> {
    theme
        .palette_hex
        .iter()
        .filter_map(|h| hex_to_srgb(h).map(|rgb| (h.clone(), Oklch::from_color_unclamped(Oklab::from_color_unclamped(rgb.into_linear())))))
        .collect()
}

fn closest_by_lightness(target: Oklch<f32>, palette: &[(String, Oklch<f32>)]) -> String {
    let mut best = (std::f32::MAX, palette.get(0).map(|p| p.0.clone()).unwrap_or_else(|| "#000000".to_string()));
    for (hex, ok) in palette {
        let d = (ok.l - target.l).abs();
        if d < best.0 { best = (d, hex.clone()); }
    }
    best.1
}

fn noctalia_output_path() -> Result<PathBuf> {
    let home = std::env::var("HOME").context("$HOME not set")?;
    let dir = Path::new(&home)
        .join(".config")
        .join("quickshell")
        .join("noctalia-shell")
        .join("Assets")
        .join("ColorScheme")
        .join("Monothematic");
    fs::create_dir_all(&dir).with_context(|| format!("Failed to create {}", dir.display()))?;
    Ok(dir.join("Monothematic.json"))
}

fn write_noctalia_colors(theme: &ThemeFile) -> Result<()> {
    // Template keys as provided by the spec; we map each to nearest palette color by lightness.
    let template_pairs = [
        ("dark", vec![
            ("mPrimary", "#aaaaaa"), ("mOnPrimary", "#111111"), ("mSecondary", "#a7a7a7"), ("mOnSecondary", "#111111"),
            ("mTertiary", "#cccccc"), ("mOnTertiary", "#111111"), ("mError", "#dddddd"), ("mOnError", "#111111"),
            ("mSurface", "#111111"), ("mOnSurface", "#828282"), ("mHover", "#cccccc"), ("mOnHover", "#111111"),
            ("mSurfaceVariant", "#191919"), ("mOnSurfaceVariant", "#5d5d5d"), ("mOutline", "#3c3c3c"), ("mShadow", "#000000"),
        ]),
        ("light", vec![
            ("mPrimary", "#555555"), ("mOnPrimary", "#eeeeee"), ("mSecondary", "#505058"), ("mOnSecondary", "#eeeeee"),
            ("mTertiary", "#333333"), ("mOnTertiary", "#eeeeee"), ("mError", "#222222"), ("mOnError", "#efefef"),
            ("mSurface", "#d4d4d4"), ("mOnSurface", "#696969"), ("mHover", "#333333"), ("mOnHover", "#eeeeee"),
            ("mSurfaceVariant", "#e8e8e8"), ("mOnSurfaceVariant", "#9e9e9e"), ("mOutline", "#c3c3c3"), ("mShadow", "#fafafa"),
        ]),
    ];

    let pal = palette_oklch(theme);
    let mut dark_obj = serde_json::Map::new();
    let mut light_obj = serde_json::Map::new();
    for (section, pairs) in template_pairs {
        for (key, hex) in pairs {
            // Special-case error: use the generated mid-red with same L/C
            let mapped = if key == "mError" {
                theme
                    .error_hex
                    .clone()
                    .unwrap_or_else(|| oklch_to_hex(compute_error_oklch_from_theme(theme)))
            } else if key == "mOnError" {
                theme
                    .on_error_hex
                    .clone()
                    .unwrap_or_else(|| oklch_to_hex(compute_on_error_oklch_from_theme(theme)))
            } else {
                let target = oklch_from_hex(hex)?;
                closest_by_lightness(target, &pal)
            };
            match section {
                "dark" => { dark_obj.insert(key.to_string(), serde_json::Value::String(mapped)); },
                "light" => { light_obj.insert(key.to_string(), serde_json::Value::String(mapped)); },
                _ => {}
            }
        }
    }
    let root = serde_json::json!({ "dark": dark_obj, "light": light_obj });
    let out = noctalia_output_path()?;
    fs::write(&out, serde_json::to_string_pretty(&root)?).with_context(|| format!("Failed to write Noctalia colors to {}", out.display()))?;
    Ok(())
}

// ----------------------------- Main flow -----------------------------

fn handle_set(monitor_id: u32) -> Result<()> {
    // Read current wallpaper path for the specified monitor from Noctalia settings
    let img = read_wallpaper_from_noctalia(monitor_id)?;

    // Generate/refresh theme file for this wallpaper
    let theme = ensure_theme_for_image(&img)?;

    // Update Noctalia colorscheme from the last processed theme (arbitrary choice if multiple)
    write_noctalia_colors(&theme)?;

    Ok(())
}

fn resolve_noctalia_settings_path() -> Result<PathBuf> {
    let home = std::env::var("HOME").context("$HOME not set")?;
    Ok(Path::new(&home).join(".config").join("noctalia").join("settings.json"))
}

fn handle_watch(monitor_id: u32) -> Result<()> {
    use std::sync::mpsc::channel;

    let settings_path = resolve_noctalia_settings_path()?;
    let parent = settings_path.parent().unwrap_or(Path::new("."));

    // Run once at start
    if let Err(e) = handle_set(monitor_id) { eprintln!("Initial generation failed: {:#}", e); }

    // Channel to receive filesystem events
    let (tx, rx) = channel();

    // Create watcher
    let mut watcher: RecommendedWatcher = notify::recommended_watcher(move |res| {
        // Forward events to the channel; ignore send errors (receiver may be gone)
        let _ = tx.send(res);
    })?;

    // Watch the parent directory non-recursively; file may be replaced atomically
    watcher.watch(parent, RecursiveMode::NonRecursive)?;

    println!("Watching {} for changes... (monitor {})", settings_path.display(), monitor_id);

    // Simple debounce to avoid duplicate triggers on a single write
    let mut last_run: Option<Instant> = None;

    loop {
        match rx.recv() {
            Ok(Ok(event)) => {
                // Only react to modifications/creates to settings.json
                let mut is_relevant = false;
                if let Some(paths) = (!event.paths.is_empty()).then_some(&event.paths) {
                    for p in paths {
                        if p.file_name() == settings_path.file_name() {
                            match &event.kind {
                                EventKind::Modify(_) | EventKind::Create(_) | EventKind::Any => {
                                    is_relevant = true;
                                }
                                _ => {}
                            }
                        }
                    }
                }
                if !is_relevant { continue; }

                let now = Instant::now();
                if let Some(prev) = last_run {
                    if now.duration_since(prev).as_millis() < 300 { continue; }
                }
                last_run = Some(now);

                match handle_set(monitor_id) {
                    Ok(()) => println!("Theme regenerated from updated wallpaper."),
                    Err(e) => eprintln!("Failed to regenerate on change: {:#}", e),
                }
            }
            Ok(Err(e)) => {
                eprintln!("Watcher error: {e}");
            }
            Err(_disconnected) => {
                eprintln!("Watcher channel disconnected. Exiting.");
                break;
            }
        }
    }

    Ok(())
}

fn main() -> Result<()> {
    // We use clap to parse arguments and expose one main action for now: `set`.
    // This tool reads existing QuickShell wallpaper(s) and generates themes.
    let cli = Cli::parse();
    match cli.command {
        Commands::Set { monitor_id } => {
            handle_set(monitor_id)?;
        }
        Commands::Watch { monitor_id } => {
            handle_watch(monitor_id)?;
        }
    }
    Ok(())
}
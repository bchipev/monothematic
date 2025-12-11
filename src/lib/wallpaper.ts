import fs from 'fs-extra';
import path from 'path';

// Attempts to find a wallpaper path inside a Noctalia-like JSON config.
export async function getWallpaperPathFromNoctalia(configPath: string): Promise<string | null> {
  const exists = await fs.pathExists(configPath);
  if (!exists) return null;
  const json = await fs.readJson(configPath).catch(() => null);
  if (!json) return null;

  const candidate = findWallpaperPath(json);
  if (!candidate) return null;
  const abs = path.isAbsolute(candidate) ? candidate : path.resolve(path.dirname(configPath), candidate);
  return (await fs.pathExists(abs)) ? abs : null;
}

function findWallpaperPath(obj: any): string | null {
  if (!obj) return null;
  if (typeof obj === 'string') {
    // Heuristic: strings that look like image paths
    if (/\.(png|jpe?g|bmp|webp|tiff?)$/i.test(obj)) return obj;
  }
  if (Array.isArray(obj)) {
    for (const item of obj) {
      const r = findWallpaperPath(item);
      if (r) return r;
    }
    return null;
  }
  if (typeof obj === 'object') {
    for (const [k, v] of Object.entries(obj)) {
      if (/wallpaper/i.test(k)) {
        const r = findWallpaperPath(v);
        if (r) return r;
      }
    }
    // Fallback: search any value
    for (const v of Object.values(obj)) {
      const r = findWallpaperPath(v);
      if (r) return r;
    }
  }
  return null;
}

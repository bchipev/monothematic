import { ensureUserConfig, getConfig, getNoctaliaConfigPath, getPaths } from './lib/config.js';
import { getWallpaperPathFromNoctalia } from './lib/wallpaper.js';
import { extractDominantOklch, generateScheme, writeSchemeJson, type Scheme } from './lib/colors.js';
import { applyTemplates } from './lib/templates.js';
import chokidar from 'chokidar';

async function runOnce(): Promise<Scheme | null> {
  const cfg = await getConfig();
  const noctaliaConfig = getNoctaliaConfigPath(cfg);
  const wallpaperPath = await getWallpaperPathFromNoctalia(noctaliaConfig);
  if (!wallpaperPath) {
    console.error('Monothematic: Could not determine wallpaper from Noctalia config:', noctaliaConfig);
    return null;
  }
  const dominant = await extractDominantOklch(wallpaperPath);
  const scheme = generateScheme(dominant);
  await writeSchemeJson(scheme);
  await applyTemplates(scheme);
  console.log('Monothematic: Scheme and themes generated.');
  return scheme;
}

async function main() {
  await ensureUserConfig();
  await runOnce();

  // Watch Noctalia config for wallpaper changes
  const cfg = await getConfig();
  const noctaliaConfig = getNoctaliaConfigPath(cfg);
  console.log('Monothematic: Watching for changes in', noctaliaConfig);
  const watcher = chokidar.watch(noctaliaConfig, { ignoreInitial: true });
  watcher.on('change', async () => {
    try {
      console.log('Monothematic: Detected Noctalia config change. Regenerating...');
      await runOnce();
    } catch (e) {
      console.error('Monothematic: Error regenerating after change:', e);
    }
  });
}

main().catch((e) => {
  console.error('Monothematic fatal error:', e);
  process.exit(1);
});

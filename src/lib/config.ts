import os from 'os';
import path from 'path';
import fs from 'fs-extra';

export type TemplateMapping = {
  name: string;
  source: string; // template file path
  destination: string; // where to write themed file
};

export type AppConfig = {
  noctaliaConfigPath: string;
  templatesDir: string;
  mappings: TemplateMapping[];
  outputDir: string;
};

export function getPaths() {
  const home = os.homedir();
  const configDir = path.join(home, '.config', 'Monothematic');
  const configFile = path.join(configDir, 'config.json');
  const userTemplatesDir = path.join(configDir, 'templates');
  const outputDir = path.join(configDir, 'themes');
  const schemeFile = path.join(configDir, 'colors.json');
  return { configDir, configFile, userTemplatesDir, outputDir, schemeFile };
}

function defaultConfig(): AppConfig {
  const { userTemplatesDir, outputDir } = getPaths();
  const home = os.homedir();
  return {
    noctaliaConfigPath: path.join(home, '.config', 'Noctalia', 'config.json'),
    templatesDir: userTemplatesDir,
    outputDir,
    mappings: [
      {
        name: 'noctalia',
        source: path.join(userTemplatesDir, 'noctalia-theme.json'),
        destination: path.join(home, '.config', 'Noctalia', 'theme.json'),
      },
      {
        name: 'niri',
        source: path.join(userTemplatesDir, 'niri.conf'),
        destination: path.join(home, '.config', 'niri', 'theme.conf'),
      },
      {
        name: 'gtk3',
        source: path.join(userTemplatesDir, 'gtk3.css'),
        destination: path.join(home, '.config', 'gtk-3.0', 'gtk.css'),
      },
      {
        name: 'gtk4',
        source: path.join(userTemplatesDir, 'gtk4.css'),
        destination: path.join(home, '.config', 'gtk-4.0', 'gtk.css'),
      },
    ],
  };
}

export async function ensureUserConfig() {
  const { configDir, configFile, userTemplatesDir } = getPaths();
  await fs.ensureDir(configDir);
  // Seed default config if missing
  if (!(await fs.pathExists(configFile))) {
    await fs.writeJson(configFile, defaultConfig(), { spaces: 2 });
  }
  // Seed templates from bundled templates if missing
  await fs.ensureDir(userTemplatesDir);
  const bundled = path.resolve(path.dirname(new URL(import.meta.url).pathname), '../../templates');
  // Copy only if directory is empty
  const entries = await fs.readdir(userTemplatesDir);
  if (entries.length === 0 && (await fs.pathExists(bundled))) {
    await fs.copy(bundled, userTemplatesDir, { overwrite: false, errorOnExist: false });
  }
}

export async function getConfig(): Promise<AppConfig> {
  const { configFile } = getPaths();
  const cfg = await fs.readJson(configFile).catch(() => null);
  return (cfg as AppConfig) ?? defaultConfig();
}

export function getNoctaliaConfigPath(cfg: AppConfig) {
  return cfg.noctaliaConfigPath;
}

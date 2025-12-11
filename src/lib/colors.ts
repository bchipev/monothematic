import fs from 'fs-extra';
import path from 'path';
import sharp from 'sharp';
import { converter, formatHex, formatCss } from 'culori';
import { getPaths } from './config.js';

export type Oklch = { mode: 'oklch'; l: number; c: number; h: number };

export type NamedColor = {
  name: string;
  oklch: Oklch;
  hex: string;
  css: string;
};

export type Scheme = {
  base: NamedColor[]; // base-02 .. base-98
  error: { light: NamedColor; dark: NamedColor };
  warning: { light: NamedColor; dark: NamedColor };
  success: { light: NamedColor; dark: NamedColor };
};

const toOklch = converter('oklch');
const toRgb = converter('rgb');

function clamp01(x: number) {
  return Math.max(0, Math.min(1, x));
}

export async function extractDominantOklch(imagePath: string): Promise<Oklch> {
  const stats = await sharp(imagePath).stats();
  const d = stats.dominant; // { r,g,b }
  const rgb = { mode: 'rgb', r: d.r / 255, g: d.g / 255, b: d.b / 255 } as const;
  const o = toOklch(rgb) as any;
  // Ensure valid values
  const l = clamp01(o.l);
  const c = Math.max(0, o.c ?? 0);
  const h = (o.h ?? 0) % 360;
  return { mode: 'oklch', l, c, h };
}

const LIGHTNESS_STOPS: number[] = Array.from({ length: 25 }, (_, i) => 98 - i * 4).filter((L) => L >= 2);

// Hues for accents in OKLCH (degrees)
const H_RED = 29;
const H_YELLOW = 109;
const H_GREEN = 142;

function oklchToNamed(name: string, o: Oklch): NamedColor {
  const rgb = toRgb(o) as any;
  const hex = formatHex(rgb);
  const css = formatCss(o);
  return { name, oklch: o, hex, css };
}

function makeOklch(l: number, c: number, h: number): Oklch {
  return { mode: 'oklch', l: l / 100, c, h };
}

export function generateScheme(seed: Oklch): Scheme {
  const C = Math.max(0.03, Math.min(seed.c, 0.25));
  const H = ((seed.h % 360) + 360) % 360;

  const base: NamedColor[] = LIGHTNESS_STOPS.map((L) => {
    const name = `base-${String(L).padStart(2, '0')}`;
    return oklchToNamed(name, makeOklch(L, C, H));
  });

  const errorLight = oklchToNamed('error-70', makeOklch(70, C, H_RED));
  const errorDark = oklchToNamed('error-30', makeOklch(30, C, H_RED));
  const warningLight = oklchToNamed('warning-70', makeOklch(70, C, H_YELLOW));
  const warningDark = oklchToNamed('warning-30', makeOklch(30, C, H_YELLOW));
  const successLight = oklchToNamed('success-70', makeOklch(70, C, H_GREEN));
  const successDark = oklchToNamed('success-30', makeOklch(30, C, H_GREEN));

  return {
    base,
    error: { light: errorLight, dark: errorDark },
    warning: { light: warningLight, dark: warningDark },
    success: { light: successLight, dark: successDark },
  };
}

export async function writeSchemeJson(scheme: Scheme) {
  const { schemeFile } = getPaths();
  const json: any = {};
  for (const b of scheme.base) {
    json[b.name] = { oklch: b.css, hex: b.hex };
  }
  json['error-70'] = { oklch: scheme.error.light.css, hex: scheme.error.light.hex };
  json['error-30'] = { oklch: scheme.error.dark.css, hex: scheme.error.dark.hex };
  json['warning-70'] = { oklch: scheme.warning.light.css, hex: scheme.warning.light.hex };
  json['warning-30'] = { oklch: scheme.warning.dark.css, hex: scheme.warning.dark.hex };
  json['success-70'] = { oklch: scheme.success.light.css, hex: scheme.success.light.hex };
  json['success-30'] = { oklch: scheme.success.dark.css, hex: scheme.success.dark.hex };

  await fs.ensureDir(path.dirname(schemeFile));
  await fs.writeJson(schemeFile, json, { spaces: 2 });
}

export function allSchemeColors(s: Scheme): NamedColor[] {
  return [
    ...s.base,
    s.error.light,
    s.error.dark,
    s.warning.light,
    s.warning.dark,
    s.success.light,
    s.success.dark,
  ];
}

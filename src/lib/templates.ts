import fs from 'fs-extra';
import path from 'path';
import { converter, parse, formatCss, formatHex } from 'culori';
import { allSchemeColors, type NamedColor, type Scheme } from './colors.js';
import { getConfig } from './config.js';

const toOklch = converter('oklch');

type Match = {
  start: number;
  end: number;
  raw: string;
  kind: 'hex' | 'oklch';
  l: number; // lightness 0..1
};

const HEX_RE = /#(?:[0-9a-fA-F]{3}|[0-9a-fA-F]{6})\b/g;
const OKLCH_RE = /oklch\(([^)]+)\)/gi;

function findColorMatches(text: string): Match[] {
  const out: Match[] = [];
  let m: RegExpExecArray | null;
  // Hex
  while ((m = HEX_RE.exec(text))) {
    const raw = m[0];
    const c = parse(raw);
    const o = toOklch(c as any) as any;
    if (o && typeof o.l === 'number') {
      out.push({ start: m.index, end: m.index + raw.length, raw, kind: 'hex', l: o.l });
    }
  }
  // oklch()
  while ((m = OKLCH_RE.exec(text))) {
    const raw = m[0];
    const c = parse(raw);
    const o = toOklch(c as any) as any;
    if (o && typeof o.l === 'number') {
      out.push({ start: m.index, end: m.index + raw.length, raw, kind: 'oklch', l: o.l });
    }
  }
  // Sort by start to allow left-to-right replacement
  out.sort((a, b) => a.start - b.start);
  return out;
}

function nearestByLightness(matches: Match[], palette: NamedColor[]): string[] {
  return matches.map((m) => {
    let best: NamedColor | null = null;
    let bestD = Infinity;
    for (const p of palette) {
      const d = Math.abs(p.oklch.l - m.l);
      if (d < bestD) {
        bestD = d;
        best = p;
      }
    }
    const chosen = best!;
    return m.kind === 'oklch' ? chosen.css : chosen.hex;
  });
}

export async function applyTemplates(scheme: Scheme) {
  const cfg = await getConfig();
  const palette = allSchemeColors(scheme);

  for (const map of cfg.mappings) {
    const src = map.source;
    if (!(await fs.pathExists(src))) continue;
    const content = await fs.readFile(src, 'utf8');
    const matches = findColorMatches(content);
    if (matches.length === 0) {
      // Write original content through, ensuring destination exists
      await fs.ensureDir(path.dirname(map.destination));
      await fs.writeFile(map.destination, content, 'utf8');
      continue;
    }
    const replacements = nearestByLightness(matches, palette);
    // Build new content by walking matches
    let cursor = 0;
    let out = '';
    matches.forEach((m, i) => {
      out += content.slice(cursor, m.start);
      out += replacements[i];
      cursor = m.end;
    });
    out += content.slice(cursor);
    await fs.ensureDir(path.dirname(map.destination));
    await fs.writeFile(map.destination, out, 'utf8');
  }
}

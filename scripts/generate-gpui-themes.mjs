// generate-gpui-themes.mjs — FE TS → checked-in Rust theme tables (Phase 6 D1).
//
// Reads:
//   fe/src/features/settings/data/ui-themes.ts        (102 UI presets)
//   fe/src/features/settings/data/terminal-themes.ts  (78 terminal presets)
// Writes:
//   desktop-gpui/crates/webtmux/src/themes_generated.rs
//
// Manual regen only (checked-in output; `cargo build` never runs this):
//   node scripts/generate-gpui-themes.mjs
// Run from the repo root. Requires node (FE Vite toolchain already does).
// If node is absent: hand-port the rows on another machine and copy the file,
// then note the fallback path in the plan SUMMARY (per 06-01 A1).
//
// Supply chain: local-only — reads two in-repo files, writes one in-repo
// file. Output is a reviewed diff; `tests/theme_test.rs` pins 102/78 lengths,
// every UI `terminalTheme` link resolution, and the
// `default-dark.background == 0x1e1e1e` spot-check (T-06-01).

import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..");
const uiPath = join(root, "fe/src/features/settings/data/ui-themes.ts");
const termPath = join(root, "fe/src/features/settings/data/terminal-themes.ts");
const outPath = join(root, "desktop-gpui/crates/webtmux/src/themes_generated.rs");

function hexToU32(hex) {
  const h = hex.replace("#", "").trim().toLowerCase();
  const full = h.length === 3 ? [...h].map((c) => c + c).join("") : h;
  const v = Number.parseInt(full, 16);
  if (Number.isNaN(v)) throw new Error(`bad hex color: ${hex}`);
  return `0x${v.toString(16).padStart(6, "0")}`;
}

// Port of FE `themeLuminance` (terminal-themes.ts:2004-2015): WCAG weights.
function themeLuminance(hex) {
  const h = hex.replace("#", "").trim();
  const full = h.length === 3 ? [...h].map((c) => c + c).join("") : h;
  const num = Number.parseInt(full, 16);
  if (Number.isNaN(num)) return 0.5;
  const r = ((num >> 16) & 0xff) / 255;
  const g = ((num >> 8) & 0xff) / 255;
  const b = (num & 0xff) / 255;
  const lin = (c) => (c <= 0.03928 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4));
  return 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b);
}

function rs(s) {
  return `"${s.replace(/\\/g, "\\\\").replace(/"/g, '\\"')}"`;
}

const UI_COLOR_KEYS = [
  ["background", "background"],
  ["foreground", "foreground"],
  ["card", "card"],
  ["cardForeground", "card_foreground"],
  ["primary", "primary"],
  ["primaryForeground", "primary_foreground"],
  ["secondary", "secondary"],
  ["secondaryForeground", "secondary_foreground"],
  ["muted", "muted"],
  ["mutedForeground", "muted_foreground"],
  ["accent", "accent"],
  ["accentForeground", "accent_foreground"],
  ["destructive", "destructive"],
  ["destructiveForeground", "destructive_foreground"],
  ["border", "border"],
  ["input", "input"],
  ["ring", "ring"],
];

const TERM_COLOR_KEYS = [
  ["foreground", "foreground"],
  ["background", "background"],
  ["cursor", "cursor"],
  ["black", "black"],
  ["red", "red"],
  ["green", "green"],
  ["yellow", "yellow"],
  ["blue", "blue"],
  ["magenta", "magenta"],
  ["cyan", "cyan"],
  ["white", "white"],
  ["brightBlack", "bright_black"],
  ["brightRed", "bright_red"],
  ["brightGreen", "bright_green"],
  ["brightYellow", "bright_yellow"],
  ["brightBlue", "bright_blue"],
  ["brightMagenta", "bright_magenta"],
  ["brightCyan", "bright_cyan"],
  ["brightWhite", "bright_white"],
];

function unesc(s) {
  return s.replace(/\\'/g, "'").replace(/\\\\/g, "\\");
}

function parseBlocks(src, withLink) {
  // Matches one preset object: name/label/(terminalTheme)/colors:{...}.
  // Labels may contain an escaped quote (e.g. 'Synthwave \'84').
  const str = "'((?:[^'\\\\]|\\\\.)*)'";
  const re = withLink
    ? new RegExp(
        `\\{\\s*name:\\s*${str}\\s*,\\s*label:\\s*${str}\\s*,\\s*terminalTheme:\\s*${str}\\s*,\\s*colors:\\s*\\{([^}]+)\\}`,
        "g",
      )
    : new RegExp(
        `\\{\\s*name:\\s*${str}\\s*,\\s*label:\\s*${str}\\s*,\\s*colors:\\s*\\{([^}]+)\\}`,
        "g",
      );
  const out = [];
  let m;
  while ((m = re.exec(src)) !== null) {
    if (withLink) {
      const [, name, label, terminalTheme, colorsBody] = m;
      out.push({
        name: unesc(name),
        label: unesc(label),
        terminalTheme: unesc(terminalTheme),
        colorsBody,
      });
    } else {
      const [, name, label, colorsBody] = m;
      out.push({ name: unesc(name), label: unesc(label), colorsBody });
    }
  }
  return out;
}

function parseColors(body) {
  const map = new Map();
  const re = /(\w+):\s*'([^']+)'/g;
  let m;
  while ((m = re.exec(body)) !== null) map.set(m[1], m[2]);
  return map;
}

const uiSrc = readFileSync(uiPath, "utf8");
const termSrc = readFileSync(termPath, "utf8");

const uiBlocks = parseBlocks(uiSrc, true);
const termBlocks = parseBlocks(termSrc, false);

if (uiBlocks.length !== 102) {
  console.error(`expected 102 UI presets, found ${uiBlocks.length}`);
  process.exit(1);
}
if (termBlocks.length !== 78) {
  console.error(`expected 78 terminal presets, found ${termBlocks.length}`);
  process.exit(1);
}

const uiRows = uiBlocks.map((b) => {
  const colors = parseColors(b.colorsBody);
  const fields = UI_COLOR_KEYS.map(([ts, rsName]) => {
    const v = colors.get(ts);
    if (!v) throw new Error(`UI preset '${b.name}' missing color '${ts}'`);
    return `        ${rsName}: ${hexToU32(v)},`;
  });
  const isDark = themeLuminance(colors.get("background")) <= 0.5;
  return [
    `    UiThemePreset {`,
    `        name: ${rs(b.name)},`,
    `        label: ${rs(b.label)},`,
    `        terminal_theme: ${rs(b.terminalTheme)},`,
    `        is_dark: ${isDark},`,
    ...fields,
    `    },`,
  ].join("\n");
});

const termRows = termBlocks.map((b) => {
  const colors = parseColors(b.colorsBody);
  const fields = TERM_COLOR_KEYS.map(([ts, rsName]) => {
    const v = colors.get(ts);
    if (!v) throw new Error(`terminal preset '${b.name}' missing color '${ts}'`);
    return `        ${rsName}: ${hexToU32(v)},`;
  });
  return [
    `    TerminalThemePreset {`,
    `        name: ${rs(b.name)},`,
    `        label: ${rs(b.label)},`,
    ...fields,
    `    },`,
  ].join("\n");
});

const out = `//! Generated theme tables (Phase 6 D1) — DO NOT HAND-EDIT.
//!
//! Source: \`fe/src/features/settings/data/ui-themes.ts\` (102 presets) +
//! \`fe/src/features/settings/data/terminal-themes.ts\` (78 presets).
//! Generator: \`scripts/generate-gpui-themes.mjs\` (manual regen only):
//! \`node scripts/generate-gpui-themes.mjs\` from the repo root, then commit
//! the diff. \`cargo build\` never invokes the generator (no build.rs).
//! \`is_dark\` is precomputed with the FE \`themeLuminance\` rule (WCAG
//! weights, light when \`> 0.5\`).

/// One UI chrome preset: stable key + display label + linked terminal preset
/// name + precomputed dark bucket + 17 chrome colors as \`0xRRGGBB\`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UiThemePreset {
    pub name: &'static str,
    pub label: &'static str,
    pub terminal_theme: &'static str,
    pub is_dark: bool,
    pub background: u32,
    pub foreground: u32,
    pub card: u32,
    pub card_foreground: u32,
    pub primary: u32,
    pub primary_foreground: u32,
    pub secondary: u32,
    pub secondary_foreground: u32,
    pub muted: u32,
    pub muted_foreground: u32,
    pub accent: u32,
    pub accent_foreground: u32,
    pub destructive: u32,
    pub destructive_foreground: u32,
    pub border: u32,
    pub input: u32,
    pub ring: u32,
}

/// One terminal ANSI preset: stable key + label + 19 colors as \`0xRRGGBB\`
/// (foreground/background/cursor + 16 ANSI).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalThemePreset {
    pub name: &'static str,
    pub label: &'static str,
    pub foreground: u32,
    pub background: u32,
    pub cursor: u32,
    pub black: u32,
    pub red: u32,
    pub green: u32,
    pub yellow: u32,
    pub blue: u32,
    pub magenta: u32,
    pub cyan: u32,
    pub white: u32,
    pub bright_black: u32,
    pub bright_red: u32,
    pub bright_green: u32,
    pub bright_yellow: u32,
    pub bright_blue: u32,
    pub bright_magenta: u32,
    pub bright_cyan: u32,
    pub bright_white: u32,
}

pub const UI_THEMES: &[UiThemePreset] = &[
${uiRows.join("\n")}
];

pub const TERMINAL_THEMES: &[TerminalThemePreset] = &[
${termRows.join("\n")}
];

/// FE \`getUiTheme\` parity: unknown/missing names fall back to \`UI_THEMES[0]\`
/// (\`default-dark\`).
pub fn ui_preset_by_name(name: &str) -> &'static UiThemePreset {
    UI_THEMES.iter().find(|t| t.name == name).unwrap_or(&UI_THEMES[0])
}

/// Terminal preset lookup with \`TERMINAL_THEMES[0]\` fallback (GPUI-side
/// total function; FE \`getTerminalTheme\` returns null, which has no
/// meaning for a \`&'static\` return).
pub fn terminal_preset_by_name(name: &str) -> &'static TerminalThemePreset {
    TERMINAL_THEMES
        .iter()
        .find(|t| t.name == name)
        .unwrap_or(&TERMINAL_THEMES[0])
}

/// FE \`resolvedTerminalTheme\` parity: the terminal always follows the UI
/// theme through its \`terminal_theme\` link (legacy explicit override is
/// intentionally ignored — one theme for the whole app).
pub fn terminal_preset_for_ui(ui_name: &str) -> &'static TerminalThemePreset {
    let ui = ui_preset_by_name(ui_name);
    terminal_preset_by_name(ui.terminal_theme)
}

/// Ported FE \`themeLuminance\` (WCAG weights) over a \`0xRRGGBB\` background.
/// Light when \`> 0.5\` (matches \`isLightUiTheme\`/\`isLightTheme\`).
pub fn theme_luminance(background: u32) -> f64 {
    let r = (((background >> 16) & 0xff) as f64) / 255.0;
    let g = (((background >> 8) & 0xff) as f64) / 255.0;
    let b = ((background & 0xff) as f64) / 255.0;
    let lin = |c: f64| {
        if c <= 0.03928 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b)
}

/// Dark/light bucket for a \`0xRRGGBB\` background (FE filter parity).
pub fn is_light_background(background: u32) -> bool {
    theme_luminance(background) > 0.5
}
`;

writeFileSync(outPath, out);
console.log(`wrote ${outPath} (${uiBlocks.length} UI + ${termBlocks.length} terminal presets)`);


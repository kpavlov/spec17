// The contrast audit of the shared palette (docs/static/tokens.css), for both themes. WCAG 2.2 AA is the floor;
// APCA (the 0.98G constants) is the design target for body and secondary text.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const css = readFileSync(new URL("../../docs/static/tokens.css", import.meta.url), "utf8");
const pairs = Object.fromEntries([...css.matchAll(/--([\w-]+):\s*light-dark\((#[0-9a-f]{6}),\s*(#[0-9a-f]{6})\)/gi)].map((m) => [m[1], [m[2], m[3]]]));
const fixed = Object.fromEntries([...css.matchAll(/--([\w-]+-(?:ink|fill)):\s*(#[0-9a-f]{6})/gi)].map((m) => [m[1], m[2]]));

const rgb = (hex) => [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16) / 255);
const lum = (hex) => rgb(hex).map((c) => (c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4)).reduce((s, c, i) => s + c * [0.2126, 0.7152, 0.0722][i], 0);
const wcag = (a, b) => { const [hi, lo] = [lum(a), lum(b)].sort((x, y) => y - x); return (hi + 0.05) / (lo + 0.05); };
const apca = (text, bg) => {
  const y = (hex) => { const v = rgb(hex).reduce((s, c, i) => s + c ** 2.4 * [0.2126729, 0.7151522, 0.072175][i], 0); return v < 0.022 ? v + (0.022 - v) ** 1.414 : v; };
  const t = y(text), b = y(bg);
  const s = (b > t ? b ** 0.56 - t ** 0.57 : b ** 0.65 - t ** 0.62) * 1.14;
  return Math.abs(s) < 0.1 ? 0 : Math.abs(s - Math.sign(s) * 0.027) * 100;
};

const THEMES = ["light", "dark"];
const SURFACES = ["bg", "card", "raised"];
const check = (names, on, floor, measure, what) => {
  const bad = [];
  for (const [t, theme] of THEMES.entries())
    for (const name of names)
      for (const surface of on) {
        assert.ok(pairs[name] && pairs[surface], `--${name} and --${surface} are light-dark() pairs`);
        const got = measure(pairs[name][t], pairs[surface][t]);
        if (got < floor) bad.push(`${theme}: --${name} on --${surface} is ${what} ${got.toFixed(1)}, needs ${floor}`);
      }
  assert.deepEqual(bad, []);
};

test("the APCA formula matches its reference values", () => {
  assert.equal(apca("#888888", "#ffffff").toFixed(1), "63.1");
  assert.equal(apca("#ffffff", "#000000").toFixed(1), "107.9");
});

// [WEB-63]
test("body text reaches WCAG 7:1 and APCA Lc 75", () => {
  check(["fg"], SURFACES, 7, wcag, "WCAG");
  check(["fg"], SURFACES, 75, apca, "Lc");
});

// [WEB-63]
test("secondary text reaches WCAG 4.5:1 and APCA Lc 60", () => {
  check(["muted"], SURFACES, 4.5, wcag, "WCAG");
  check(["muted"], ["bg", "card"], 60, apca, "Lc");
});

// [WEB-63]
test("coloured text reaches WCAG 4.5:1, control borders and the focus ring 3:1", () => {
  check(["accent", "brand", "accent-2", "accent-3", "ok", "warn", "bad", "info"], ["bg", "card"], 4.5, wcag, "WCAG");
  check(["line-strong", "accent"], ["bg", "card"], 3, wcag, "WCAG");
});

// [WEB-63]
test("accent text stays at WCAG 4.5:1 on the tints: selected row, hover, the selected item", () => {
  for (const name of ["tint-sel", "tint-btn", "tint-on"]) {
    const [, percent, under] = css.match(new RegExp(`--${name}:\\s*color-mix\\(in srgb, var\\(--accent\\) (\\d+)%, var\\(--([\\w-]+)\\)`));
    const p = Number(percent) / 100;
    for (const [t, theme] of THEMES.entries()) {
      const accent = pairs.accent[t], base = rgb(pairs[under][t]);
      const tint = "#" + rgb(accent).map((c, i) => Math.round((c * p + base[i] * (1 - p)) * 255).toString(16).padStart(2, "0")).join("");
      assert.ok(wcag(accent, tint) >= 4.5, `${theme}: --${name} ${wcag(accent, tint).toFixed(2)}`);
    }
  }
});

// [WEB-63]
test("every pill ink reaches WCAG 4.5:1 on its fill", () => {
  const inks = Object.keys(fixed).filter((k) => k.endsWith("-ink"));
  assert.ok(inks.length > 5);
  for (const k of inks) assert.ok(wcag(fixed[k], fixed[k.replace("-ink", "-fill")]) >= 4.5, k);
});

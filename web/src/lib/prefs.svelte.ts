// (WEB-68) how the dashboard looks, one choice per browser (localStorage). web/index.html applies the same keys before the first paint.
import { stored } from "$lib/store";

export type Pref = "theme" | "density" | "textsize" | "font";

// the one list of choices, as Settings and the command palette name them; `def` is not stored and puts nothing on <html>
export const DISPLAY: { key: Pref; label: string; def: string; options: { key: string; label: string }[] }[] = [
  { key: "theme", label: "Theme", def: "auto", options: [{ key: "auto", label: "System" }, { key: "light", label: "Light" }, { key: "dark", label: "Dark" }] },
  { key: "density", label: "Row density", def: "comfortable", options: [{ key: "comfortable", label: "Comfortable" }, { key: "compact", label: "Compact" }] },
  { key: "textsize", label: "Text size", def: "100", options: ["90", "100", "112.5", "125"].map((v) => ({ key: v, label: `${v}%` })) },
  { key: "font", label: "Font", def: "system", options: [{ key: "system", label: "System" }, { key: "legible", label: "High legibility" }] },
];
const field = (key: Pref) => DISPLAY.find((f) => f.key === key)!;

const read = (key: Pref): string => {
  const value = stored(`spec17.${key}`);
  return field(key).options.some((o) => o.key === value) ? value : field(key).def;
};
export const prefs = $state<Record<Pref, string>>({ theme: read("theme"), density: read("density"), textsize: read("textsize"), font: read("font") });

export function setPref(key: Pref, value: string) {
  prefs[key] = value;
  const el = document.documentElement, off = value === field(key).def;
  if (key === "textsize") el.style.fontSize = off ? "" : `${value}%`;
  else if (off) delete el.dataset[key];
  else el.dataset[key] = value;
  try { off ? localStorage.removeItem(`spec17.${key}`) : localStorage.setItem(`spec17.${key}`, value); } catch { /* storage blocked */ }
}

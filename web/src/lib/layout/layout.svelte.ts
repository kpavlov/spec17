// (WEB-52) Split-pane layout shared by every page that lists on one side and previews on the other:
// the orientation is one choice for the whole dashboard, the size of the list pane is remembered per page `name` ("" = requirements and tests).
const P = "spec17.splitpos";
const clampPos = (n: number) => Math.min(85, Math.max(15, Math.round(n)));
const read = (k: string) => { try { return localStorage.getItem(k); } catch { return null; } };
const write = (k: string, v: string | null) => { try { v === null ? localStorage.removeItem(k) : localStorage.setItem(k, v); } catch { /* storage blocked */ } };

const loadPos = () => {
  const out: Record<string, number> = {};
  try {
    for (let i = 0; i < localStorage.length; i++) {
      const k = localStorage.key(i)!;
      const n = Number(localStorage.getItem(k));
      if (k.startsWith(P) && n) out[k.slice(P.length)] = clampPos(n);
    }
  } catch { /* storage blocked */ }
  return out;
};

export const layout = $state<{ split: "v" | "h"; pos: Record<string, number> }>({ split: read("spec17.split") === "h" ? "h" : "v", pos: loadPos() });

export const toggleSplit = () => { layout.split = layout.split === "v" ? "h" : "v"; write("spec17.split", layout.split); };
// percent of the list pane: dragged, else `def` beside the preview and at most half on top of it
export const listPct = (name: string, def: number) => layout.pos[name] ?? (layout.split === "v" ? def : Math.min(50, def));
export const setPos = (name: string, n: number | null, keep = true) => {
  if (n === null) delete layout.pos[name]; else layout.pos[name] = clampPos(n);
  if (keep) write(P + name, n === null ? null : String(layout.pos[name]));
};
// beside (wide screens, orientation "v") or stacked
export const sideBySide = () => layout.split === "v" && matchMedia("(min-width: 1000px)").matches;

// What the preview of a changed item shows: the item as it is, or what changed against HEAD. One choice for every preview.
export const preview = $state<{ view: "actual" | "diff" }>({ view: read("spec17.view") === "diff" ? "diff" : "actual" });
// Whether long diff lines wrap or scroll sideways; remembered.
export const diffWrap = $state({ on: read("spec17.diffwrap") === "1" });
export const setDiffWrap = (on: boolean) => { diffWrap.on = on; write("spec17.diffwrap", on ? "1" : null); };
export const setView = (v: string) => { preview.view = v === "diff" ? "diff" : "actual"; write("spec17.view", preview.view); };

// A collapsible Section with an `id` remembers per browser whether it is open and the height (px) its bottom edge was dragged to, a limit: shorter content is never padded; `h: null` = no limit.
export type SectionState = { open: boolean; h: number | null };
export const loadSection = (id: string, open: boolean): SectionState => {
  try {
    const v = JSON.parse(read("spec17.section." + id) ?? "{}");
    return { open: typeof v.open === "boolean" ? v.open : open, h: typeof v.h === "number" && v.h > 0 ? v.h : null };
  } catch { return { open, h: null }; }
};
// the defaults are not stored
export const saveSection = (id: string, s: SectionState, open: boolean) => write("spec17.section." + id, s.open === open && s.h === null ? null : JSON.stringify(s));


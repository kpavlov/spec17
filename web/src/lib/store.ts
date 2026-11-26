// localStorage that works (as a no-op) where the browser blocks it
export const stored = (k: string) => { try { return localStorage.getItem(k) ?? ""; } catch { return ""; } };
export const save = (k: string, v: string) => { try { localStorage.setItem(k, v); } catch { /* storage blocked */ } };
export const plural = (n: number, w: string) => `${n} ${w}${n === 1 ? "" : "s"}`;
export const plain = (t: string) => t.replace(/[`*~]/g, "");

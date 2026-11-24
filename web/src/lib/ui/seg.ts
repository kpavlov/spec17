// What Seg.svelte decides without a DOM.

/** A click with Shift, Ctrl or ⌘ held adds to or removes from a multi-choice group; a plain click replaces its selection (WEB-60). */
export const additive = (e: { shiftKey: boolean; ctrlKey: boolean; metaKey: boolean }) => e.shiftKey || e.ctrlKey || e.metaKey;

/** The key an arrow moves to in a single-choice group. It wraps around, and with nothing selected starts from the first (→) or last (←). */
export function step(keys: string[], current: string | undefined, delta: 1 | -1): string {
  const i = keys.indexOf(current ?? "");
  if (i < 0) return delta > 0 ? keys[0] : keys[keys.length - 1];
  return keys[(i + delta + keys.length) % keys.length];
}

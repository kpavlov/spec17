// (WEB-35, WEB-52) Reviewed flags: {requirement ID: the commit it had when it was reviewed}, one record per repo in IndexedDB.
// Where storage is blocked the flags still work, for this session.
import { db, run, update } from "$lib/db.svelte";
import { applyMarks, type Marks, type Edits } from "$lib/marks";

export const reviewed = $state<Marks>({});

// edits made before the stored record has loaded; non-null only while loading
let pending: Edits | null = new Map();

// only the edits go onto the stored record, not this tab's copy over it: another tab may have saved since
const save = (root: string, edits: Edits) => update<Marks>("reviewed", root, (old) => applyMarks(old ?? {}, edits)).catch(() => {});

/** Stores the marks held now, as after a database reset (WEB-72). */
export const storeReviewed = () => { if (db.root !== null) save(db.root, new Map(Object.entries(reviewed))); };

export async function loadReviewed(root: string) {
  if (!root) return; // no root yet: db.root stays null and nothing is persisted
  try { Object.assign(reviewed, (await run<Record<string, string> | undefined>("reviewed", "readonly", (s) => s.get(root))) ?? {}); } catch { /* storage blocked */ }
  // what the user did meanwhile wins over the stored record
  const edits: Edits = pending ?? new Map();
  pending = null;
  applyMarks(reviewed, edits);
  db.root = root;
  if (edits.size) save(root, edits);
}

/** Mark `id` reviewed at `commit`, or clear it. */
export function setReviewed(id: string, commit?: string) {
  const edit: Edits = new Map([[id, commit]]);
  applyMarks(reviewed, edit);
  if (pending) pending.set(id, commit);
  else if (db.root !== null) save(db.root, edit);
}

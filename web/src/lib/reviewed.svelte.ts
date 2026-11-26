// (WEB-35, WEB-52) Reviewed flags: {requirement ID: the commit it had when it was reviewed}, one record per repo in IndexedDB.
// Where storage is blocked the flags still work, for this session.
import { db, run } from "$lib/db.svelte";

export const reviewed = $state<Record<string, string>>({});

// edits made before the stored record has loaded (undefined = cleared); non-null only while loading
let pending: Map<string, string | undefined> | null = new Map();

const save = (root: string) => run("reviewed", "readwrite", (s) => s.put($state.snapshot(reviewed), root)).catch(() => {});

/** Stores the marks held now, as after a database reset (WEB-72). */
export const storeReviewed = () => { if (db.root !== null) save(db.root); };

export async function loadReviewed(root: string) {
  if (!root) return; // no root yet: db.root stays null and nothing is persisted
  try { Object.assign(reviewed, (await run<Record<string, string> | undefined>("reviewed", "readonly", (s) => s.get(root))) ?? {}); } catch { /* storage blocked */ }
  // what the user did meanwhile wins over the stored record
  const edits = pending ?? new Map<string, string | undefined>();
  pending = null;
  for (const [id, commit] of edits) if (commit) reviewed[id] = commit; else delete reviewed[id];
  db.root = root;
  if (edits.size) save(root);
}

/** Mark `id` reviewed at `commit`, or clear it. */
export function setReviewed(id: string, commit?: string) {
  if (commit) reviewed[id] = commit; else delete reviewed[id];
  if (pending) { pending.set(id, commit); return; }
  if (db.root !== null) save(db.root);
}

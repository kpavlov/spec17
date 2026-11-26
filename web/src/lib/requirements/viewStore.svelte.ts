// (WEB-71, WEB-52) saved views, one record per repo in IndexedDB. Where storage is blocked they last for the session.
import { db, run } from "$lib/db.svelte";
import { nav, navigate } from "$lib/nav.svelte";
import { putView, viewQuery, VIEW_KEYS, type SavedView } from "$lib/requirements/views";

export const saved = $state<{ list: SavedView[] }>({ list: [] });

type Edit = (list: SavedView[]) => SavedView[];
// edits made before the root was known: stored once it is
const early: Edit[] = [];

// the edit is applied to the stored list in one transaction, never this tab's copy over it: another tab may have saved since
async function write(root: string, edit: Edit) {
  try {
    const before = await run<SavedView[] | undefined>("views", "readwrite", (s) => {
      const get = s.get(root);
      get.onsuccess = () => s.put(edit(get.result ?? []), root);
      return get;
    });
    saved.list = edit(before ?? []);
  } catch { /* storage blocked */ }
}
function change(edit: Edit) {
  saved.list = edit(saved.list); // at once, also without storage
  if (db.root === null) early.push(edit); else write(db.root, edit);
}

export async function loadViews(root: string) {
  if (!root) return;
  const queued = early.splice(0);
  if (queued.length) return write(root, (list) => queued.reduce((l, edit) => edit(l), list));
  try { saved.list = (await run<SavedView[] | undefined>("views", "readonly", (s) => s.get(root))) ?? saved.list; } catch { /* storage blocked */ }
}

/** Stores the views held now, as after a database reset (WEB-72). */
export function storeViews() {
  const mine = $state.snapshot(saved.list);
  if (db.root !== null && mine.length) write(db.root, (list) => mine.reduce(putView, list));
}

export const currentQuery = () => viewQuery(nav.route);
export function saveView(name: string) {
  const view: SavedView = { name, tab: nav.route.tab === "changes" ? "changes" : "requirements", query: currentQuery() };
  change((list) => putView(list, view));
}
export const deleteView = (name: string) => change((list) => list.filter((v) => v.name !== name));
/** Replaces the search and filters; on the view's own tab the rest of the page (the open requirement, the compare scope) stays. */
export function applyView(v: SavedView) {
  const base = nav.route.tab === v.tab && !nav.inDocs ? nav.route : { tab: v.tab };
  navigate({ ...base, ...Object.fromEntries(VIEW_KEYS.map((k) => [k, undefined])), ...v.query });
}

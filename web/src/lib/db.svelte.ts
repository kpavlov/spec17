// (WEB-52) Per-repo UI state in IndexedDB: the same dashboard origin can serve different roots, so every record is keyed by the root.
// `db.root` is null until the server has said which root this is (loadReviewed sets it); readers wait for it.
// `db.broken` names the failure while a reset would fix it (WEB-72).
import { resettable } from "$lib/dbError";

export const db = $state<{ root: string | null; broken: string }>({ root: null, broken: "" });

const STORES = ["reviewed", "collapsed", "views"];
// Each call opens and closes its own connection: an open one would block the upgrade in another tab, which then never settles.
export const run = <T>(store: string, mode: IDBTransactionMode, f: (s: IDBObjectStore) => IDBRequest<T>) =>
  new Promise<T>((ok, reject) => {
    const no = (e: unknown) => { if (resettable(e)) db.broken = (e as DOMException).name; reject(e); };
    const q = indexedDB.open("spec17", 3);
    q.onupgradeneeded = () => { for (const n of STORES) if (!q.result.objectStoreNames.contains(n)) q.result.createObjectStore(n); };
    q.onerror = () => no(q.error);
    q.onblocked = () => no(new Error("storage blocked by another tab"));
    q.onsuccess = () => {
      const c = q.result;
      try {
        const tx = c.transaction(store, mode);
        const r = f(tx.objectStore(store));
        tx.oncomplete = () => { c.close(); ok(r.result); };
        tx.onerror = tx.onabort = () => { c.close(); no(tx.error); };
      } catch (e) { c.close(); no(e); }
    };
  });

/** Replaces the record at `key` with `edit` of the stored one, in one transaction: a tab's own copy never goes over what another tab saved. */
export async function update<T>(store: string, key: string, edit: (old: T | undefined) => T): Promise<T> {
  let next!: T;
  await run<T | undefined>(store, "readwrite", (s) => {
    const get = s.get(key);
    get.onsuccess = () => s.put((next = edit(get.result)), key);
    return get;
  });
  return next;
}

/** (WEB-72) Deletes the whole database, every root's records: call it only once the user has agreed. */
export const deleteDb = (blocked: () => void) =>
  new Promise<void>((ok, no) => {
    const q = indexedDB.deleteDatabase("spec17");
    q.onsuccess = () => { db.broken = ""; ok(); };
    q.onerror = () => no(q.error);
    q.onblocked = blocked; // it goes ahead once the other tab lets go
  });

// (WEB-52) Per-repo UI state in IndexedDB: the same dashboard origin can serve different roots, so every record is keyed by the root.
// `db.root` is null until the server has said which root this is (loadReviewed sets it); readers wait for it.
export const db = $state<{ root: string | null }>({ root: null });

const STORES = ["reviewed", "collapsed"];
// Each call opens and closes its own connection: an open one would block the upgrade in another tab, which then never settles.
export const run = <T>(store: string, mode: IDBTransactionMode, f: (s: IDBObjectStore) => IDBRequest<T>) =>
  new Promise<T>((ok, no) => {
    const q = indexedDB.open("spec17", 2);
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

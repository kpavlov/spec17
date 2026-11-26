// (WEB-01) routing: the URL is the state
import { embedded } from "$lib/transport";
import { format, parse, type Route } from "$lib/router";
import { stored } from "$lib/store";

const here = (): Route => {
  try { return parse(new URL(location.href)); } catch { return { tab: "dashboard" }; }
};

class Nav {
  route = $state<Route>(here());
  /** the Documents view of Requirements */
  inDocs = $derived(this.route.tab === "requirements" && this.route.view === "docs");
  popped() { this.route = here(); }
}
export const nav = new Nav();
export const u = format;

export function navigate(r: Route, replace = false) {
  if (r !== nav.route) nav.route = r;
  if (embedded) return; // a host iframe has no URL of its own
  try { (replace ? window.history.replaceState : window.history.pushState).call(window.history, null, "", format(r)); } catch { /* sandboxed */ }
}
// typing in the filter replaces the URL instead of stacking a history entry per keystroke
export const filter = (patch: Partial<Route>) => navigate(Object.assign(nav.route, patch), true);

export const req = (id: string): Route => ({ tab: "requirements", id });
// a spec document (the Documents view of Requirements), optionally at one requirement
export const doc = (file?: string, id?: string, changed?: string): Route => ({ tab: "requirements", view: "docs", file, id, changed });
export const withId = (tab: Route["tab"], id: string): Route => ({ tab, id });

// (WEB-51) the Requirements view the navigation link restores
const kept = stored("spec17.requirements");
export const reqView = $state({ value: ["graph", "docs"].includes(kept) ? kept : "list" });

export const DOCS = "https://spec17.dev";

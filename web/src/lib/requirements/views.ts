// (WEB-71) what a saved view keeps: the tab, the search and the filters, never the selected requirement
export const VIEW_KEYS = ["state", "level", "priority", "effort", "q", "changed", "reviewed"] as const;
export type Query = Partial<Record<(typeof VIEW_KEYS)[number], string>>;
export type SavedView = { name: string; tab: "requirements" | "changes"; query: Query };

export const viewQuery = (route: Query): Query => Object.fromEntries(VIEW_KEYS.filter((k) => route[k] && !(k === "state" && route[k] === "all")).map((k) => [k, route[k]]));
// saving under an existing name replaces that view, in its place
export const putView = (list: SavedView[], view: SavedView): SavedView[] => (list.some((v) => v.name === view.name) ? list.map((v) => (v.name === view.name ? view : v)) : [...list, view]);

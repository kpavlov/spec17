// requirement table columns (the state colours the row edge) and their sort values; the classes size the one-line rows (views.css)
export const reqCols = [{ key: "id", label: "ID", cls: "req-id mono nowrap" }, { key: "status", label: "status", cls: "req-status" }, { key: "text", label: "requirement", cls: "req-text" }, { key: "file", label: "location", cls: "req-loc mono muted" }];
export const reqVal = (r: any, k: string): string | number => (k === "file" ? `${r.file}:${String(r.line).padStart(6, "0")}` : k === "status" ? r.state : r[k]);

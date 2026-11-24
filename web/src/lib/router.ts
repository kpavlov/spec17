// The URL is the state: every view and selection round-trips through parse/format, so back/forward, reload and shared links work.
//   /  (the dashboard; /report still works)   /requirements[/DOC-25][?state=none&level=MUST&q=text]   /requirements?view=docs[&file=04-documents.md&id=DOC-25] (the spec documents)   /modules[?module=core&q=text]   /tests[?requirement_id=DOC-25&module=core&state=tagged&q=text]
//   /code?path=RunSolo/RunSolo/A.swift&line=12
//   /overlap[?id=DOC-25]   /uncovered[?id=…]   /settings
//   /changes[?staged=1&untracked=0&ref=main&commit=SHA&ws=1&ignored=1&all=1]      ?changed=1 on requirements, tests and modules: only what the working tree changed   ?reviewed=0 on requirements: only the unreviewed ones
// File names go in the query, not the path, so a dev server never mistakes `x.md` or `A.swift` for a static file.
export const TABS = ["dashboard", "changes", "requirements", "modules", "tests", "overlap", "uncovered", "settings"] as const;
export type Tab = (typeof TABS)[number] | "code";

export type Route = { tab: Tab; id?: string; file?: string; path?: string; line?: number; view?: string; state?: string; level?: string; priority?: string; effort?: string; q?: string; module?: string; requirementId?: string; changed?: string; reviewed?: string; staged?: string; untracked?: string; ref?: string; commit?: string; ws?: string; ignored?: string; all?: string };

export function parse(url: URL): Route {
  const [raw, second] = url.pathname.split("/").filter(Boolean).map(decodeURIComponent);
  // old links: /setup is now /settings, /specs the documents view of /requirements
  const first = raw === "setup" ? "settings" : raw === "specs" ? "requirements" : raw;
  const tab = ([...TABS, "code"] as string[]).includes(first) ? (first as Tab) : "dashboard";
  const p = url.searchParams;
  const line = Number(p.get("line"));
  return {
    tab,
    id: (tab === "requirements" ? second : undefined) ?? p.get("id") ?? undefined,
    file: p.get("file") ?? undefined,
    path: p.get("path") ?? undefined,
    line: line > 0 ? line : undefined,
    view: p.get("view") ?? (raw === "specs" ? "docs" : undefined),
    state: p.get("state") ?? undefined,
    level: p.get("level") ?? undefined,
    priority: p.get("priority") ?? undefined,
    effort: p.get("effort") ?? undefined,
    q: p.get("q") ?? undefined,
    module: p.get("module") ?? undefined,
    requirementId: tab === "tests" ? p.get("requirement_id") ?? undefined : undefined,
    changed: p.get("changed") ?? undefined,
    reviewed: p.get("reviewed") ?? undefined,
    staged: p.get("staged") ?? undefined,
    untracked: p.get("untracked") ?? undefined,
    ref: p.get("ref") ?? undefined,
    commit: p.get("commit") ?? undefined,
    ws: p.get("ws") ?? undefined,
    ignored: p.get("ignored") ?? undefined,
    all: p.get("all") ?? undefined,
  };
}

export function format(r: Route): string {
  const q = new URLSearchParams();
  const put = (k: string, v?: string | number) => v !== undefined && v !== "" && q.set(k, String(v));
  if (r.tab !== "requirements") put("id", r.id);
  put("file", r.file);
  put("view", r.view);
  put("path", r.path);
  put("line", r.line);
  if (r.state && r.state !== "all") put("state", r.state);
  put("level", r.level);
  put("priority", r.priority);
  put("effort", r.effort);
  put("q", r.q);
  put("module", r.module);
  put("requirement_id", r.requirementId);
  put("changed", r.changed);
  put("reviewed", r.reviewed);
  put("staged", r.staged);
  put("untracked", r.untracked);
  put("ref", r.ref);
  put("commit", r.commit);
  put("ws", r.ws);
  put("ignored", r.ignored);
  put("all", r.all);
  const path = (r.tab === "dashboard" ? "/" : `/${r.tab}`) + (r.tab === "requirements" && r.id ? `/${encodeURIComponent(r.id)}` : "");
  const s = q.toString();
  return s ? `${path}?${s}` : path;
}

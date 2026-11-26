// What the server said about the repo, and what the views derive from it. One instance: every view reads the same scan.
import { untrack } from "svelte";
import { call, read } from "$lib/transport";
import { nav } from "$lib/nav.svelte";
import { plain } from "$lib/store";
import { reviewed, loadReviewed } from "$lib/reviewed.svelte";
import { loadViews } from "$lib/requirements/viewStore.svelte";

export type Ref = { file: string; line: number; text: string };
export type Row = { id: string; state: string; file: string; line: number; level: string; priority: string | null; effort: string | null; text: string; references: string[]; mark: string; done: boolean; verdict: string; built: boolean; issues: string[]; tests: Ref[]; cites: Ref[]; graph: string[] };

// (WEB-37) directly changed: its bullet, a tagged test or a cited unit differs from the base; one only reached through a test is indirect
export const isDirect = (r: any) => r.notes.some((n: any) => n.action !== "reached by a test");

class Data {
  rows = $state<Row[]>([]);
  report = $state<any>(null);
  // the working tree against HEAD: undefined until loaded, null when it cannot be read (not a git work tree)
  changes = $state<any>(undefined);
  // (WEB-35, WEB-36) {requirement ID: its newest commit (sha, short, author, date, subject)}; empty without git
  commits = $state<Record<string, any>>({});
  // the Changes page's own scope when it is not the default
  custom = $state<any>(undefined);
  // commits not on the base branch, for the Changes page
  history = $state<any>(null);
  modules = $state<any>(null);
  tests = $state<any>(null);
  overlap = $state<any[] | null>(null);
  uncovered = $state<any>(null);
  trace = $state<{ text: string; data: any } | null>(null);
  code = $state<{ path: string; text: string } | null>(null);
  specFiles = $state<string[] | null>(null);
  specLoaded = $state("");
  specMd = $state("");
  root = $state("");
  branch = $state("");
  loadedAt = $state(0);
  error = $state("");
  // server connection: null until known; the push stream sets it, and so does every successful load or poll (the only signal when embedded)
  up = $state<boolean | null>(null);
  // bumped by every load: the views refetch (keeping what they show until the new data lands)
  gen = $state(0);
  // (WEB-36) who last changed the line of a test: asked per file when a preview shows it; keyed by reload (gen), so a reload asks again
  blame = $state<Record<string, any>>({});

  noReqs = $derived(this.report && this.report.requirements + this.report.refs === 0);
  known = $derived(new Set(this.rows.map((r) => r.id)));
  byId = $derived(new Map(this.rows.map((r) => [r.id, r])));
  // (WEB-22) what the working tree changed: files by path, the kinds a requirement is touched by, the modules holding changed files
  fileOf = $derived(new Map<string, any>((this.changes?.files ?? []).map((f: any) => [f.path, f])));
  directReqs = $derived<any[]>((this.changes?.requirements ?? []).filter(isDirect));
  changeOf = $derived(new Map<string, any>(this.directReqs.map((r: any) => [r.id, r])));
  reqKinds = $derived(new Map<string, string[]>(this.directReqs.map((r: Row & { changed: string[] }) => [r.id, r.changed])));
  changedMods = $derived(new Set<string>((this.changes?.files ?? []).map((f: any) => moduleOfFile(f.path))));
  // The tests screen filters the detected cases; the requirement table uses the same cases for its links (WEB-39).
  detectedCases = $derived<any[]>((this.tests?.modules ?? []).flatMap((m: any) => m.files.flatMap((f: any) => f.tests.map((t: any) => ({ module: m.module, file: f.file, t })))));
  testedIds = $derived(new Set<string>(this.detectedCases.flatMap((c) => c.t.ids)));
  // the open document: the one named, else the one that holds the selected requirement, else the first
  docFile = $derived(nav.route.file ?? (nav.route.id ? this.byId.get(nav.route.id)?.file : undefined) ?? this.specFiles?.[0]);

  // the Changes page asks for its own scope when it is not the default (staged + unstaged + untracked)
  scope = $derived({
    ref: nav.route.ref ?? "", commit: nav.route.commit ?? "", staged: nav.route.staged === "1", include_untracked: nav.route.untracked !== "0",
    ignore_whitespace: nav.route.ws === "1", show_ignored: nav.route.ignored === "1", show_all: nav.route.all === "1",
  });
  defaultScope = $derived(!this.scope.ref && !this.scope.commit && !this.scope.staged && this.scope.include_untracked && !this.scope.ignore_whitespace && !this.scope.show_ignored && !this.scope.show_all);
  view = $derived(this.defaultScope ? this.changes : this.custom);
}
export const data = new Data();

export const moduleOfFile = (path: string): string => (data.modules?.modules ?? []).map((m: any) => m.path).filter((m: string) => m !== "." && path.startsWith(`${m}/`)).sort((a: string, b: string) => b.length - a.length)[0] ?? ".";
export const changeMark = (path: string): string => data.fileOf.get(path)?.change ?? "";

// (WEB-35) reviewed = its commit is the one it was reviewed at; committed again since, it is unreviewed again
export const isReviewed = (id: string) => !!data.commits[id] && reviewed[id] === data.commits[id].sha;
export const reviewTip = (id: string) => (isReviewed(id) ? `reviewed at ${data.commits[id].short}` : reviewed[id] ? `changed since reviewed (${reviewed[id].slice(0, 7)} → ${data.commits[id].short})` : "mark reviewed");

export const blameKey = (file: string, line: number) => `${data.gen}:${file}:${line}`;
export const wantBlame = (refs: { file: string; line: number }[]) => {
  const by = new Map<string, number[]>();
  for (const r of refs) if (!(blameKey(r.file, r.line) in data.blame)) by.set(r.file, [...(by.get(r.file) ?? []), r.line]);
  const gen = data.gen; // an answer that lands after a reload is dropped: keyed anew it would pass for fresh
  for (const [path, lines] of by) call("blame", { path, lines }).then((b) => { if (gen === data.gen) for (const [n, c] of Object.entries<any>(b.data.lines)) data.blame[blameKey(path, +n)] = c; }).catch(() => {});
};

export const guard = <T,>(p: Promise<T>) => p.catch((e) => { data.error = String(e.message ?? e); throw e; });

// (WEB-45) Share in-flight requests and discard them on a dashboard reload.
let requirementDetails = new Map<string, Promise<string>>();
export function requirementText(id: string): Promise<string> {
  const row = data.byId.get(id);
  if (row) return Promise.resolve(plain(row.text));
  const cache = requirementDetails;
  if (!cache.has(id)) cache.set(id, call("trace", { id }).then((r) => r.data?.spec?.text ? plain(r.data.spec.text) : "Not in any spec").catch(() => { cache.delete(id); return "Could not load requirement"; }));
  return cache.get(id)!;
}

let loadId = 0;
/** How many loads have started. */
export const loadCount = () => loadId;
export async function load() {
  data.error = "";
  const me = ++loadId;
  const [r, s] = await guard(Promise.all([call("report", { limit: 1000 }), call("status", { all: true, limit: 5000 })]));
  if (me !== loadId) return; // a newer load started meanwhile: its answer wins
  data.report = r.data;
  data.rows = s.data.rows;
  requirementDetails = new Map();
  data.up = true;
  // the modules views are optional: a failed resource read leaves them empty instead of failing the load
  const latest = <T,>(set: (v: T) => void) => (v: T) => { if (me === loadId) set(v); };
  read("spec17://modules").then(latest((m) => (data.modules = m))).catch(latest(() => (data.modules = null)));
  call("changes").then(latest((c) => (data.changes = c.data))).catch(latest(() => (data.changes = null)));
  call("commits").then(latest((c) => (data.commits = c.data.commits))).catch(latest(() => (data.commits = {})));
  data.gen++;
  data.loadedAt = Date.now();
  // asked on every load, as the branch may have changed; the reviewed marks are read once, for the root
  const first = !untrack(() => data.root);
  fetch("/api/info").then((x) => x.json()).then((i) => { data.root = i.root ?? ""; data.branch = i.branch ?? ""; }).catch(() => {}).finally(() => { if (first) loadReviewed(data.root).then(() => loadViews(data.root)); });
}

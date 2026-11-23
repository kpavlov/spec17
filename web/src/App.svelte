<script lang="ts">
  import { tick, untrack } from "svelte";
  import { call, embedded, onToolInput, read, watch } from "$lib/transport";
  import { format, parse, type Route } from "$lib/router";
  import Markdown from "$lib/ui/Markdown.svelte";
  import Graph from "$lib/views/Graph.svelte";
  import { cycles } from "$lib/requirements/cycles";
  import Split from "$lib/layout/Split.svelte";
  import SplitToggle from "$lib/layout/SplitToggle.svelte";
  import DataTable from "$lib/ui/DataTable.svelte";
  import { MediaQuery } from "svelte/reactivity";
  import { levelPills } from "$lib/requirements/levelPills";
  import RequirementBadges from "$lib/requirements/RequirementBadges.svelte";
  import { readValues, writeValues, toggleValue, matches, type Value } from "$lib/requirements/filters";
  import Filters from "$lib/ui/Filters.svelte";
  import Seg from "$lib/ui/Seg.svelte";
  import FileTree from "$lib/ui/FileTree.svelte";
  import Icon from "$lib/ui/Icon.svelte";
  import Pill from "$lib/ui/Pill.svelte";
  import IdLink from "$lib/ui/IdLink.svelte";
  import RequirementLink from "$lib/requirements/RequirementLink.svelte";
  import Section from "$lib/ui/Section.svelte";
  import PaneHead from "$lib/ui/PaneHead.svelte";
  import { requirementLevelTone, priorityTone, effortTone, stateTone, edgeTone, levelTone, changeTone, stateLabel, STATE_HINT, type Tone } from "$lib/tone";
  import { preview } from "$lib/layout/layout.svelte";
  import { reviewed, loadReviewed, setReviewed } from "$lib/reviewed.svelte";
  import DiffView from "$lib/views/DiffView.svelte";
  import ViewSwitch from "$lib/layout/ViewSwitch.svelte";
  import { sorted, toggleSort, type Sort } from "$lib/sort";
  import { inline } from "$lib/markdown";
  import { highlightLines } from "$lib/highlight";

  type Ref = { file: string; line: number; text: string };
  type Row = { id: string; state: string; file: string; line: number; level: string; priority: string | null; effort: string | null; text: string; references: string[]; mark: string; done: boolean; verdict: string; built: boolean; issues: string[]; tests: Ref[]; cites: Ref[]; graph: string[] };

  // ---- routing: the URL is the state ----
  const here = (): Route => {
    try { return parse(new URL(location.href)); } catch { return { tab: "dashboard" }; }
  };
  let route = $state<Route>(here());
  const u = format;

  function navigate(r: Route, replace = false) {
    if (r !== route) route = r;
    if (embedded) return; // a host iframe has no URL of its own
    try { (replace ? window.history.replaceState : window.history.pushState).call(window.history, null, "", format(r)); } catch { /* sandboxed */ }
  }

  // MCP Apps host: the `dashboard` tool's `view` ("requirements/DOC-25?state=none") picks the page
  onToolInput((a) => typeof a.view === "string" && navigate(parse(new URL(a.view.replace(/^\/?/, "/"), location.href))));

  $effect(() => {
    const onpop = () => (route = here());
    // same-origin links navigate in place; modified clicks keep the browser's own behaviour (new tab, download)
    const onclick = (e: MouseEvent) => {
      const a = (e.target as HTMLElement).closest<HTMLAnchorElement>("a[href^='/']");
      if (!a || e.defaultPrevented || e.button !== 0 || e.metaKey || e.ctrlKey || e.shiftKey || e.altKey || (a.target && a.target !== "_self")) return;
      e.preventDefault();
      navigate(route.view === "docs" && a.dataset.id ? { ...route, id: a.dataset.id } : parse(new URL(a.href)));
    };
    window.addEventListener("popstate", onpop);
    document.addEventListener("click", onclick);
    return () => { window.removeEventListener("popstate", onpop); document.removeEventListener("click", onclick); };
  });

  $effect(() => {
    document.title = ["spec17", route.tab, route.id ?? route.file ?? route.path].filter(Boolean).join(" · ");
  });

  // setup banner: hidden per browser until its message changes
  const stored = (k: string) => { try { return localStorage.getItem(k) ?? ""; } catch { return ""; } };
  let dismissed = $state(stored("spec17.banner"));
  const dismiss = (m: string) => { dismissed = m; try { localStorage.setItem("spec17.banner", m); } catch { /* storage blocked */ } };
  // sidebar: labels + icons, or an icon rail (remembered; narrow screens start as a rail, phones always are)
  const compactNav = new MediaQuery("(max-width: 75rem)");
  const mobileNav = new MediaQuery("(max-width: 48rem)");
  let rail = $state(stored("spec17.nav") ? stored("spec17.nav") === "rail" : compactNav.current);
  const toggleNav = () => { rail = !rail; try { localStorage.setItem("spec17.nav", rail ? "rail" : "full"); } catch { /* storage blocked */ } };
  const NAV: { name: string; tabs: string[] }[] = [
    { name: "Overview", tabs: ["dashboard", "changes"] },
    { name: "Browse", tabs: ["requirements", "modules", "tests"] },
    { name: "Gaps", tabs: ["overlap", "uncovered"] },
  ];
  const plural = (n: number, w: string) => `${n} ${w}${n === 1 ? "" : "s"}`;

  // ---- theme: auto (OS) | light | dark, remembered per browser ----
  type Theme = "auto" | "light" | "dark";
  // one button cycles system → light → dark; it shows the current state
  const themes: { v: Theme; icon: "system" | "sun" | "moon"; name: string }[] = [{ v: "auto", icon: "system", name: "system" }, { v: "light", icon: "sun", name: "light" }, { v: "dark", icon: "moon", name: "dark" }];
  const curTheme = $derived(themes.find((t) => t.v === theme)!);
  const nextTheme = $derived(themes[(themes.indexOf(curTheme) + 1) % themes.length]);
  const saved = (): Theme => { try { const t = localStorage.getItem("spec17.theme"); return t === "light" || t === "dark" ? t : "auto"; } catch { return "auto"; } };
  let theme = $state<Theme>(saved());
  $effect(() => {
    const el = document.documentElement;
    if (theme === "auto") delete el.dataset.theme; else el.dataset.theme = theme;
    try { theme === "auto" ? localStorage.removeItem("spec17.theme") : localStorage.setItem("spec17.theme", theme); } catch { /* storage blocked */ }
  });

  // keyboard: `/` focuses the filter (opening the requirements list first), filter shortcuts, Esc closes the trace panel (↑/↓ on the lists: DataTable)
  $effect(() => {
    const onkey = (e: KeyboardEvent) => {
      const t = e.target as HTMLElement;
      if (e.key === "Escape" && reqFiltersOpen && t.closest(".req-options")) { setReqFiltersOpen(false); document.querySelector<HTMLButtonElement>(".filter-toggle")?.focus(); return; }
      if (t.closest("input, select, textarea, .gutter, .seg") || e.metaKey || e.ctrlKey || e.altKey) { if (e.key === "Escape") (t as HTMLElement).blur(); return; }
      if (e.key === "/") {
        e.preventDefault();
        if (inDocs || (route.tab !== "requirements" && route.tab !== "changes")) navigate({ tab: "requirements" });
        queueMicrotask(() => document.querySelector<HTMLInputElement>("input[name=filter]")?.focus());
      } else if ((route.tab === "requirements" || route.tab === "changes") && e.key.toLowerCase() === "f") {
        e.preventDefault(); setReqFiltersOpen(true); tick().then(() => document.querySelector<HTMLInputElement>(".req-options button")?.focus());
      } else if (document.querySelector(".req-toolbar .filter-toggle") && /^[0-7slpecr]$/i.test(e.key)) {
        const k = e.key.toLowerCase();
        const field = facets.find((key) => facetShortcuts[key].toLowerCase() === k);
        if (k === "0") navigate({ ...route, ...clearReqFilters });
        else if (field) {
          const choices = options(field);
          if (choices.length === 1) return;
          const values = selected(field);
          const index = values.length ? choices.findIndex((choice) => choice.key === JSON.stringify(values[0])) : 0;
          const next = choices[(index + (e.shiftKey ? -1 : 1) + choices.length) % choices.length];
          navigate({ ...route, [field]: next.key ? writeValues([JSON.parse(next.key)]) : undefined });
        }
        else if (stateKeys.includes(states[+k - 1])) navigate({ ...route, state: states[+k - 1] === "all" ? undefined : states[+k - 1] });
      } else if (e.key === "Escape" && reqFiltersOpen && (route.tab === "requirements" || route.tab === "changes")) setReqFiltersOpen(false);
      else if (e.key === "Escape" && route.id && (route.tab === "requirements" || route.tab === "overlap")) navigate({ ...route, id: undefined });
      else if (e.key === "Escape" && route.path && route.tab === "tests") navigate({ ...route, path: undefined, line: undefined });
    };
    document.addEventListener("keydown", onkey);
    return () => document.removeEventListener("keydown", onkey);
  });

  // ---- data ----
  let setup = $state<{ status: any; suggestions: string[]; text: string; version?: string } | null>(null);
  let globalScope = $state(false);
  let checking = $state(false);
  let setupError = $state("");
  let busy = $state(false);
  let checks = $state<{ name: string; level: string; detail: string; hint: string | null }[]>([]);
  // the Settings button flags a failed check (red) or an unfinished install (amber)
  const setupBadge = $derived(checks.some((k) => k.level === "fail") ? "bad" : checks.some((k) => k.level !== "ok") || setup?.suggestions.length ? "warn" : "");
  let indexAction = $state<"init" | "sync" | null>(null);
  let indexLog = $state("");
  let specFiles = $state<string[] | null>(null);
  let specLoaded = $state("");
  let specMd = $state("");
  let treeOpen = $state(stored("spec17.tree") !== "0");
  const toggleTree = () => { treeOpen = !treeOpen; try { localStorage.setItem("spec17.tree", treeOpen ? "1" : "0"); } catch { /* storage blocked */ } };
  const inDocs = $derived(route.tab === "requirements" && route.view === "docs");
  let code = $state<{ path: string; text: string } | null>(null);
  let report = $state<any>(null);
  // the working tree against HEAD: undefined until loaded, null when it cannot be read (not a git work tree)
  let changes = $state<any>(undefined);
  // (WEB-35, WEB-36) {requirement ID: its newest commit (sha, short, author, date, subject)}; empty without git
  let commits = $state<Record<string, any>>({});
  let custom = $state<any>(undefined);
  // commits not on the base branch, for the Changes page
  let history = $state<any>(null);
  let openDiffs = $state<string[]>([]);
  const toggleDiff = (p: string) => (openDiffs = openDiffs.includes(p) ? openDiffs.filter((x) => x !== p) : [...openDiffs, p]);
  async function showDiff(path: string) {
    if (!openDiffs.includes(path)) openDiffs = [...openDiffs, path];
    await tick();
    const row = document.getElementById(`changed-file-${path}`);
    const section = row?.closest("details");
    if (section) section.open = true;
    row?.scrollIntoView({ block: "center" });
  }
  let modules = $state<any>(null);
  let tests = $state<any>(null);
  let rows = $state<Row[]>([]);
  const plain = (t: string) => t.replace(/[`*~]/g, "");
  const requirementCycles = $derived(cycles(rows));
  const requirementGraph = $derived(rows.map((r) => ({ path: r.id, text: plain(r.text), tests: 0, symbols: { requirement: 1 }, depends_on: r.references.map((id) => ({ module: id, edges: 1 })) })));
  let overlap = $state<any[] | null>(null);
  let uncovered = $state<any>(null);
  let trace = $state<{ text: string; data: any } | null>(null);
  let root = $state("");
  let error = $state("");

  const guard = <T,>(p: Promise<T>) => p.catch((e) => { error = String(e.message ?? e); throw e; });

  // server connection: null until known; the push stream sets it, and so does every successful load or poll (the only signal when embedded)
  let up = $state<boolean | null>(null);
  // bumped by every load: the views below refetch (keeping what they show until the new data lands)
  let gen = $state(0);
  let loadId = 0;
  let requirementDetails = new Map<string, Promise<string>>();
  async function load() {
    error = "";
    const me = ++loadId;
    const [r, s] = await guard(Promise.all([call("report", { limit: 1000 }), call("status", { all: true, limit: 5000 })]));
    if (me !== loadId) return; // a newer load started meanwhile: its answer wins
    report = r.data;
    rows = s.data.rows;
    requirementDetails = new Map();
    up = true;
    // the modules views are optional: a failed resource read leaves them empty instead of failing the load
    read("spec17://modules").then((m) => (modules = m)).catch(() => (modules = null));
    call("changes").then((c) => (changes = c.data)).catch(() => (changes = null));
    call("commits").then((c) => (commits = c.data.commits)).catch(() => (commits = {}));
    gen++;
    if (!root) fetch("/api/info").then((x) => x.json()).then((i) => (root = i.root ?? "")).catch(() => {}).finally(() => loadReviewed(root));
  }

  // (WEB-10, WEB-11) hide a module (after a confirm) or show it again via .spec17ignore, then rescan
  async function setHidden(path: string, hidden: boolean) {
    if (hidden && !confirm(`Add /${path}/ to .spec17ignore? spec17 will leave the module out of every scan.`)) return;
    try {
      await guard(call("module", { path, hidden }));
      await load();
    } catch { /* guard shows it */ }
  }

  // (WEB-27) Setup needs only these two calls, never the scan; a failure is shown there with a retry, not left at "Checking…"
  async function loadSetup() {
    checking = true;
    try {
      const [r, d] = await Promise.all([call("install_status"), call("doctor")]);
      setup = { ...r.data, text: r.text };
      checks = d.data.checks;
      indexAction = d.data.index_action;
      setupError = "";
    } catch (e: any) {
      setupError = String(e.message ?? e);
    } finally {
      checking = false;
    }
  }

  // (WEB-30) the rescan button: sync the index (`refresh` syncs even beside a running codegraph), then reload; a failed sync is shown, the reload still happens
  async function rescan() {
    busy = true;
    let sync = "";
    try { await call("codegraph", { action: "refresh" }); } catch (e: any) { sync = String(e.message ?? e); }
    await Promise.allSettled([load(), loadSetup()]);
    if (sync) error = sync;
    busy = false;
  }

  // the only thing here that indexes: runs when the user clicks, never on its own
  async function runIndex(action: "init" | "sync") {
    if (!confirm(action === "init" ? "Run `codegraph init` in this repo? It creates .codegraph/ and can take a while." : "Run `codegraph sync` to refresh the index?")) return;
    busy = true;
    indexLog = `codegraph ${action} running…`;
    try {
      indexLog = (await call("codegraph", { action })).text;
      await Promise.all([loadSetup(), load()]);
    } catch (e: any) {
      indexLog = String(e.message ?? e);
    } finally {
      busy = false;
    }
  }

  // `server: "codegraph"` (WEB-29) only adds or removes the CodeGraph entry of .mcp.json
  async function runSetup(tool: "install" | "uninstall", global = false, options: { server?: "codegraph"; only?: "skill" | "mcp"; transport?: "stdio" | "http" } = {}) {
    busy = true;
    try {
      await guard(call(tool, { global, ...options }));
      await loadSetup(); // the status and the checks name what is still missing
    } finally {
      busy = false;
    }
  }

  function launchCommand(launch?: { command: string; args: string[] } | null) {
    if (!launch) return undefined;
    return [launch.command, ...launch.args].map((arg) => /^[\w./:-]+$/.test(arg) ? arg : "'" + arg.replaceAll("'", "'\"'\"'") + "'").join(" ");
  }

  // each view loads what its URL names
  $effect(() => {
    const id = route.id;
    void rows; // a refresh replaces the rows and must reload the open trace
    if (!id || inDocs) { trace = null; return; }
    // the previous trace stays (dimmed) until the new one lands, so the preview pane never collapses and reopens
    call("trace", { id }).then((r) => { if (!inDocs && route.id === id) { trace = r; document.querySelector(".split .preview")?.scrollTo(0, 0); } }).catch((e) => { trace = null; error = e.message; });
  });

  // each selection takes a token (also when the file is already loaded), so a slower earlier response cannot replace it
  let docReq = 0;
  $effect(() => {
    if (!inDocs) return;
    if (specFiles === null) { call("file").then((r) => (specFiles = r.data.files)).catch((e) => (error = e.message)); return; }
    const f = docFile, n = ++docReq;
    if (f && f !== untrack(() => specLoaded)) call("file", { name: f }).then((r) => { if (n !== docReq) return; specLoaded = f; specMd = r.data.markdown; document.querySelector(".doc")?.scrollTo(0, 0); }).catch((e) => { if (n === docReq) error = e.message; });
  });

  $effect(() => {
    const path = route.tab === "code" || route.tab === "tests" ? route.path : undefined;
    if (path && untrack(() => code?.path) !== path) call("source", { path }).then((r) => (code = r.data)).catch((e) => { code = null; error = e.message; });
  });

  // fetched once per load generation, when its tab is open
  const fetched: Record<string, number> = {};
  const fetchOnce = (key: string, get: () => Promise<void>) => {
    if (fetched[key] === gen) return;
    fetched[key] = gen;
    get().catch((e) => { fetched[key] = -1; error = e.message; });
  };
  $effect(() => {
    if (route.tab === "overlap") fetchOnce("overlap", () => call("overlap").then((r) => void (overlap = r.data.rows)));
    if (route.tab === "tests" || route.tab === "requirements" || route.tab === "changes" || (route.tab === "modules" && route.module)) fetchOnce("tests", () => read("spec17://tests").then((r) => void (tests = r)));
    if (route.tab === "uncovered") fetchOnce("uncovered", () => call("uncovered").then((r) => void (uncovered = r.data)));
  });

  // the Changes page asks for its own scope when it is not the default (staged + unstaged + untracked)
  const scope = $derived({
    ref: route.ref ?? "", commit: route.commit ?? "", staged: route.staged === "1", include_untracked: route.untracked !== "0",
    ignore_whitespace: route.ws === "1", show_ignored: route.ignored === "1", show_all: route.all === "1",
  });
  const defaultScope = $derived(!scope.ref && !scope.commit && !scope.staged && scope.include_untracked && !scope.ignore_whitespace && !scope.show_ignored && !scope.show_all);
  const view = $derived(defaultScope ? changes : custom);
  $effect(() => {
    if (route.tab !== "changes") return;
    void gen;
    call("history", { ignore_whitespace: scope.ignore_whitespace, show_ignored: scope.show_ignored, show_all: scope.show_all }).then((r) => (history = r.data)).catch(() => (history = null));
    if (defaultScope) return;
    call("changes", scope).then((r) => (custom = r.data)).catch(() => (custom = null));
  });

  // scroll to what the URL points at once it is rendered (the lists scroll their own selection: DataTable)
  $effect(() => {
    void [specMd, code, route.id, route.line];
    queueMicrotask(() => {
      const el = route.tab === "code" ? document.getElementById(`L${route.line}`) : inDocs && route.id ? document.querySelector(`article a[data-id="${route.id}"]`) : null;
      el?.scrollIntoView({ block: "center" });
    });
  });

  // (WEB-27) the first run lands on Setup: nothing to scan and nothing installed, or a failed check; never inside a host that picks the view
  Promise.allSettled([load(), loadSetup()]).then(() => {
    const first = report && report.requirements + report.refs === 0 && setup?.suggestions.length;
    if (!embedded && route.tab === "dashboard" && (first || checks.some((k) => k.level === "fail"))) navigate({ tab: "settings" }, true);
  });
  // the server pushes `resources/updated` when the repo changes; one notification per subscribed resource arrives for one change
  let reloadTimer: ReturnType<typeof setTimeout> | undefined;
  // throttled: the first push reloads at once, a burst after it collapses into one trailing reload at most every 2 s
  let lastLoad = 0;
  $effect(() => watch(["spec17://report", "spec17://status", "spec17://changes"], () => {
    clearTimeout(reloadTimer);
    reloadTimer = setTimeout(() => { lastLoad = Date.now(); load().catch(() => {}); }, Math.max(0, lastLoad + 2000 - Date.now()));
  }, (s) => (up = s)));
  // fallback when no push arrives (embedded host, a dropped stream, a proxy that buffers): every 30 s, reload only if the report changed
  $effect(() => {
    const t = setInterval(async () => {
      if (document.hidden) return;
      try { if (JSON.stringify((await call("report", { limit: 1000 })).data) !== JSON.stringify(untrack(() => report))) await load(); up = true; } catch { up = false; /* the next tick retries */ }
    }, 30_000);
    return () => clearInterval(t);
  });

  // ---- views ----
  // the states that need work come first; the hints say what the scanner saw, not that anything passes
  const noReqs = $derived(report && report.requirements + report.refs === 0);
  const sum = (key: string) => (modules?.modules ?? []).reduce((n: number, m: any) => n + m[key], 0);
  const states = ["all", "none", "src-only", "tested", "manual", "later", "ref"];
  // (WEB-22) what the working tree changed: files by path, the kinds a requirement is touched by, the modules holding changed files
  const fileOf = $derived(new Map<string, any>((changes?.files ?? []).map((f: any) => [f.path, f])));
  // (WEB-37) directly changed: its bullet, a tagged test or a cited unit differs from the base; one only reached through a test is indirect
  const isDirect = (r: any) => r.notes.some((n: any) => n.action !== "reached by a test");
  const directReqs = $derived<any[]>((changes?.requirements ?? []).filter(isDirect));
  const changeOf = $derived(new Map<string, any>(directReqs.map((r: any) => [r.id, r])));
  const reqKinds = $derived(new Map<string, string[]>(directReqs.map((r: Row & { changed: string[] }) => [r.id, r.changed])));
  const moduleOfFile = (path: string) => (modules?.modules ?? []).map((m: any) => m.path).filter((m: string) => m !== "." && path.startsWith(`${m}/`)).sort((a: string, b: string) => b.length - a.length)[0] ?? ".";
  const changedMods = $derived(new Set<string>((changes?.files ?? []).map((f: any) => moduleOfFile(f.path))));
  const onlyChanged = $derived(route.changed === "1");
  // (WEB-35) reviewed = its commit is the one it was reviewed at; committed again since, it is unreviewed again
  const isReviewed = (id: string) => !!commits[id] && reviewed[id] === commits[id].sha;
  const reviewTip = (id: string) => (isReviewed(id) ? `reviewed at ${commits[id].short}` : reviewed[id] ? `changed since reviewed (${reviewed[id].slice(0, 7)} → ${commits[id].short})` : "mark reviewed");
  // (WEB-36) who last changed the line of a test: asked per file when a preview shows it; keyed by reload (gen), so a reload asks again
  let blame = $state<Record<string, any>>({});
  const blameKey = (file: string, line: number) => `${gen}:${file}:${line}`;
  const wantBlame = (refs: { file: string; line: number }[]) => {
    const by = new Map<string, number[]>();
    for (const r of refs) if (!(blameKey(r.file, r.line) in blame)) by.set(r.file, [...(by.get(r.file) ?? []), r.line]);
    for (const [path, lines] of by) call("blame", { path, lines }).then((b) => { for (const [n, c] of Object.entries<any>(b.data.lines)) blame[blameKey(path, +n)] = c; }).catch(() => {});
  };
  $effect(() => { void gen; const t = trace?.data?.tests; if (t?.length) untrack(() => wantBlame(t)); });
  $effect(() => { void gen; if (route.tab === "tests" && route.path && route.line) untrack(() => wantBlame([{ file: route.path!, line: route.line! }])); });
  const changeMark = (path: string) => fileOf.get(path)?.change ?? "";
  // (WEB-25) the preview of a changed item shows the diff when the remembered view is "diff"
  const diffOn = (changed: boolean) => changed && preview.view === "diff";
  // the changed files behind a requirement: its spec file when the bullet was edited, and its tests and citing sources that changed
  const reqFiles = (t: any, kinds: string[] = []) => [...new Set<string>([...(kinds.includes("spec") && t.spec ? [t.spec.file] : []), ...(t.tests ?? []).map((x: any) => x.file), ...(t.source_cites ?? []).map((x: any) => x.file)].filter((f) => fileOf.has(f)))];
  const stateFilter = $derived(route.state ?? "all");
  const query = $derived(route.q ?? "");
  const levels = ["MUST", "SHOULD", "MAY"];
  const onChanges = $derived(route.tab === "changes");
  // (WEB-57) change records include their own metadata, even when the status list is truncated
  const reqFilterRows = $derived<Row[]>(onChanges ? view?.requirements ?? [] : rows);
  const clearReqFilters = { state: undefined, level: undefined, priority: undefined, effort: undefined, q: undefined, changed: undefined, reviewed: undefined };
  let reqFiltersOpen = $state(stored("spec17.requirementFilters") !== "0"); // (WEB-58)
  // (WEB-59) each facet ignores its own selection when counting choices.
  type Facet = "state" | "level" | "priority" | "effort" | "changed" | "reviewed";
  const facets: Facet[] = ["state", "level", "priority", "effort", "changed", "reviewed"];
  const facetLabels: Record<Facet, string> = { state: "State", level: "Level", priority: "Priority", effort: "Effort", changed: "Changes", reviewed: "Reviewed" };
  const facetShortcuts: Record<Facet, string> = { state: "S", level: "L", priority: "P", effort: "E", changed: "C", reviewed: "R" };
  const setReqFiltersOpen = (open: boolean) => { reqFiltersOpen = open; save("spec17.requirementFilters", open ? "1" : "0"); };
  const selected = (key: Facet) => key === "state" && route.state === "all" ? [] : readValues(route[key]);
  const levelOf = (r: Row) => levels.includes(r.level) ? r.level : "other";
  const facetValue = (r: Row, key: Facet): Value => key === "level" ? levelOf(r) : key === "changed" ? (onChanges ? isDirect(r) : reqKinds.has(r.id)) ? "1" : "0" : key === "reviewed" ? isReviewed(r.id) ? "1" : "0" : r[key] ?? null;
  const facet = (skip?: Facet) => reqFilterRows.filter((r) => facets.every((key) => key === skip || matches(selected(key), facetValue(r, key))) && (!query || `${r.id} ${r.text} ${r.file}`.toLowerCase().includes(query.toLowerCase())));
  const shown = $derived(facet());
  const stateKeys = $derived(states.filter((st) => st === "all" || selected("state").includes(st) || reqFilterRows.some((r) => r.state === st)));
  const activeReqFilters = $derived(facets.filter((key) => selected(key).length).length);
  const filtering = $derived(!!activeReqFilters || !!query);
  const changedSeg = (n: number) => [{ key: "all", label: "all" }, { key: "changed", label: "changed", count: n }];
  const showChanged = $derived(!!changes?.files.length || onlyChanged);
  const pickChanged = (k: string) => filter({ changed: k === "changed" ? "1" : undefined });
  const optionLabel = (key: Facet, v: Value) => v === null ? "Not set" : key === "state" ? stateLabel(v) : key === "level" ? v.toLowerCase() : key === "changed" ? onChanges ? v === "1" ? "directly changed" : "reached through tests" : v === "1" ? "changed" : "unchanged" : key === "reviewed" ? v === "1" ? "reviewed" : "unreviewed" : v;
  const optionTone = (key: Facet, value: Value) => value === null ? undefined : key === "state" ? stateTone[value] : key === "level" ? requirementLevelTone[value.toUpperCase()] : key === "priority" ? priorityTone(value) : key === "effort" ? effortTone(value) : undefined;
  const options = (key: Facet) => {
    const present = new Set(reqFilterRows.map((row) => facetValue(row, key)));
    const order = key === "state" ? states.slice(1) : key === "level" ? [...levels, "other"] : ["1", "0"];
    const values = [...new Set([...present, ...selected(key)])];
    values.sort((a, b) => {
      if (a === null) return 1;
      if (b === null) return -1;
      if (key !== "effort" && key !== "priority") {
        const rank = (value: string) => order.includes(value) ? order.indexOf(value) : order.length;
        return rank(a) - rank(b);
      }
      return a.localeCompare(b, undefined, { numeric: true });
    });
    const eligible = facet(key);
    const counts = new Map<Value, number>();
    for (const row of eligible) {
      const value = facetValue(row, key);
      counts.set(value, (counts.get(value) ?? 0) + 1);
    }
    return [{ key: "", label: "All", count: eligible.length }, ...values.map((value) => ({ key: JSON.stringify(value), label: optionLabel(key, value), count: counts.get(value) ?? 0, tone: optionTone(key, value), title: key === "state" && value ? STATE_HINT[value] : undefined }))];
  };
  const pickFacet = (key: Facet, value: Value) => navigate({ ...route, [key]: writeValues(toggleValue(selected(key), value)) });
  const clearFacet = (key: Facet) => navigate({ ...route, [key]: undefined });
  const editFacet = (key: Facet) => { setReqFiltersOpen(true); tick().then(() => document.querySelector<HTMLButtonElement>(`[data-facet="${key}"] button[aria-pressed=true]`)?.focus()); };
  // the modules table sorts here; the requirement, test and spec lists sort inside DataTable
  let modSort = $state<Sort>({ key: "", dir: 1 });
  const reqVal = (r: Row, k: string) => (k === "file" ? `${r.file}:${String(r.line).padStart(6, "0")}` : (r as any)[k]);
  const modVal = (m: any, k: string) => (k === "languages" ? m.languages.join(", ") : k === "symbols" ? symbolsOf(m) : k === "depends" ? m.depends_on.length : (m[k] ?? ""));
  const prefixOf = (id: string) => id.replace(/-.*$/, "");
  // requirement table columns (the state colours the row edge)
  const reqCols = [{ key: "id", label: "ID", cls: "mono nowrap" }, { key: "text", label: "requirement" }, { key: "file", label: "location", cls: "mono muted wrap" }];
  const invalidN = $derived(report ? Object.values(report.invalid_tests).reduce((n: number, v: any) => n + v.length, 0) : 0);
  const known = $derived(new Set(rows.map((r) => r.id)));
  const mods = $derived<any[]>(sorted((modules?.modules ?? []).filter((m: any) => (!onlyChanged || changedMods.has(m.path)) && (!query || m.path.toLowerCase().includes(query.toLowerCase()))), modSort, modVal));
  const hiddenMods = $derived<string[]>((modules?.hidden ?? []).filter((m: string) => !query || m.toLowerCase().includes(query.toLowerCase())));
  const modOf = $derived(modules?.modules.find((m: any) => m.path === route.module));
  const usedBy = $derived((modules?.modules ?? []).filter((m: any) => modOf && m.depends_on.some((d: any) => d.module === modOf.path)));
  const testsOf = (path: string) => tests?.modules.find((m: any) => m.module === path);
  const symbolsOf = (m: any) => Object.values<number>(m.symbols).reduce((a, b) => a + b, 0);
  // The tests screen filters the detected cases; the requirement table uses the same cases for its links (WEB-39).
  const testOpts = $derived(["all", "tagged", "untagged"].map((k) => ({ key: k, label: k, count: tcounts[k] })));
  const VIEWS = [{ key: "list", label: "list" }, { key: "graph", label: "graph" }];
  const detectedCases = $derived<any[]>((tests?.modules ?? []).flatMap((m: any) => m.files.flatMap((f: any) => f.tests.map((t: any) => ({ module: m.module, file: f.file, t })))));
  const testedIds = $derived(new Set<string>(detectedCases.flatMap((c) => c.t.ids)));
  const testRequirement = $derived(route.requirementId?.toUpperCase());
  const allCases = $derived(detectedCases.filter((c) => (!route.module || c.module === route.module) && (!testRequirement || c.t.ids.includes(testRequirement))));
  const tcounts = $derived({ all: allCases.length, tagged: allCases.filter((c) => c.t.ids.length).length, untagged: allCases.filter((c) => !c.t.ids.length).length } as Record<string, number>);
  const matched = $derived(allCases.filter((c) => (stateFilter === "all" || (stateFilter === "tagged") === c.t.ids.length > 0) && (!onlyChanged || fileOf.has(c.file)) && (!query || `${c.file} ${c.t.name} ${c.t.ids.join(" ")}`.toLowerCase().includes(query.toLowerCase()))));
  // list rows: a test is told apart by file and line, which is also how the URL selects it (`path` + `line`)
  const testRows = $derived(matched.map((c) => ({ ...c, key: `${c.file}:${c.t.line}` })));
  const testCols = [{ key: "name", label: "test", cls: "wrap" }, { key: "ids", label: "requirements", cls: "test-ids" }, { key: "file", label: "location", cls: "mono muted wrap" }];
  const testVal = (c: any, k: string) => (k === "file" ? `${c.file}:${String(c.t.line).padStart(6, "0")}` : k === "ids" ? c.t.ids.join(" ") : c.t.name);
  const selTest = $derived(route.tab === "tests" && route.path ? `${route.path}:${route.line}` : undefined);
  const byId = $derived(new Map(rows.map((r) => [r.id, r])));
  // the open document: the one named, else the one that holds the selected requirement, else the first
  const docFile = $derived(route.file ?? (route.id ? byId.get(route.id)?.file : undefined) ?? specFiles?.[0]);
  // the preview shows the lines of the selected test: a few before it and the start of its body
  const TEST_BEFORE = 3, TEST_AFTER = 24;
  const docFiles = $derived((specFiles ?? []).filter((file) => !onlyChanged || fileOf.has(file) || file === docFile));
  // (WEB-33) the short hash of the explained commit, for the section titles
  const at = $derived(scope.commit ? ` (${history?.commits.find((c: any) => c.sha === scope.commit || c.short === scope.commit)?.short ?? scope.commit.slice(0, 7)})` : "");
  const CHANGES = ["added", "modified", "renamed", "deleted"];
  const nOf = (kind: string) => (view?.files ?? []).filter((f: any) => f.kind === kind).length;
  const viewMods = $derived(new Set<string>((view?.files ?? []).map((f: any) => moduleOfFile(f.path))));
  const noTestN = $derived(directReqs.filter((r: Row) => r.state === "none" || r.state === "src-only").length);
  // one pill per kind of note, the files it happened in as its title
  const noteGroups = (notes: { role: string; action: string; file: string }[]) => {
    const g = new Map<string, Set<string>>();
    for (const n of notes) g.set(`${n.role} ${n.action}`, (g.get(`${n.role} ${n.action}`) ?? new Set()).add(n.file));
    return [...g].map(([k, v]) => [k, [...v]] as const);
  };
  const tallyBy = (files: any[]) => CHANGES.map((c) => [c, files.filter((f) => f.change === c).length] as const).filter(([, n]) => n);
  const taggedN = $derived(tests ? tests.modules.reduce((n: number, m: any) => n + m.files.reduce((k: number, f: any) => k + f.tests.filter((t: any) => t.ids.length).length, 0), 0) : 0);
  // (WEB-45) Share in-flight requests and discard them on a dashboard reload.
  function requirementText(id: string): Promise<string> {
    const row = byId.get(id);
    if (row) return Promise.resolve(plain(row.text));
    const cache = requirementDetails;
    if (!cache.has(id)) cache.set(id, call("trace", { id }).then((r) => r.data?.spec?.text ? plain(r.data.spec.text) : "Not in any spec").catch(() => { cache.delete(id); return "Could not load requirement"; }));
    return cache.get(id)!;
  }
  const req = (id: string): Route => ({ tab: "requirements", id });
  // a spec document (the Documents view of Requirements), optionally at one requirement
  const doc = (file?: string, id?: string, changed?: string): Route => ({ tab: "requirements", view: "docs", file, id, changed });
  const docs = "https://spec17.dev";
  const withId = (tab: Route["tab"], id: string): Route => ({ tab, id });
  // typing in the filter replaces the URL instead of stacking a history entry per keystroke
  const filter = (patch: Partial<Route>) => navigate(Object.assign(route, patch), true);
  // highlighted line HTML with known requirement IDs turned into links (only text between tags is touched)
  const linkIds = (html: string) => html.replace(/(<[^>]+>)|\b[A-Z]{2,5}-\d{1,4}\b/g, (m, tag) => (tag || !known.has(m) ? m : `<a class="id" href="${u(req(m))}">${m}</a>`));
  const lines = $derived(code ? highlightLines(code.path, code.text).map(linkIds) : []);

  // code viewer: wrap, copy the path, open in an editor (URL scheme of the chosen IDE, remembered per browser)
  // `scheme://file/<abs path>:<line>` (VS Code family, Zed), JetBrains `open?file=&line=`, or the editor's own form
  const fileUrl = (scheme: string) => (f: string, l: number) => `${scheme}://file${f}:${l}`;
  const jetbrains = (scheme: string) => (f: string, l: number) => `${scheme}://open?file=${encodeURIComponent(f)}&line=${l}`;
  const EDITORS: Record<string, (file: string, line: number) => string> = {
    IntelliJ: jetbrains("idea"),
    WebStorm: jetbrains("webstorm"),
    PyCharm: jetbrains("pycharm"),
    GoLand: jetbrains("goland"),
    RustRover: jetbrains("rustrover"),
    CLion: jetbrains("clion"),
    PhpStorm: jetbrains("phpstorm"),
    Rider: jetbrains("rider"),
    "Android Studio": jetbrains("androidstudio"),
    "VS Code": fileUrl("vscode"),
    "VS Code Insiders": fileUrl("vscode-insiders"),
    VSCodium: fileUrl("vscodium"),
    Cursor: fileUrl("cursor"),
    Windsurf: fileUrl("windsurf"),
    Zed: fileUrl("zed"),
    Sublime: (f, l) => `subl://open?url=${encodeURIComponent("file://" + f)}&line=${l}`,
    TextMate: (f, l) => `txmt://open?url=${encodeURIComponent("file://" + f)}&line=${l}`,
    Xcode: (f, l) => `xcode://open?file=${encodeURIComponent(f)}&line=${l}`,
  };
  const savedEditor = stored("spec17.editor");
  let editor = $state(savedEditor in EDITORS ? savedEditor : "IntelliJ");
  let wrap = $state(stored("spec17.wrap") === "1");
  let copied = $state(false);
  let showHidden = $state(stored("spec17.hidden") === "1");
  let modView = $state(stored("spec17.modules") === "graph" ? "graph" : "list");
  let reqView = $state(["graph", "docs"].includes(stored("spec17.requirements")) ? stored("spec17.requirements") : "list");
  const save = (k: string, v: string) => { try { localStorage.setItem(k, v); } catch { /* storage blocked */ } };
  const copyPath = (path: string) => navigator.clipboard.writeText(path).then(() => { copied = true; setTimeout(() => (copied = false), 1200); }).catch(() => {});
</script>

{#snippet viewSeg()}
  <!-- (WEB-49) -->
  <Seg label="View" options={[...VIEWS, { key: "docs", label: "documents" }]} value={route.view ?? "list"} onpick={(k) => { save("spec17.requirements", (reqView = k)); navigate({ ...route, view: k === "list" ? undefined : k, file: undefined, id: undefined }); }} /> <!-- (WEB-51) -->
{/snippet}

{#snippet requirementToolbar()}
  <!-- (WEB-33, WEB-43, WEB-59) -->
  <div class="bar req-toolbar">
    <input name="filter" aria-label={onChanges ? "Search affected requirements" : "Search requirements"} placeholder="Search requirements…" title="Search by ID, text or file (/)" value={query} oninput={(e) => filter({ q: e.currentTarget.value })} />
    <button type="button" class="filter-toggle" aria-expanded={reqFiltersOpen} aria-controls="requirement-filter-options" title="Filter requirements (F)" onclick={() => setReqFiltersOpen(!reqFiltersOpen)}>Filters{activeReqFilters ? ` (${activeReqFilters})` : ""}</button>
    <span class="found muted nowrap" aria-live="polite">{shown.length} of {reqFilterRows.length}</span>
    {#if filtering}<a class="clear-filters" href={u({ ...route, ...clearReqFilters })} aria-label="Clear requirement filters" title="Clear requirement filters (0)">Clear</a>{/if}
    {#if route.tab === "requirements"}{@render viewSeg()}{/if}
    {#if !onChanges && route.id}<SplitToggle />{/if}
  </div>
  <div class="req-options" id="requirement-filter-options" hidden={!reqFiltersOpen}>
    {#if reqFiltersOpen}
      {#each facets as key}
        {@const values = selected(key)}
        {@const choices = options(key)}
        {#if choices.length > 1}
        <div data-facet={key}>
          <Seg label={facetLabels[key]} keycap={facetShortcuts[key]} hint={`${facetShortcuts[key]} for next choice, Shift+${facetShortcuts[key]} for previous`} options={choices} selected={values.length ? values.map((value) => JSON.stringify(value)) : [""]} onpick={(value) => value ? pickFacet(key, JSON.parse(value)) : values.length && clearFacet(key)} />
        </div>
        {/if}
      {/each}
    {/if}
  </div>
  {#if !reqFiltersOpen && activeReqFilters}
    <div class="filter-chips" role="group" aria-label="Active requirement filters">
      {#each facets as key}
        {#each new Set(selected(key)) as value (value)}
          {@const label = value === null ? `No ${facetLabels[key].toLowerCase()}` : optionLabel(key, value)}
          <span class="filter-chip" data-tone={optionTone(key, value)}>
            <button type="button" title="Edit {facetLabels[key]} filter" aria-label="Edit {facetLabels[key]} filter, {label}" onclick={() => editFacet(key)}>{label}</button>
            <button type="button" title="Remove {label}" aria-label="Remove {label} from {facetLabels[key]} filter" onclick={(e) => {
              const chip = e.currentTarget.parentElement;
              const next = (chip?.nextElementSibling ?? chip?.previousElementSibling)?.querySelector<HTMLButtonElement>("button:last-child");
              pickFacet(key, value);
              (next ?? document.querySelector<HTMLButtonElement>(".req-toolbar .filter-toggle"))?.focus();
            }}>×</button>
          </span>
        {/each}
      {/each}
    </div>
  {/if}
{/snippet}

{#snippet reqCell(r: any, k: string)}
  {#if k === "id"}{#if commits[r.id]}<input type="checkbox" class="rv" aria-label="reviewed" title={reviewTip(r.id)} checked={isReviewed(r.id)} onchange={(e) => setReviewed(r.id, e.currentTarget.checked ? commits[r.id].sha : undefined)} />{:else if Object.keys(commits).length}<span class="rv muted" title="not committed yet: commit it to review it">·</span>{/if}{#if r.deleted || !known.has(r.id)}{r.id}{:else}<a href={u(route.tab === "requirements" ? { ...route, id: r.id } : req(r.id))}>{r.id}</a>{/if}
    <div class="badges"><RequirementBadges level={r.level} state={r.state} priority={r.priority} effort={r.effort} deleted={r.deleted} /></div>
    {#if r.done}<Pill tone={stateTone.done} title="check-marked: {r.verdict}">✓ {r.verdict}</Pill>{/if}
  {:else if k === "text"}<div class="txt" title={plain(r.text)} {@attach levelPills(r.text)}>{@html inline(r.text)}</div>
    <div class="notes">
      {#if isReviewed(r.id)}<Pill tone="muted">Reviewed</Pill>{/if}
      {#if !r.deleted && testedIds.has(r.id)}
        <a href={u({ tab: "tests", requirementId: r.id })} aria-label="View tagged tests for {r.id}">View tagged tests</a>
      {/if}
      <!-- (WEB-04) -->
      {#each noteGroups(r.notes ?? changeOf.get(r.id)?.notes ?? []) as [what, files]}
        {#each files as file}
          {#if onChanges && (scope.commit || view?.files?.find((f: any) => f.path === file)?.change === "deleted")}
            <button type="button" class="note-link" aria-label="Show diff: {what}: {file}" onclick={() => showDiff(file)}><Pill tone={r.deleted ? "bad" : "accent"} title={file}>✎ {what}{files.length > 1 ? ` · ${file}` : ""}</Pill></button>
          {:else if changes?.files?.find((f: any) => f.path === file)?.change === "deleted"}
            <Pill tone={r.deleted ? "bad" : "accent"} title={file}>✎ {what}{files.length > 1 ? ` · ${file}` : ""}</Pill>
          {:else}
            <a href={u(what.startsWith("spec ") ? doc(file, r.id) : { tab: "code", path: file })} aria-label="{what}: {file}"><Pill tone={r.deleted ? "bad" : "accent"} title={file}>✎ {what}{files.length > 1 ? ` · ${file}` : ""}</Pill></a>
          {/if}
        {/each}
      {/each}
    </div>
  {:else if r.line}<a href={u(doc(r.file, r.id))}>{r.file}:{r.line}</a>{:else}{r.file}{/if}
{/snippet}

{#snippet who(c: any, note = false)}
  {#if c && note}<p class="muted note">Last changed by <b>{c.author}</b>, {c.date} · <span class="mono">{c.short}</span> {c.subject}</p>
  {:else if c}<span class="muted who" title={c.subject}>{c.author}, {c.date} · <span class="mono">{c.short}</span></span>{/if}
{/snippet}

{#snippet requirementLink(id: string, to: Route)}
  <RequirementLink {id} href={u(to)} getText={requirementText} />
{/snippet}

{#snippet reqRef(id: string, to: Route)}
  {@render requirementLink(id, to)}{#if byId.get(id)}{" "}<RequirementBadges level={byId.get(id)!.level} state={byId.get(id)!.state} priority={byId.get(id)!.priority} effort={byId.get(id)!.effort} />{/if}
{/snippet}

{#snippet th(label: string, key: string, st: Sort, cls = "")}
  <th class={cls} aria-sort={st.key === key ? (st.dir === 1 ? "ascending" : "descending") : "none"}>
    <button type="button" class="sort" onclick={() => toggleSort(st, key)}>{label}<span class="arrow">{st.key === key ? (st.dir === 1 ? "▲" : "▼") : ""}</span></button>
  </th>
{/snippet}

{#snippet cases(files: any[])}
  {#each files as f}
    <h3 class="file mono nowrap"><a href={u({ tab: "code", path: f.file })}>{f.file}</a> <span class="muted">{f.tests.length}</span></h3>
    <ul class="refs">
      {#each f.tests as t}
        <li><a class="mono" href={u({ tab: "code", path: f.file, line: t.line })} title={t.qualified}>{t.name}</a> {#each t.ids as id}{@render reqRef(id, known.has(id) ? req(id) : { tab: "requirements", q: id })}{" "}{/each}{#if !t.ids.length}<span class="muted">untagged</span>{/if}</li>
      {/each}
    </ul>
  {/each}
{/snippet}

{#snippet tracePanel(float = false)}
  {#if trace || !float}
    <aside class:float class:stale={trace && route.id && trace.data?.id?.toUpperCase() !== route.id.toUpperCase()}>
      {#if !trace || !route.id}
        <p class="muted empty">Loading requirement…</p>
      {:else}
      {@const sp = trace.data?.spec}
      <PaneHead title={trace.data.id} close={u({ ...route, id: undefined })}><RequirementBadges level={sp?.level} state={byId.get(trace.data.id)?.state} priority={sp?.priority} effort={sp?.effort} /></PaneHead>
      {@const dfiles = reqFiles(trace.data, reqKinds.get(trace.data.id))}
      {#if dfiles.length}<p><ViewSwitch n={dfiles.length} /></p>{/if}
      {#if diffOn(dfiles.length > 0)}
        <DiffView paths={dfiles} {gen} href={(p) => u({ tab: "code", path: p })} />
        <p class="muted note">What changed against HEAD in the files that spec, tag or cite this requirement. Evidence of edits, not that it is done.</p>
      {:else}
      {#if sp}
        <p class="muted mono nowrap"><a href={u(doc(sp.file, trace.data.id))}>{sp.file}:{sp.line}</a></p>
        {@render who(commits[trace.data.id], true)}
        <Markdown source={sp.text} />
      {:else}<p class="muted">Not in any spec: cited only in tests or code.</p>{/if}
      {#if trace.data.tests?.length}
        <h3 class="preview-section">Tests ({trace.data.tests.length})</h3>
        <ul class="refs">{#each trace.data.tests as t}<li><a class="mono nowrap" href={u({ tab: "code", path: t.file, line: t.line })}>{t.file}:{t.line}</a> {t.text} {@render who(blame[blameKey(t.file, t.line)])}</li>{/each}</ul>
      {/if}
      {#if trace.data.source_cites?.length}
        <h3 class="preview-section">Code citations ({trace.data.source_cites.length})</h3>
        <ul class="refs">{#each trace.data.source_cites as c}<li><a class="mono nowrap" href={u({ tab: "code", path: c.file, line: c.line })}>{c.file}:{c.line}</a> <code>{c.decl}</code></li>{/each}</ul>
      {/if}
      {#if trace.data.related?.length}
        <h3 class="preview-section">Related code ({trace.data.related_total})</h3>
        <ul class="refs">{#each trace.data.related as r}<li><a class="mono nowrap" href={u({ tab: "code", path: r.file })}>{r.file}</a> <span class="muted">{r.types.join(", ")}{r.cites ? " · cites it" : ""}</span></li>{/each}</ul>
      {/if}
      <details class="plain"><summary class="muted">plain text</summary><pre>{trace.text}</pre></details>
      <p class="muted note">Evidence, not truth.</p>
      {/if}
      {/if}
    </aside>
  {/if}
{/snippet}

{#snippet testPanel()}
  {#if !route.path}
    <p class="muted empty">Select a test to preview it.</p>
  {:else}
    {@const c = allCases.find((x) => x.file === route.path && x.t.line === route.line)}
    {@const lo = Math.max(1, (route.line ?? 1) - TEST_BEFORE)}
    <PaneHead title={c?.t.name ?? route.path} close={u({ ...route, path: undefined, line: undefined })} />
    <p class="muted mono nowrap"><a href={u({ tab: "code", path: route.path, line: route.line })}>{route.path}:{route.line}</a>{#if c}{" "}in <a href={u({ tab: "modules", module: c.module })}>{c.module}</a>{/if}</p>
    {@render who(route.path && route.line ? blame[blameKey(route.path, route.line)] : undefined, true)}
    {#if fileOf.has(route.path)}<p><ViewSwitch /></p>{/if}
    {#if diffOn(fileOf.has(route.path))}
      <DiffView paths={[route.path]} {gen} />
    {:else}
    {#if c}
      <h3 class="preview-section">Requirements</h3>
      {#if c.t.ids.length}
        <ul class="refs">{#each c.t.ids as id}<li>{@render reqRef(id, known.has(id) ? req(id) : { tab: "requirements", q: id })} <span class="muted">{byId.get(id) ? plain(byId.get(id)!.text).slice(0, 140) : "not in any spec"}</span></li>{/each}</ul>
      {:else}<p class="muted">Untagged: no requirement ID in its tag.</p>{/if}
    {/if}
    <h3 class="preview-section">Source</h3>
    {#if code?.path === route.path}
      <table class="code snip">
        <tbody>
          {#each lines.slice(lo - 1, lo - 1 + TEST_BEFORE + TEST_AFTER + 1) as l, i}
            <tr class:hl={route.line === lo + i}>
              <td class="ln"><a href={u({ tab: "code", path: route.path, line: lo + i })}>{lo + i}</a></td>
              <td class="src">{@html l}</td>
            </tr>
          {/each}
        </tbody>
      </table>
      <p class="muted note"><a href={u({ tab: "code", path: route.path, line: route.line })}>Open the whole file</a>. A tag is evidence, not proof that the test passes.</p>
    {:else}<p class="muted">Loading {route.path}…</p>{/if}
    {/if}
  {/if}
{/snippet}

{#snippet navLink(t: string)}
  <a class="tab" class:on={route.tab === t} aria-current={route.tab === t ? "page" : undefined} href={u({ tab: t as Route["tab"], view: t === "requirements" && reqView !== "list" ? reqView : undefined })} title={t} aria-label={t}>
    <span class="ico"><Icon name={t as "dashboard"} /></span><span class="lbl">{t}</span>
    {#if t === "dashboard" && report?.total_problems}<span class="badge" data-tone="bad" title="{report.total_problems} problems">{report.total_problems}</span>{/if}
    {#if t === "changes" && directReqs.length}<span class="badge" data-tone="accent" title="{directReqs.length} requirements may be affected">{directReqs.length}</span>{/if}
  </a>
{/snippet}

<header>
  <h1><a href="/">spec17</a></h1>
  <span class="dot" data-tone={up === true ? "ok" : up === false ? "bad" : undefined} role="status" title={up === null ? "server: connecting" : up ? "server: connected" : "server: disconnected, retrying"}></span>
  <a class="root mono muted" href="/">{root}</a>
  <span class="grow"></span>
  <button class="icon" disabled={busy} onclick={rescan} title="Rescan the repo (syncs the CodeGraph index first)" aria-label="rescan"><Icon name="rescan" /></button>
  <button class="icon" onclick={() => (theme = nextTheme.v)} title="Theme: {curTheme.name} (click for {nextTheme.name})" aria-label="Theme: {curTheme.name}, switch to {nextTheme.name}"><Icon name={curTheme.icon} /></button>
  <a class="tab icon" class:on={route.tab === "settings"} aria-current={route.tab === "settings" ? "page" : undefined} href={u({ tab: "settings" })} title={setupBadge ? "Settings (needs attention)" : "Settings"} aria-label="Settings"><Icon name="settings" />{#if setupBadge}<span class="badge" data-tone={setupBadge} title="needs attention: open Settings">!</span>{/if}</a>
</header>

<div class="shell" class:rail={rail || mobileNav.current}>
<nav aria-label="views">
  {#each NAV as g}
    <div class="group">
      <span class="gname">{g.name}</span>
      {#each g.tabs as t}{@render navLink(t)}{/each}
    </div>
  {/each}
  <span class="grow"></span>
  <div class="group">
    <button class="collapse" onclick={toggleNav} aria-label={rail ? "Expand the sidebar" : "Collapse the sidebar"} title={rail ? "Expand the sidebar" : "Collapse the sidebar"}><span class="ico"><Icon name={rail ? "expand" : "collapse"} /></span><span class="lbl">Collapse</span></button>
  </div>
</nav>
<div class="content">
{#if error}<p class="error">{error}</p>{/if}
{#if setup && setup.suggestions.length && route.tab !== "settings" && dismissed !== setup.suggestions[0]}
  <p class="banner">
    <span>spec17 is not fully set up for this repo: {setup.suggestions[0]}</span>
    <span class="grow"></span>
    <a class="tab" href="/settings">Open Settings</a>
    <button aria-label="dismiss" title="Hide until the message changes" onclick={() => dismiss(setup!.suggestions[0])}>✕</button>
  </p>
{/if}

<main>
  {#if route.tab === "settings"}
    {#if !setup}
      {#if setupError}<p class="error">Could not check the setup: {setupError} <button onclick={loadSetup}>Retry</button></p>{:else}<p class="muted">Checking…</p>{/if}
    {:else}
      {@const cgBin = checks.find((k) => k.name === "codegraph binary")}
      {@const cgIdx = checks.find((k) => k.name === "codegraph index")}
      {@const cgMcp = setup.status.project.codegraph_mcp === "ok" ? "this repo" : setup.status.global.codegraph_mcp === "ok" ? "global" : ""}
      {@const passed = checks.filter(k => k.level === "ok")}
      {@const issues = checks.filter(k => k.level !== "ok")}
      {@const where = globalScope ? setup.status.global : setup.status.project}
      {@const scope = globalScope ? "Global" : "This repo"}
      {@const fails = issues.filter((k) => k.level === "fail").length}
      {@const tone = fails ? "bad" : issues.length ? "warn" : "ok"}
      <!-- (WEB-44, WEB-42) -->
      <div class="setup-page">
      <section class="card setup health" data-tone={tone} aria-labelledby="health-title">
        <div class="health-head">
          <span class="health-ico"><Icon name={fails ? "x-circle" : issues.length ? "alert" : "check-circle"} /></span>
          <div>
            <div class="eyebrow muted">Health{#if setup.version} · spec17 {setup.version}{/if}</div>
            <h2 id="health-title" role="status">{fails ? plural(fails, "check") + " failed" : issues.length ? plural(issues.length, "warning") : checks.length ? "All checks passed" : "No checks run yet"}</h2>
            <div class="muted">{passed.length} of {checks.length} checks passed{#if issues.length} · {plural(issues.length, "issue")}{/if}</div>
          </div>
          <button class="end" disabled={checking || busy} onclick={loadSetup}>{checking ? "Checking…" : "Run checks"}</button>
        </div>
        {#if setupError}<p class="error" role="alert">Could not refresh checks: {setupError}</p>{/if}
        {#snippet checkRow(k: (typeof checks)[number])}
          <div class="check-row">
            <Pill tone={levelTone(k.level)}>{k.level === "ok" ? "Passed" : k.level === "warn" ? "Warning" : "Failed"}</Pill>
            <div><strong>{k.name}</strong> · {k.detail}{#if k.hint}<div class="muted">{k.hint}</div>{/if}</div>
          </div>
        {/snippet}
        {#each issues as k}{@render checkRow(k)}{/each}
        {#if passed.length}
          <details><summary>{issues.length ? `${plural(passed.length, "check")} passed` : "Show the checks"}</summary>
            {#each passed as k}{@render checkRow(k)}{/each}
          </details>
        {:else if !issues.length}<p class="muted">No check results available.</p>{/if}
      </section>
      <section class="card setup">
        <div class="setup-head">
          <h2>Agent setup</h2>
          <Pill tone={setup.suggestions.length ? "warn" : "ok"}>{setup.suggestions.length ? "Needs attention" : "Ready"}</Pill>
          <div class="end"><Seg label="Scope" options={[{key: "repo", label: "This repo"}, {key: "global", label: "Global"}]} value={globalScope ? "global" : "repo"} onpick={(k) => globalScope = k === "global"} /></div>
        </div>
        {#each setup.suggestions as suggestion}<p>{suggestion}</p>{/each}
        <!-- (WEB-38, WEB-40, WEB-41) -->
        {#snippet item(name: string, state: string, path: string, transport?: "stdio" | "http", connection?: string | null)}
          {@const status = state === "current" || state === "ok" ? "Installed" : state === "missing" ? "Not configured" : state === "other" ? "Custom entry" : state}
          {@const options = transport ? { only: "mcp" as const, transport } : { only: "skill" as const }}
          <div class="setup-item">
            <div>
              <strong>{name}</strong> <Pill tone={status === "Installed" ? "ok" : status === "Not configured" ? "muted" : "warn"}>{status}</Pill>
              {#if transport}<span class="muted transport-note">{transport === "stdio" ? "Starts a process per session" : "Connects to a running server"}</span>{/if}
              <div><code>{path}</code></div>
              {#if connection}<div><code>{connection}</code></div>{/if}
            </div>
            <div class="actions">
              {#if state === "missing" || state === "outdated"}
                <button disabled={busy} aria-label="{state === 'outdated' ? 'Update' : 'Install'} {name} ({scope})" onclick={() => runSetup("install", globalScope, options)}>{state === "outdated" ? "Update" : "Install"}</button>
              {/if}
              {#if state !== "missing"}
                <button disabled={busy} aria-label="Remove {name} ({scope})" onclick={() => confirm(`Remove ${name} (${scope}) from ${path}?`) && runSetup("uninstall", globalScope, options)}>Remove</button>
              {/if}
            </div>
          </div>
        {/snippet}
        {@render item("Agent skill", where.skill, where.skill_path)}
        {#if where.mcp_stdio !== undefined}
          {@render item("Stdio MCP", where.mcp_stdio, where.mcp_path, "stdio", launchCommand(where.mcp_details?.stdio))}
          {@render item("HTTP MCP", globalScope ? where.mcp_file : where.mcp, where.mcp_path, "http", where.mcp_details?.http_url)}
        {:else}<p>Update spec17 and restart the server to manage MCP transports.</p>{/if}
        <p class="muted"><a href="{docs}/agents/" target="_blank" rel="noopener noreferrer">Agent docs</a>: connect a client, the tools it gets.</p>
        {#if globalScope && where.mcp !== "missing"}
          <p class="muted">Claude user MCP entry {where.mcp === "ok" ? "installed" : "customised"} · <code>~/.claude.json</code> (read-only)</p>
        {/if}
      {#if noReqs}
        <section>
          <h3>No specs yet</h3>
          <p>spec17 found no requirements in this repo and works without them: Modules and Tests still list its sources and tests. {setup.suggestions.length ? "Install the skill, then ask" : "Ask"} your agent to draft <code>specs/01-name.md</code> with you (lines like <code>- **ABC-12** MUST — text</code>). <a href="{docs}/specs/#requirement-styles" target="_blank" rel="noopener noreferrer">Requirement formats</a>.</p>
        </section>
      {/if}
      </section>
      <section class="card setup">
        <div class="setup-head"><h2>CodeGraph</h2><Pill tone="muted">Optional</Pill>
          {#if cgBin?.level === "ok"}
            {#if indexAction === "init"}<button class="end" disabled={busy} onclick={() => runIndex("init")}>Create index</button>
            {:else if cgIdx?.level === "ok" || indexAction === "sync"}<button class="end" disabled={busy} onclick={() => runIndex("sync")}>Sync index</button>{/if}
          {/if}
        </div>
        {#if indexLog}<pre class="log" aria-live="polite">{indexLog}</pre>{/if}
        <details open={cgBin?.level !== "ok" || cgIdx?.level !== "ok" || !cgMcp}>
          <summary>{cgBin?.level === "ok" ? "Installed" : "Installation needed"} · {cgMcp ? `Connected (${cgMcp})` : "Agent disconnected"} · {cgIdx?.level === "ok" ? "Index current" : "Check index"}</summary>
          <p class="muted">Enables test-to-code links, overlap and uncovered-code analysis. <a href="{docs}/usage/#codegraph" target="_blank" rel="noopener noreferrer">How spec17 uses it</a>.</p>
          {#if cgBin}<p><strong>Installation</strong> · {cgBin.detail}</p>{/if}
        {#if cgBin && cgBin.level !== "ok"}
          <pre class="log">curl -fsSL https://raw.githubusercontent.com/colbymchenry/codegraph/main/install.sh | sh
npm i -g @colbymchenry/codegraph
irm https://raw.githubusercontent.com/colbymchenry/codegraph/main/install.ps1 | iex   # Windows PowerShell</pre>
          <p class="muted">After installing, select Run checks in Health. <a href="https://colbymchenry.github.io/codegraph/getting-started/installation/" target="_blank" rel="noreferrer">Installation docs</a>. spec17 never installs it for you.</p>
        {/if}
        <p>
          <strong>Agent connection</strong> ·
          {#if setup.status.project.codegraph_mcp === "ok"}
            <button disabled={busy} onclick={() => confirm("Remove the CodeGraph server from this repo’s .mcp.json?") && runSetup("uninstall", false, { server: "codegraph" })}>Remove from this repo</button>
          {:else}
            <button disabled={busy} onclick={() => runSetup("install", false, { server: "codegraph" })}>Connect this repo</button>
          {/if}
        </p>
        {#if cgIdx}<p><strong>Index</strong> · {cgIdx.detail}</p>{/if}
        </details>
      </section>
      </div>
    {/if}
  {:else if !report}
    {#if error}<p class="muted">The scan failed (see above). <a href="/settings">Settings</a> shows what is missing.</p>{:else}<p class="muted">Scanning…</p>{/if}
  {:else if route.tab === "dashboard"}
    {#if noReqs}
      <section class="hero card">
        <h2>No requirements yet</h2>
        <p>spec17 found none. Write them as <code>- **ABC-12** MUST — text</code> in <code>specs/01-name.md</code> or <code>SPEC.md</code>, tag tests <code>[ABC-12]</code> and cite <code>// (ABC-12)</code> in code. <a href="/settings">Settings</a> installs the skill so your agent drafts them with you. <a href="{docs}/specs/#requirement-styles" target="_blank" rel="noopener noreferrer">Requirement formats</a>.</p>
        {#if modules}<p class="facts muted"><a href={u({ tab: "modules" })}>{plural(sum("src_files"), "source file")}</a> · {plural(sum("test_files"), "test file")} · <a href={u({ tab: "tests" })}>{plural(sum("tests"), "test")} detected</a></p>{/if}
      </section>
    {:else}
    <section class="hero card">
      <div class="pct"><b>{report.tested_pct}%</b><span>of due requirements have a tagged test <span class="muted metric-count">({report.tested}/{report.due})</span></span></div>
      <div class="meter" role="img" aria-label="{report.tested_pct}% of due requirements have a tagged test"><i style="width:{report.tested_pct}%"></i></div>
      <p class="facts muted">
        <a href={u({ tab: "requirements" })}>{report.requirements} requirements</a> in {report.spec_files} specs
        {#if report.manual} · <a href={u({ tab: "requirements", state: "manual" })}>{report.manual} manual</a>{/if}
        {#if report.later} · <a href={u({ tab: "requirements", state: "later" })}>{report.later} later</a>{/if}
        {#if report.refs} · <a href={u({ tab: "requirements", state: "ref" })}>{report.refs} reference docs</a>{/if}
        · <b data-tone={report.total_problems ? "bad" : "ok"}>{report.total_problems ? `${report.total_problems} problems` : "no problems found"}</b>
      </p>
    </section>
    {/if}
    {#if !report.index}<p class="note-line muted">No CodeGraph index: test-to-code links and uncovered code are not checked.</p>{/if}
    {#if changes}
      <Section title="Changes" count={changes.files.length} tone={changes.files.length ? "accent" : undefined}>
        {#if !changes.files.length}<p class="muted">Working tree clean: nothing to compare with HEAD.</p>
        {:else}
          <p>{#each tallyBy(changes.files) as [c, n]}<Pill tone={changeTone(c)}>{n} {c}</Pill>{/each}</p>
          <p>
            <a href={u({ tab: "requirements", changed: "1" })}>{plural(directReqs.length, "requirement")}</a> may be affected{#if noTestN} · <b data-tone="bad">{noTestN} without a test</b>{/if}
            <span class="muted"> · evidence of edits, not of done</span>
          </p>
        {/if}
        <p><a href={u({ tab: "changes" })}>All changes</a></p>
      </Section>
    {/if}
    {#if modules?.modules.length}
      <Section title="Modules" count={modules.modules.length}>
        <table>
          <tbody>
            {#each [...modules.modules].sort((a, b) => b.tests - a.tests || a.path.localeCompare(b.path)).slice(0, 8) as m}
              <tr>
                <td class="mono nowrap"><a href={u({ tab: "modules", module: m.path })}>{m.path}</a></td>
                <td class="muted">{m.languages.join(", ")}</td>
                <td class="nowrap">{m.tests} tests</td>
                <td class="muted nowrap">{m.src_files + m.test_files} files</td>
                <td class="muted">{#if m.depends_on.length}→ {m.depends_on.slice(0, 3).map((d: any) => d.module).join(", ")}{/if}</td>
              </tr>
            {/each}
          </tbody>
        </table>
        <p><a href="/modules">All {modules.modules.length} modules</a></p>
      </Section>
    {/if}

    {#snippet ids(title: string, list: string[], sev: Tone)}
      {#if list.length}
        <Section {title} count={list.length} tone={sev}>
          <p class="ids">{#each list as id}{@render requirementLink(id, req(id))}{/each}</p>
        </Section>
      {/if}
    {/snippet}
    {@render ids("Unimplemented: due, no test and no code", report.unimplemented, "bad")}
    {@render ids("Untested: due, cited in code but no test", report.untested, "warn")}
    {@render ids("Marked later but already has tests or code", report.built, "warn")}

    {#if report.unproven.length}
      <Section title="Check-marked done without proof" count={report.unproven.length} tone="warn">
        <ul>{#each report.unproven as x}<li>{@render reqRef(x.id, req(x.id))} <code>{x.verdict}</code> <a href={u(doc(x.file, x.id))}>{x.file}:{x.line}</a> <span class="muted">{plain(x.text).slice(0, 80)}</span></li>{/each}</ul>
      </Section>
    {/if}
    {#if invalidN}
      <Section title="Invalid tests" count={invalidN} tone="bad">
        <p class="muted"><a href="{docs}/specs/#tests-and-citations" target="_blank" rel="noopener noreferrer">What these mean</a></p>
        {#each Object.entries(report.invalid_tests) as [kind, list]}
          {#if (list as any[]).length}<p><code>{kind}</code> {#each list as any[] as p}{@render requirementLink(p.id, req(p.id))} <a class="muted mono" href={u({ tab: "code", path: p.file, line: p.line })}>{p.file}:{p.line}</a> {/each}</p>{/if}
        {/each}
      </Section>
    {/if}
    {#if report.spec_issues.length}
      <Section title="Spec issues" count={report.spec_issues.length} tone="bad">
        <ul>{#each report.spec_issues as x}<li class="mono">{x}</li>{/each}</ul>
      </Section>
    {/if}
    {#if report.dangling_tests.length + report.dangling_sources.length}
      <Section title="Dangling citations" count={report.dangling_tests.length + report.dangling_sources.length} tone="warn">
        <p class="muted"><a href="{docs}/specs/#tests-and-citations" target="_blank" rel="noopener noreferrer">What these mean</a></p>
        <p class="mono">{#if report.dangling_tests.length}tests: {report.dangling_tests.join(", ")}<br />{/if}{#if report.dangling_sources.length}sources: {report.dangling_sources.join(", ")}{/if}</p>
      </Section>
    {/if}
  {:else if inDocs}
    {#if specFiles === null}<p class="muted">Loading…</p>
    {:else if !specFiles.length}<p class="muted empty">No spec files found. <a href="/settings">Settings</a> says what spec17 looks for; <a href="{docs}/specs/#requirement-styles" target="_blank" rel="noopener noreferrer">the docs</a> show the formats.</p>
    {:else}
      {@const f = docFile!}
      <div class="page">
        <div class="bar">
          {@render viewSeg()}
          <button type="button" class="filter-toggle" aria-pressed={treeOpen} title="Show or hide the document tree" onclick={toggleTree}>Files</button>
          {#if showChanged}<Seg label="Changes" options={changedSeg(specFiles.filter((x) => fileOf.has(x)).length)} value={onlyChanged ? "changed" : "all"} onpick={pickChanged} />{/if}
        </div>
        <div class="docs">
          {#if treeOpen}<aside class="tree"><FileTree files={docFiles} selected={f} href={(d) => u(doc(d, undefined, route.changed))} mark={changeMark} /></aside>{/if}
          <div class="doc">
            {#if fileOf.has(f)}<p><ViewSwitch /></p>{/if}
            {#if diffOn(fileOf.has(f))}<DiffView paths={[f]} {gen} />{:else}<article><Markdown source={specMd} mark={(id) => (known.has(id) ? edgeTone(byId.get(id)!.state) : undefined)} /></article>{/if}
          </div>
        </div>
      </div>
    {/if}
  {:else if route.tab === "requirements" && noReqs}
    <p class="muted empty">No requirements found. <a href="/settings">Settings</a> says what spec17 looks for; <a href="{docs}/specs/#requirement-styles" target="_blank" rel="noopener noreferrer">the docs</a> show the formats.</p>
  {:else if route.tab === "requirements"}
    <div class="page">
    {#if route.view === "graph"}
      <div class="bar req-toolbar"><input name="filter" aria-label="Filter requirements" placeholder="Filter requirements" value={query} oninput={(e) => filter({ q: e.currentTarget.value })} />{@render viewSeg()}</div>
      <p class="muted">References are detected from requirement text and may not mean a dependency.</p>
      {#if requirementCycles.length}
        <p><b>Circular reference groups:</b> {#each requirementCycles as group}<span class="cycle-group">{"{"}{#each group as id, i}{#if i}{", "}{/if}<a href={u(req(id))}>{id}</a>{/each}{"}"}</span>{/each}</p>
      {:else}<p class="muted">No circular references detected.</p>{/if}
      <Graph modules={requirementGraph} requirements q={query} cycleGroups={requirementCycles} open={(id) => navigate(req(id))} />
    {:else}
    {@render requirementToolbar()}
    <Split showPreview={!!route.id}>
      {#snippet list()}
        <DataTable rows={shown} columns={reqCols} cell={reqCell} rowKey={(r) => r.id} selected={route.id} onselect={(k, how) => navigate({ ...route, id: k }, how === "key")} value={reqVal} groupOf={(r) => prefixOf(r.id)} tone={(r) => edgeTone(r.state)} storageKey="spec17.collapsed">
          {#snippet empty()}<p class="muted empty">No requirement matches. <a href={u({ tab: "requirements" })}>Clear filters</a></p>{/snippet}
        </DataTable>
      {/snippet}
      {#snippet preview()}{@render tracePanel()}{/snippet}
    </Split>
    {/if}
    </div>
  {:else if route.tab === "code"}
    {#if !route.path}<p class="muted">No file selected: open one from a requirement's tests or citations.</p>
    {:else if !code || code.path !== route.path}<p class="muted">Loading {route.path}…</p>
    {:else}
      <p class="mono crumb">
        <span class="nowrap"><a href="/requirements">requirements</a> / {code.path}{route.line ? `:${route.line}` : ""}</span>
        <button class="small" title="Copy the file path" onclick={() => copyPath(code!.path)}>{copied ? "Copied" : "Copy path"}</button>
        {#if root}<a class="btn small" title="Open in {editor}" href={EDITORS[editor](`${root}/${code.path}`, route.line ?? 1)}>Open in</a>{/if}
        <select class="small" bind:value={editor} onchange={() => save("spec17.editor", editor)} aria-label="Editor">{#each Object.keys(EDITORS) as e}<option>{e}</option>{/each}</select>
        <button class="small" class:on={wrap} aria-pressed={wrap} onclick={() => { wrap = !wrap; save("spec17.wrap", wrap ? "1" : "0"); }}>Wrap</button>
        {#if fileOf.has(code.path)}<ViewSwitch />{/if}
      </p>
      {#if diffOn(fileOf.has(code.path))}<DiffView paths={[code.path]} {gen} />
      {:else}
      <table class="code" class:wrap>
        <tbody>
          {#each lines as l, i}
            <tr id={`L${i + 1}`} class:hl={route.line === i + 1}>
              <td class="ln"><a href={u({ tab: "code", path: code.path, line: i + 1 })}>{i + 1}</a></td>
              <td class="src">{@html l}</td>
            </tr>
          {/each}
        </tbody>
      </table>
      {/if}
    {/if}
  {:else if route.tab === "modules" && route.module}
    <p class="mono nowrap crumb"><a href="/modules">modules</a> / {route.module}</p>
    {#if !modules}<p class="muted">Loading…</p>
    {:else if !modOf}<p class="muted empty">No module <code>{route.module}</code>. <a href="/modules">All modules</a></p>
    {:else}
      <section class="card block">
        <h2 class="mono">{modOf.path}</h2>
        <p>
          {#if modOf.manifest}<Pill>{modOf.manifest}</Pill>{/if}
          {#each modOf.languages as l}<Pill>{l}</Pill>{/each}
          <span class="muted">{plural(modOf.src_files, "source file")} · {plural(modOf.test_files, "test file")} · {plural(symbolsOf(modOf), "symbol")}</span>
        </p>
        {#if Object.keys(modOf.symbols).length}<p class="muted">{Object.entries(modOf.symbols).map(([k, n]) => `${n} ${k}`).join(" · ")}</p>{/if}
        {#if modOf.depends_on.length}<p>depends on {#each modOf.depends_on as d}<IdLink href={u({ tab: "modules", module: d.module })}>{d.module}</IdLink>{" "}{/each}</p>{/if}
        {#if usedBy.length}<p>used by {#each usedBy as d}<IdLink href={u({ tab: "modules", module: d.path })}>{d.path}</IdLink>{" "}{/each}</p>{/if}
      </section>
      <h2>Sources <span class="muted">{modOf.sources.length}</span></h2>
      {#if modOf.sources.length}<p class="mono">{#each modOf.sources as f}<IdLink href={u({ tab: "code", path: f })}>{f}</IdLink>{" "}{/each}</p>{:else}<p class="muted">No source files in this module.</p>{/if}
      <h2>Tests <span class="muted">{modOf.tests}</span> <a class="more" href={u({ tab: "tests", module: modOf.path })}>filter in Tests</a></h2>
      {#if tests === null}<p class="muted">Loading tests…</p>
      {:else if !testsOf(modOf.path)}<p class="muted">No tests detected in this module.</p>
      {:else}{@render cases(testsOf(modOf.path).files)}{/if}
      {#if tests}<p class="muted note">{tests.note}</p>{/if}
    {/if}
  {:else if route.tab === "modules"}
    <div class="bar">
      <input name="filter" aria-label="filter modules" placeholder="Filter modules" value={query} oninput={(e) => filter({ q: e.currentTarget.value })} />
      <Filters active={Number(onlyChanged) + Number(showHidden && modView === "list")}>
        {#if showChanged}<Seg label="Changes" options={changedSeg(changedMods.size)} value={onlyChanged ? "changed" : "all"} onpick={pickChanged} />{/if}
        {#if modView === "list" && modules?.hidden?.length}
          <label class="muted nowrap"><input type="checkbox" checked={showHidden} onchange={(e) => save("spec17.hidden", (showHidden = e.currentTarget.checked) ? "1" : "0")} /> show hidden ({modules.hidden.length})</label>
        {/if}
      </Filters>
      <span class="muted">{modules?.modules.length ?? 0} modules</span>
      <Seg label="View" options={VIEWS} value={modView} onpick={(k) => save("spec17.modules", (modView = k))} />
    </div>
    {#if modules && !modules.index}<p class="note-line muted">{modules.note}</p>{/if}
    {#if modView === "graph"}
      {#if !modules}<p class="muted">Loading…</p>
      {:else if !modules.index}<!-- the note above says why there is no graph -->
      {:else if !modules.modules.some((m: any) => m.depends_on.length)}<p class="muted empty">No dependencies between modules in the index.</p>
      {:else}<Graph modules={modules.modules} q={query} open={(p) => navigate({ tab: "modules", module: p })} />{/if}
    {:else if mods.length || (showHidden && hiddenMods.length)}
      <table>
        <thead><tr>{@render th("module", "path", modSort)}{@render th("manifest", "manifest", modSort)}{@render th("languages", "languages", modSort)}{@render th("source", "src_files", modSort, "num")}{@render th("test files", "test_files", modSort, "num")}{@render th("tests", "tests", modSort, "num")}{@render th("symbols", "symbols", modSort, "num")}{@render th("depends on", "depends", modSort, "num")}<th></th></tr></thead>
        <tbody>
          {#each mods as m (m.path)}
            <tr>
              <td class="mono nowrap"><a href={u({ tab: "modules", module: m.path })}>{m.path}</a>{#if changedMods.has(m.path)}{" "}<Pill tone="accent" title="files of this module changed in the working tree">✎ changed</Pill>{/if}</td>
              <td class="muted">{m.manifest ?? ""}</td>
              <td class="muted">{m.languages.join(", ")}</td>
              <td class="num">{m.src_files}</td>
              <td class="num">{m.test_files}</td>
              <td class="num"><a href={u({ tab: "tests", module: m.path })}>{m.tests}</a></td>
              <td class="num">{symbolsOf(m)}</td>
              <td class="num">{m.depends_on.length}</td>
              <td>{#if m.path !== "."}<button type="button" class="ignore" title="Add /{m.path}/ to .spec17ignore" onclick={() => setHidden(m.path, true)}>ignore</button>{/if}</td>
            </tr>
          {/each}
          {#if showHidden}
            {#each hiddenMods as path (path)}
              <tr class="hidden-row">
                <td class="mono nowrap">{path}</td>
                <td class="muted" colspan="7">hidden by .spec17ignore</td>
                <td><button type="button" class="ignore" title="Remove /{path}/ from .spec17ignore" onclick={() => setHidden(path, false)}>unhide</button></td>
              </tr>
            {/each}
          {/if}
        </tbody>
      </table>
    {:else if modules}<p class="muted empty">No module matches. <a href="/modules">Clear filter</a></p>{/if}
  {:else if route.tab === "tests"}
    <div class="page">
    <div class="bar">
      <input name="filter" aria-label="filter tests" placeholder="Filter test, file, requirement ID   ( / )" value={query} oninput={(e) => filter({ q: e.currentTarget.value })} />
      <Filters active={[stateFilter !== "all", route.module, onlyChanged].filter(Boolean).length}>
        <Seg label="Tagged" options={testOpts} value={stateFilter} onpick={(k) => filter({ state: k })} />
        {#if showChanged}<Seg label="Changes" options={changedSeg(allCases.filter((c) => fileOf.has(c.file)).length)} value={onlyChanged ? "changed" : "all"} onpick={pickChanged} />{/if}
        <select aria-label="module" value={route.module ?? ""} onchange={(e) => filter({ module: e.currentTarget.value || undefined })}>
          <option value="">all modules</option>
          {#each modules?.modules ?? [] as m}<option value={m.path}>{m.path}</option>{/each}
        </select>
      </Filters>
      {#if testRequirement}
        <a href={u({ ...route, requirementId: undefined })} aria-label="Clear requirement filter {testRequirement}">{testRequirement} ×</a>
      {/if}
      <SplitToggle />
    </div>
    {#if tests === null}<p class="muted">Loading…</p>
    {:else}
      <p class="muted found">{tests.total} tests detected in {tests.modules.length} modules, {taggedN} tagged with a requirement. {tests.note}</p>
      <Split>
        {#snippet list()}
          <DataTable rows={testRows} columns={testCols} rowKey={(c) => c.key} selected={selTest} onselect={(k, how) => { const c = testRows.find((x) => x.key === k); navigate({ ...route, path: c?.file, line: c?.t.line }, how === "key"); }} value={testVal} groupOf={(c) => c.module} storageKey="spec17.collapsed.tests">
            {#snippet cell(c, k)}
              {#if k === "name"}<span class="mono" title={c.t.qualified}>{c.t.name}</span>{#if changeMark(c.file)}{" "}<Pill tone={changeTone(changeMark(c.file))} title="its file is {changeMark(c.file)} in the working tree">✎ {changeMark(c.file)}</Pill>{/if}
              {:else if k === "file"}<a href={u({ tab: "code", path: c.file, line: c.t.line })}>{c.file}:{c.t.line}</a>
              {:else}<div class="test-links">{#each c.t.ids as id}{@render requirementLink(id, known.has(id) ? req(id) : { tab: "requirements", q: id })}{/each}{#if !c.t.ids.length}<span class="muted">untagged</span>{/if}</div>{/if}
            {/snippet}
            {#snippet empty()}<p class="muted empty">No test matches. <a href="/tests">Clear filters</a></p>{/snippet}
          </DataTable>
        {/snippet}
        {#snippet preview()}{@render testPanel()}{/snippet}
      </Split>
    {/if}
    </div>
  {:else if route.tab === "changes"}
    <div class="bar">
      {#if history?.base && !history.on_base}
        <Seg label="Compare with" hint="What the changes are measured against" options={[{ key: "head", label: "HEAD" }, { key: "base", label: history.base }, ...(scope.commit ? [{ key: "commit", label: scope.commit.slice(0, 7) }] : [])]} value={scope.commit ? "commit" : scope.ref === history.base ? "base" : "head"} onpick={(k) => filter({ ref: k === "base" ? history.base : undefined, commit: k === "commit" ? scope.commit : undefined })} />
      {/if}
      {#if !scope.commit}
        <Seg label="Scope" hint="Staged, unstaged and untracked, or only the index" options={[{ key: "all", label: "staged + unstaged" }, { key: "staged", label: "staged only" }]} value={scope.staged ? "staged" : "all"} onpick={(k) => filter({ staged: k === "staged" ? "1" : undefined })} />
        <label class="muted nowrap"><input type="checkbox" checked={scope.include_untracked && !scope.staged} disabled={scope.staged} onchange={(e) => filter({ untracked: e.currentTarget.checked ? undefined : "0" })} /> untracked files</label>
      {/if}
      <label class="muted nowrap"><input type="checkbox" checked={scope.ignore_whitespace} onchange={(e) => filter({ ws: e.currentTarget.checked ? "1" : undefined })} /> ignore whitespace</label>
      <label class="muted nowrap" title="Files .aiignore, .spec17ignore and .agents hide"><input type="checkbox" checked={scope.show_ignored} onchange={(e) => filter({ ignored: e.currentTarget.checked ? "1" : undefined })} /> ignored files</label>
      <label class="muted nowrap" title="Files spec17 does not scan"><input type="checkbox" checked={scope.show_all} onchange={(e) => filter({ all: e.currentTarget.checked ? "1" : undefined })} /> unscanned files</label>
    </div>
    {#if history?.commits.length}
      <Section id="changes-commits" title={history.on_base ? "Recent commits" : `Commits not on ${history.base}`} count={history.commits.length} tone={history.on_base ? undefined : "accent"}>
        {#each history.commits.filter((c: any) => scope.commit && (c.sha === scope.commit || c.short === scope.commit)) as c}<p class="note"><b>Explaining</b> <span class="mono">{c.short}</span> {c.subject} <a href={u({ tab: "changes" })}>clear</a></p>{/each}
        <div class="commits"><table>
          <tbody>
            {#each history.commits as c (c.sha)}
              <tr class:sel={scope.commit === c.sha || scope.commit === c.short}>
                <td class="mono nowrap"><a href={u({ tab: "changes", commit: c.sha })} title="Explain this commit">{c.short}</a></td>
                <td class="muted nowrap">{c.date.slice(0, 10)}</td>
                <td class="muted nowrap">{c.author}</td>
                <td>{c.subject}</td>
              </tr>
            {/each}
          </tbody>
        </table></div>
        {#if history.uncommitted}<p class="muted note">{plural(history.uncommitted, "file")} uncommitted{scope.commit ? "; pick HEAD above to see them" : ""}.</p>{/if}
      </Section>
    {/if}
    {#if view === undefined}<p class="muted">Loading…</p>
    {:else if view === null}<p class="muted empty">Not checked: this is not a git work tree, it has no commits yet, or git is not available.</p>
    {:else if !view.files.length}<p class="muted empty">{view.commit ? "This commit changed no scanned file." : `Nothing ${scope.staged ? "staged" : "changed"} against ${view.reference}.`}{#if !scope.show_all} Lockfiles, CI and build files are not scanned: <a href={u({ ...route, all: "1" })}>show unscanned files</a>.{/if}</p>
    {:else}
      {@const direct = shown.filter(isDirect)}
      {@const indirect = shown.filter((r) => !isDirect(r))}
      {#snippet reqTable(rows: any[])}
        <DataTable {rows} columns={reqCols} cell={reqCell} rowKey={(r) => r.id} onselect={(id, how) => how === "click" && id && known.has(id) && navigate(req(id))} value={reqVal} tone={(r: any) => (r.deleted ? "red" : edgeTone(r.state))} keys={false} />
      {/snippet}
      <Section id="changes-requirements" title={`Requirements that may be affected${at}`} count={direct.length} tone={direct.length ? "accent" : undefined}>
        {#if reqFilterRows.length || filtering}{@render requirementToolbar()}{/if}
        {#if filtering && !shown.length}<p class="muted">No affected requirement matches the filters.</p>
        {:else if !direct.length && !filtering}<p class="muted">No requirement's bullet, tagged test or cited code differs from {view.commit ? "its parent" : view.reference}.</p>
        {:else if !direct.length}<p class="muted">No directly affected requirement matches the filters.</p>
        {:else}{@render reqTable(direct)}{/if}
        {#if indirect.length}<details class="indirect"><summary class="muted">{indirect.length} more reached only through tests that run the changed code (not edited themselves)</summary>{@render reqTable(indirect)}</details>{/if}
        <p class="muted note">A changed file may affect a requirement; it does not say the requirement is done or its test passes. Evidence, not truth.</p>
      </Section>
      <Section id="changes-files" title={`Files${at}`} count={view.files.length} tone="accent">
        <p class="note"><button onclick={() => (openDiffs = view.files.map((f: any) => f.path))}>Expand all diffs</button> <button onclick={() => (openDiffs = [])} disabled={!openDiffs.length}>Collapse all diffs</button></p>
        <table>
          <tbody>
            {#each view.files as f (f.path)}
              <tr id={`changed-file-${f.path}`}>
                <td class="nowrap"><Pill tone={changeTone(f.change)}>{f.change}</Pill></td>
                <td class="muted nowrap">{f.stage}</td>
                <td class="muted nowrap">{f.kind}</td>
                <td class="mono">
                  {#if f.change === "deleted" || f.kind === "other"}{f.path}
                  {:else if f.kind === "spec"}<a href={u(doc(f.path))}>{f.path}</a>
                  {:else}<a href={u({ tab: "code", path: f.path })}>{f.path}</a>{/if}
                  {#if f.old_path}<span class="muted"> ← {f.old_path}</span>{/if}
                </td>
                <td class="nowrap"><button class="icon" class:on={openDiffs.includes(f.path)} aria-pressed={openDiffs.includes(f.path)} title={openDiffs.includes(f.path) ? "Hide the diff" : "Show the diff"} aria-label="{openDiffs.includes(f.path) ? 'Hide' : 'Show'} the diff of {f.path}" onclick={() => toggleDiff(f.path)}>±</button></td>
              </tr>
              {#if openDiffs.includes(f.path)}<tr class="diffrow"><td colspan="5"><button class="hidediff" onclick={() => toggleDiff(f.path)}>Hide diff ✕</button><DiffView paths={[f.path]} {gen} {scope} /></td></tr>{/if}
            {/each}
          </tbody>
        </table>
      </Section>
      <Section title="Tests, specs and modules">
        <p>
          <a href={u({ tab: "tests", changed: "1" })}>{plural(nOf("test"), "test file")}</a> ·
          <a href={u(doc(undefined, undefined, "1"))}>{plural(nOf("spec"), "spec file")}</a> ·
          <a href={u({ tab: "modules", changed: "1" })}>{plural(viewMods.size, "module")}</a>
          <span class="muted"> changed ({plural(nOf("src"), "source file")}, {plural(nOf("other"), "other file")} not scanned)</span>
        </p>
      </Section>
    {/if}
  {:else if route.tab === "overlap"}
    {#if overlap === null}<p class="muted">Loading…</p>
    {:else if overlap.length === 0}<p class="muted">No declaration is covered by two or more requirements (or no CodeGraph index).</p>
    {:else}
      {#each overlap as r}
        <div class="card block">
          <div class="mono nowrap"><a href={u({ tab: "code", path: r.file, line: r.line })}>{r.file}:{r.line}</a> <b>{r.name}</b></div>
          {#each r.pairs as [a, b, linked]}
            <div>{@render reqRef(a, withId("overlap", a))} {linked ? "~ linked" : "? UNLINKED"} {@render reqRef(b, withId("overlap", b))}</div>
          {/each}
        </div>
      {/each}
      {@render tracePanel(true)}
    {/if}
  {:else if route.tab === "uncovered"}
    {#if uncovered === null}<p class="muted">Loading…</p>
    {:else if !uncovered.files}<p class="muted">Not available (no CodeGraph index).</p>
    {:else}
      <p>{uncovered.units} units / {uncovered.lines} lines in {uncovered.files.length} files have no requirement. Candidates, not proof.</p>
      <table>
        <tbody>
          {#each uncovered.files as f}
            <tr>
              <!-- f.file is shown relative to the app folder; the viewer needs the repo-relative path every unit carries -->
              <td class="mono"><a href={u({ tab: "code", path: f.units[0].file })}>{f.file}</a></td>
              <td>{f.lines} lines</td>
              <td class="mono muted">{#each f.units as un}<a href={u({ tab: "code", path: un.file, line: un.lo })}>{un.name}:{un.lo}-{un.hi}</a>{" "}{/each}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  {/if}
</main>

<footer class="muted">
  Evidence, not truth: tags and citations do not prove a requirement is implemented or its test passes.
  <div>Copyright &copy; 2026 Konstantin Pavlov, <a href="https://itstaff.ee">Staff OÜ</a>. All rights reserved. · <a href={docs} target="_blank" rel="noopener noreferrer">Documentation</a> · <a href="https://github.com/kpavlov/spec17/issues" target="_blank" rel="noopener noreferrer">Feedback</a></div>
</footer>
</div>
</div>

<style>
  .commits { max-height: 13rem; overflow: auto; }
  tr.sel { background: var(--tint-sel); }
  /* layout */
  header { --header-h: 53px; position: sticky; top: 0; z-index: 3; display: flex; align-items: center; gap: var(--gap); height: var(--header-h); padding: 0 var(--gutter); background: color-mix(in srgb, var(--bg) 92%, transparent); backdrop-filter: blur(8px); border-bottom: 1px solid var(--line); }
  header > :not(.root) { flex-shrink: 0; }
  .dot { width: 8px; height: 8px; border-radius: 50%; background: var(--tone, var(--muted)); margin-left: -4px; }
  h1 { margin: 0; font-size: var(--text-section); font-weight: var(--weight-bold); letter-spacing: -0.01em; }
  /* sidebar: labels + icons, or an icon rail (.rail); a strip above the page on phones */
  .shell { display: grid; grid-template-columns: 12.5rem minmax(0, 1fr); min-height: calc(100dvh - 53px); }
  .shell.rail { grid-template-columns: 3.5rem minmax(0, 1fr); }
  /* the footer sits at the bottom of a short page and below a long one */
  .content { min-width: 0; display: flex; flex-direction: column; }
  .content > main { flex: 1; }
  nav { position: sticky; top: 53px; align-self: start; height: calc(100dvh - 53px); display: flex; flex-direction: column; gap: 2px; padding: 10px 8px; border-right: 1px solid var(--line); overflow-y: auto; scrollbar-width: none; }
  nav .group { display: flex; flex-direction: column; gap: 2px; }
  nav .group + .group, nav .grow + .group { border-top: 1px solid var(--line); margin-top: 6px; padding-top: 6px; }
  nav .grow { flex: 1; }
  nav .gname { padding: 6px 10px 2px; font-size: var(--text-sm); font-weight: var(--weight-semibold); color: var(--muted); }
  nav a.tab, nav .collapse { display: flex; align-items: center; gap: 10px; position: relative; padding: 6px 10px; white-space: nowrap; background: transparent; color: var(--fg); border-radius: var(--radius); text-decoration: none; }
  nav .collapse { color: var(--muted); font-size: var(--text-label); }
  nav a.tab:hover, nav .collapse:hover:not(:disabled) { background: var(--tint-btn); color: var(--accent); }
  nav a.tab.on { background: var(--tint-on); color: var(--accent); }
  nav .ico { display: flex; flex-shrink: 0; }
  nav .badge { margin-left: auto; }
  .rail nav .gname, .rail nav .lbl { display: none; }
  .rail nav { align-items: stretch; }
  .rail nav a.tab, .rail nav .collapse { justify-content: center; padding: 7px 0; }
  .rail nav .badge { position: absolute; top: 0; right: 2px; margin: 0; min-width: 14px; padding: 0 3px; font-size: .65rem; }
  @media (max-width: 48rem) {
    .shell, .shell.rail { grid-template-columns: minmax(0, 1fr); grid-template-rows: auto 1fr; }
    nav, .rail nav { position: static; height: auto; flex-direction: row; align-items: center; padding: 4px 8px; border-right: 0; border-bottom: 1px solid var(--line); overflow-x: auto; }
    nav .group, nav .group + .group, nav .grow + .group { flex-direction: row; border: 0; margin: 0; padding: 0; }
    nav .grow, nav .collapse { display: none; }
    nav a.tab, .rail nav a.tab { padding: 7px 10px; }
    .content .page { height: calc(100dvh - 150px); }
  }
  main { padding: var(--gutter); max-width: 100%; overflow-x: clip; }
  footer { padding: 14px 16px; text-align: center; font-size: .75rem; line-height: 1.4; color: color-mix(in srgb, var(--muted) 70%, var(--bg)); }
  footer a { color: inherit; }
  .grow { flex: 1; }
  .error { margin: 12px var(--gutter); padding: 10px 14px; border: 1px solid var(--bad); background: color-mix(in srgb, var(--bad) 10%, var(--card)); color: var(--bad); border-radius: var(--radius); }
  .banner { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; margin: 0; padding: 8px var(--gutter); background: color-mix(in srgb, var(--warn) 13%, var(--bg)); border-bottom: 1px solid color-mix(in srgb, var(--warn) 35%, var(--line)); font-size: var(--text-sm); }
  button:disabled { opacity: 0.5; cursor: default; }

  /* header controls (a.tab is also the banner button) */
  a.tab { color: var(--fg); background: var(--raised); text-decoration: none; border-radius: var(--radius); padding: 5px 12px; font-size: var(--text-label); text-transform: capitalize; }
  a.tab:hover { background: var(--tint-btn); color: var(--accent); }
  a.tab.on { background: var(--tint-on); color: var(--accent); font-weight: var(--weight-semibold); }
  h1 a, a.root { color: inherit; text-decoration: none; }
  a.root { color: var(--muted); max-width: 32ch; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; direction: rtl; text-align: left; }
  .badge { display: inline-block; min-width: 18px; margin-left: 6px; padding: 0 5px; border-radius: 2px; font-size: var(--text-sm); font-weight: var(--weight-semibold); line-height: 1.3; text-align: center; color: var(--card); background: var(--tone); }
  a.tab.on .badge { outline: 1px solid var(--raised); }
  .icon { display: inline-flex; align-items: center; position: relative; padding: 6px 9px; line-height: 1.2; }
  a.tab.icon { padding: 6px 9px; }
  a.tab.icon .badge { position: absolute; top: -4px; right: -4px; margin: 0; min-width: 14px; padding: 0 3px; font-size: .65rem; }
  .diffrow > td { max-width: 0; }
  .hidediff { position: sticky; top: 0; float: right; z-index: 1; }
  .chevron { display: inline-block; width: .45rem; height: .45rem; border: solid currentColor; border-width: 0 1.5px 1.5px 0; transform: rotate(45deg); }
  @media (max-width: 90rem) { a.root { flex: 0 1 auto; min-width: 0; } }
  @media (max-width: 48rem) {
    header { padding: 0 8px; }
    a.root { display: none; }
    .icon { padding: 4px 6px; }
  }

  /* report */
  .card { background: var(--card); border: 1px solid var(--line); border-radius: var(--radius); padding: 14px 16px; box-shadow: var(--shadow); }
  .hero { margin-bottom: 14px; }
  .pct { display: flex; align-items: baseline; gap: 12px; flex-wrap: wrap; }
  .pct b { font-size: var(--text-metric); font-weight: var(--weight-bold); line-height: 1.1; letter-spacing: -0.02em; }
  .metric-count { font-size: var(--text-sm); }
  .meter { height: 8px; margin: 10px 0 8px; border-radius: 4px; background: var(--raised); overflow: hidden; }
  .meter i { display: block; height: 100%; background: var(--accent); border-radius: 4px; }
  .facts { margin: 0; font-size: var(--text-sm); }
  tr.hidden-row { opacity: 0.6; }
  button.ignore { padding: 1px 8px; font-size: var(--text-sm); }
  .note-line { font-size: var(--text-sm); margin: 0 0 12px; }
  b[data-tone] { color: var(--tone); }
  .ids { display: flex; flex-wrap: wrap; gap: 6px; margin-bottom: 0; }
  .test-links { display: flex; flex-wrap: wrap; gap: 4px; }

  h3.file, .preview-section { margin: 14px 0 4px; font-size: var(--text-body); }
  h2 { margin: 18px 0 6px; font-size: var(--text-section); }
  h2.mono { font-size: var(--text-title); }
  details.plain { margin: 12px 0 0; padding: 6px 10px; background: var(--card); border: 1px solid var(--line); border-radius: var(--radius); }
  a.more { font-size: var(--text-sm); font-weight: 400; margin-left: 8px; }
  select { font: inherit; }


  /* requirements */
  .bar { display: flex; gap: var(--gap); align-items: center; margin-bottom: 14px; flex-wrap: wrap; }
  .bar input:not([type=checkbox]) { flex: 1; min-width: 200px; }
  .bar input[type=checkbox] { padding: 0; margin: 0; }
  .bar label { display: inline-flex; align-items: center; gap: 6px; white-space: nowrap; cursor: pointer; }

  .req-toolbar { flex-wrap: nowrap; gap: 8px; }
  .cycle-group { display: inline-block; margin-left: 0.75em; }
  .req-toolbar input:not([type=checkbox]) { min-width: 0; }
  .req-toolbar .found:empty { display: none; }
  .filter-toggle { display: inline-flex; align-items: center; gap: 8px; padding-inline: 8px; white-space: nowrap; }
  .req-toolbar :is(.found, .clear-filters, .filter-toggle) { font-size: var(--text-sm); }
  .req-options:not([hidden]) { display: flex; flex-wrap: wrap; align-content: start; align-items: center; gap: 8px 16px; overflow: auto; max-height: min(28dvh, 15rem); flex-shrink: 0; margin-bottom: 12px; padding: 8px 10px; background: var(--card); border: 1px solid var(--line); border-radius: var(--radius); }
  .req-options > div { min-width: 0; max-width: 100%; }
  .filter-chips { display: flex; flex-wrap: wrap; align-items: center; gap: 4px 6px; margin: -6px 0 10px; }
  .filter-chip { display: inline-flex; align-items: stretch; min-width: 0; max-width: 100%; border-radius: var(--badge-radius); color: var(--pill-ink, var(--fg)); background: var(--pill-fill, var(--tint-on)); }
  .filter-chip button { display: grid; place-items: center; min-height: 24px; min-width: 0; border: 0; background: transparent; color: inherit; font-size: var(--text-sm); line-height: 1.2; padding: 2px 4px 2px 8px; overflow-wrap: anywhere; }
  .filter-chip button:last-child { flex: 0 0 24px; padding: 0; font-size: 1rem; line-height: 1; }
  .filter-chip button:hover:not(:disabled) { background: color-mix(in srgb, currentColor 12%, transparent); color: inherit; }
  @media (max-width: 48rem) { .req-toolbar { flex-wrap: wrap; } .req-toolbar input:not([type=checkbox]) { flex-basis: 100%; } }
  /* list and preview are panes that scroll on their own: side by side (wide screens, default) or stacked (.h, and always on narrow screens) */
  aside.stale { opacity: .55; }
  .refs a, li a { text-decoration: none; }
  .refs a:hover, li a:hover { text-decoration: underline; }
  .page { display: flex; flex-direction: column; height: calc(100dvh - 100px); min-height: 440px; }
  .doc { flex: 1; min-width: 0; overflow: auto; background: var(--card); border: 1px solid var(--line); border-radius: var(--radius); padding: 10px 16px 14px; box-shadow: var(--shadow); }
  .docs { display: flex; flex: 1; min-height: 0; gap: var(--gap); }
  .tree { flex: 0 0 15rem; min-height: 0; padding: 10px; background: var(--card); border: 1px solid var(--line); border-radius: var(--radius); box-shadow: var(--shadow); }
  .filter-toggle[aria-pressed=true] { background: var(--tint-on); color: var(--accent); }
  @media (max-width: 62rem) { .docs { flex-direction: column; } .tree { flex-basis: auto; max-height: 12rem; } }
  .badges { margin-top: 3px; }
  .notes { margin-top: 3px; font-size: var(--text-sm); line-height: 1.6; }
  .note-link { all: unset; cursor: pointer; }
  .note-link:focus-visible { outline: var(--focus-ring); }
  .txt { display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
  /* a trace panel outside a split (overlap) is a floating card; inside one, Split draws the card */
  aside.float { position: sticky; top: 64px; background: var(--card); border: 1px solid var(--line); border-radius: var(--radius); padding: 10px 16px 14px; overflow: auto; max-height: calc(100vh - 80px); box-shadow: var(--shadow); }
  table.snip { margin-top: 4px; }
  .refs { margin: 0; padding-left: 16px; font-size: var(--text-sm); }
  .note { font-size: var(--text-sm); margin: 10px 0 0; }
  details.indirect { margin-top: 10px; }
  details.indirect summary { cursor: pointer; font-size: var(--text-sm); }
  pre { margin: 0; white-space: pre-wrap; word-break: break-word; }

  /* specs, code, setup */
  .crumb { margin: 0 0 8px; }
  table.code { display: block; max-width: 100%; overflow-x: auto; font-family: var(--mono); font-size: var(--text-sm); background: var(--card); border: 1px solid var(--line); border-radius: var(--radius); }
  table.code td { border: none; padding: 0 10px; white-space: pre; }
  td.ln { text-align: right; width: 1%; user-select: none; }
  td.ln a { color: var(--muted); }
  tr.hl { background: color-mix(in srgb, var(--warn) 22%, transparent); }
  .rv { margin: 0 6px 0 0; vertical-align: middle; }
  span.rv { display: inline-block; width: 13px; text-align: center; }
  td.src { overflow-x: auto; font-size: var(--text-label); }
  table.code.wrap td.src { white-space: pre-wrap; overflow-wrap: anywhere; }
  .crumb { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; }
  .crumb .small { font-size: var(--text-sm); padding: 2px 8px; }
  a.btn { background: var(--raised); color: var(--fg); border-radius: var(--radius); text-decoration: none; }
  a.btn:hover { background: var(--tint-btn); color: var(--accent); }
  /* highlight.js token colours (the markup comes from {@html}, hence :global) */
  :global(.hljs-comment, .hljs-quote) { color: var(--muted); font-style: italic; }
  :global(.hljs-keyword, .hljs-selector-tag, .hljs-literal, .hljs-built_in, .hljs-doctag) { color: var(--accent); }
  :global(.hljs-string, .hljs-regexp, .hljs-addition) { color: var(--ok); }
  :global(.hljs-number, .hljs-symbol, .hljs-bullet, .hljs-meta) { color: var(--warn); }
  :global(.hljs-title, .hljs-section, .hljs-attr, .hljs-attribute, .hljs-name) { color: var(--info); }
  :global(.hljs-type, .hljs-class .hljs-title) { color: var(--accent-2); }
  :global(.hljs-deletion) { color: var(--bad); }
  .block { margin-bottom: 10px; }
  .setup-page { display: grid; gap: var(--gap); max-width: 64rem; margin-inline: auto; }
  .setup { min-width: 0; overflow-wrap: anywhere; }
  .setup-head, .actions { display: flex; align-items: center; flex-wrap: wrap; gap: var(--gap); }
  .setup-head h2 { margin: 0; }
  /* Health: the verdict first, tinted by its tone (green when every check passed) */
  .health { border-color: color-mix(in srgb, var(--tone) 45%, var(--line)); background: color-mix(in srgb, var(--tone) 9%, var(--card)); }
  .health-head { display: flex; align-items: center; flex-wrap: wrap; gap: 14px; }
  .health-head h2 { margin: 0; font-size: var(--text-title); color: var(--tone); }
  .health-head .end { margin-inline-start: auto; }
  .health-ico { display: flex; font-size: 2rem; color: var(--tone); }
  .eyebrow { font-size: var(--text-sm); }
  .setup button { font-size: var(--text-label); }
  .setup strong { font-weight: var(--weight-semibold); }
  .setup-head .end { margin-inline-start: auto; }
  .setup p, .setup details { margin: 6px 0; }
  .setup summary { cursor: pointer; }
  .setup-item, .check-row { display: grid; align-items: baseline; gap: var(--gap); padding-block: 8px; border-top: 1px solid var(--line); }
  .setup-item { grid-template-columns: minmax(0, 1fr) auto; margin-top: 4px; }
  .setup-item code, .transport-note, .check-row .muted { color: var(--muted); font-size: var(--text-sm); }
  .check-row { grid-template-columns: auto minmax(0, 1fr); }
  .setup .log { overflow-x: auto; }
  .log { margin-top: 12px; padding: 10px 14px; background: var(--raised); border-radius: var(--radius); font-family: var(--mono); font-size: var(--text-sm); }
</style>

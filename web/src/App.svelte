<script lang="ts">
  import { untrack } from "svelte";
  import { call, embedded, onToolInput, read, watch } from "$lib/transport";
  import { parse } from "$lib/router";
  import { nav, navigate, DOCS as docs } from "$lib/nav.svelte";
  import { data, load } from "$lib/data.svelte";
  import { setupState, loadSetup } from "$lib/setup.svelte";
  import { filters, states, facets, facetShortcuts, clearReqFilters, selected, options, actionable, openFilterMenu } from "$lib/requirements/filterState.svelte";
  import { writeValues } from "$lib/requirements/filters";
  import { stored, save } from "$lib/store";
  import Header from "$lib/layout/Header.svelte";
  import Sidebar from "$lib/layout/Sidebar.svelte";
  import Settings from "./views/Settings.svelte";
  import Dashboard from "./views/Dashboard.svelte";
  import Documents from "./views/Documents.svelte";
  import Requirements from "./views/Requirements.svelte";
  import Code from "./views/Code.svelte";
  import Modules from "./views/Modules.svelte";
  import Tests from "./views/Tests.svelte";
  import Changes from "./views/Changes.svelte";
  import Overlap from "./views/Overlap.svelte";
  import Uncovered from "./views/Uncovered.svelte";

  const route = $derived(nav.route);
  const inDocs = $derived(nav.inDocs);

  // MCP Apps host: the `dashboard` tool's `view` ("requirements/DOC-25?state=none") picks the page
  onToolInput((a) => typeof a.view === "string" && navigate(parse(new URL(a.view.replace(/^\/?/, "/"), location.href))));

  $effect(() => {
    const onpop = () => nav.popped();
    // same-origin links navigate in place; modified clicks keep the browser's own behaviour (new tab, download)
    const onclick = (e: MouseEvent) => {
      const a = (e.target as HTMLElement).closest<HTMLAnchorElement>("a[href^='/']");
      if (!a || e.defaultPrevented || e.button !== 0 || e.metaKey || e.ctrlKey || e.shiftKey || e.altKey || (a.target && a.target !== "_self")) return;
      e.preventDefault();
      navigate(nav.route.view === "docs" && a.dataset.id ? { ...nav.route, id: a.dataset.id } : parse(new URL(a.href)));
    };
    window.addEventListener("popstate", onpop);
    document.addEventListener("click", onclick);
    return () => { window.removeEventListener("popstate", onpop); document.removeEventListener("click", onclick); };
  });

  $effect(() => {
    document.title = ["spec17", route.tab, route.id ?? route.file ?? route.path].filter(Boolean).join(" · ");
  });

  // setup banner: hidden per browser until its message changes
  let dismissed = $state(stored("spec17.banner"));
  const dismiss = (m: string) => { dismissed = m; save("spec17.banner", m); };
  const setup = $derived(setupState.setup);
  let rail = $state(false);

  // keyboard: `/` focuses the filter (opening the requirements list first), filter shortcuts, Esc closes the trace panel (↑/↓ on the lists: DataTable)
  $effect(() => {
    const onkey = (e: KeyboardEvent) => {
      const route = nav.route;
      const onReqs = route.tab === "requirements" || route.tab === "changes";
      const t = e.target as HTMLElement;
      // an open popover (the filter menu, a tooltip) takes Escape; nothing behind it closes
      if (e.key === "Escape" && document.querySelector(":popover-open")) return;
      if (t.closest("input, select, textarea, .gutter, .seg") || e.metaKey || e.ctrlKey || e.altKey) { if (e.key === "Escape") (t as HTMLElement).blur(); return; }
      if (e.key === "/") {
        e.preventDefault();
        if (nav.inDocs || !onReqs) navigate({ tab: "requirements" });
        queueMicrotask(() => document.querySelector<HTMLInputElement>("input[name=filter]")?.focus());
      } else if (onReqs && e.key.toLowerCase() === "f" && filters.hasFilters) {
        e.preventDefault(); openFilterMenu();
      } else if (onReqs && document.querySelector(".req-toolbar .filter-toggle") && /^[0-7slpecr]$/i.test(e.key)) {
        const k = e.key.toLowerCase();
        const field = facets.find((key) => facetShortcuts[key].toLowerCase() === k);
        if (k === "0") navigate({ ...route, ...clearReqFilters });
        else if (field) {
          const choices = options(field);
          if (!actionable(field, choices)) return;
          const values = selected(field);
          const index = values.length ? choices.findIndex((choice) => choice.key === JSON.stringify(values[0])) : 0;
          const next = choices[(index + (e.shiftKey ? -1 : 1) + choices.length) % choices.length];
          navigate({ ...route, [field]: next.key ? writeValues([JSON.parse(next.key)]) : undefined });
        }
        else if (filters.stateKeys.includes(states[+k - 1])) navigate({ ...route, state: states[+k - 1] === "all" ? undefined : states[+k - 1] });
      } else if (e.key === "Escape" && route.id && (route.tab === "requirements" || route.tab === "overlap")) navigate({ ...route, id: undefined });
      else if (e.key === "Escape" && route.path && route.tab === "tests") navigate({ ...route, path: undefined, line: undefined });
    };
    document.addEventListener("keydown", onkey);
    return () => document.removeEventListener("keydown", onkey);
  });

  // each view loads what its URL names
  $effect(() => {
    const id = route.id;
    void data.rows; // a refresh replaces the rows and must reload the open trace
    if (!id || inDocs) { data.trace = null; return; }
    // the previous trace stays (dimmed) until the new one lands, so the preview pane never collapses and reopens
    call("trace", { id }).then((r) => { if (!nav.inDocs && nav.route.id === id) { data.trace = r; document.querySelector(".split .preview")?.scrollTo(0, 0); } }).catch((e) => { data.trace = null; data.error = e.message; });
  });

  // each selection takes a token (also when the file is already loaded), so a slower earlier response cannot replace it
  let docReq = 0;
  $effect(() => {
    if (!inDocs) return;
    if (data.specFiles === null) { call("file").then((r) => (data.specFiles = r.data.files)).catch((e) => (data.error = e.message)); return; }
    const f = data.docFile, n = ++docReq;
    if (f && f !== untrack(() => data.specLoaded)) call("file", { name: f }).then((r) => { if (n !== docReq) return; data.specLoaded = f; data.specMd = r.data.markdown; document.querySelector(".doc")?.scrollTo(0, 0); }).catch((e) => { if (n === docReq) data.error = e.message; });
  });

  $effect(() => {
    const path = route.tab === "code" || route.tab === "tests" ? route.path : undefined;
    if (path && untrack(() => data.code?.path) !== path) call("source", { path }).then((r) => (data.code = r.data)).catch((e) => { data.code = null; data.error = e.message; });
  });

  // fetched once per load generation, when its tab is open
  const fetched: Record<string, number> = {};
  const fetchOnce = (key: string, get: () => Promise<void>) => {
    if (fetched[key] === data.gen) return;
    fetched[key] = data.gen;
    get().catch((e) => { fetched[key] = -1; data.error = e.message; });
  };
  $effect(() => {
    if (route.tab === "overlap") fetchOnce("overlap", () => call("overlap").then((r) => void (data.overlap = r.data.rows)));
    if (route.tab === "tests" || route.tab === "requirements" || route.tab === "changes" || (route.tab === "modules" && route.module)) fetchOnce("tests", () => read("spec17://tests").then((r) => void (data.tests = r)));
    if (route.tab === "uncovered") fetchOnce("uncovered", () => call("uncovered").then((r) => void (data.uncovered = r.data)));
  });

  $effect(() => {
    if (route.tab !== "changes") return;
    void data.gen;
    const scope = data.scope;
    call("history", { ignore_whitespace: scope.ignore_whitespace, show_ignored: scope.show_ignored, show_all: scope.show_all }).then((r) => (data.history = r.data)).catch(() => (data.history = null));
    if (data.defaultScope) return;
    call("changes", scope).then((r) => (data.custom = r.data)).catch(() => (data.custom = null));
  });

  // scroll to what the URL points at once it is rendered (the lists scroll their own selection: DataTable)
  $effect(() => {
    void [data.specMd, data.code, route.id, route.line];
    queueMicrotask(() => {
      const el = nav.route.tab === "code" ? document.getElementById(`L${nav.route.line}`) : nav.inDocs && nav.route.id ? document.querySelector(`article a[data-id="${nav.route.id}"]`) : null;
      el?.scrollIntoView({ block: "center" });
    });
  });

  // (WEB-27) the first run lands on Setup: nothing to scan and nothing installed, or a failed check; never inside a host that picks the view
  Promise.allSettled([load(), loadSetup()]).then(() => {
    const first = data.report && data.report.requirements + data.report.refs === 0 && setupState.setup?.suggestions.length;
    if (!embedded && nav.route.tab === "dashboard" && (first || setupState.checks.some((k) => k.level === "fail"))) navigate({ tab: "settings" }, true);
  });
  // the server pushes `resources/updated` when the repo changes; one notification per subscribed resource arrives for one change
  let reloadTimer: ReturnType<typeof setTimeout> | undefined;
  // throttled: the first push reloads at once, a burst after it collapses into one trailing reload at most every 2 s
  let lastLoad = 0;
  $effect(() => watch(["spec17://report", "spec17://status", "spec17://changes"], () => {
    clearTimeout(reloadTimer);
    reloadTimer = setTimeout(() => { lastLoad = Date.now(); load().catch(() => {}); }, Math.max(0, lastLoad + 2000 - Date.now()));
  }, (s) => (data.up = s)));
  // fallback when no push arrives (embedded host, a dropped stream, a proxy that buffers): every 30 s, reload only if the report changed
  $effect(() => {
    const t = setInterval(async () => {
      if (document.hidden) return;
      try { if (JSON.stringify((await call("report", { limit: 1000 })).data) !== JSON.stringify(untrack(() => data.report))) await load(); data.up = true; } catch { data.up = false; /* the next tick retries */ }
    }, 30_000);
    return () => clearInterval(t);
  });
</script>

<Header />

<div class="shell" class:rail>
<Sidebar bind:collapsed={rail} />
<div class="content">
{#if data.error}<p class="error">{data.error}</p>{/if}
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
    <Settings />
  {:else if !data.report}
    {#if data.error}<p class="muted">The scan failed (see above). <a href="/settings">Settings</a> shows what is missing.</p>{:else}<p class="muted">Scanning…</p>{/if}
  {:else if route.tab === "dashboard"}
    <Dashboard />
  {:else if inDocs}
    <Documents />
  {:else if route.tab === "requirements"}
    <Requirements />
  {:else if route.tab === "code"}
    <Code />
  {:else if route.tab === "modules"}
    <Modules />
  {:else if route.tab === "tests"}
    <Tests />
  {:else if route.tab === "changes"}
    <Changes />
  {:else if route.tab === "overlap"}
    <Overlap />
  {:else if route.tab === "uncovered"}
    <Uncovered />
  {/if}
</main>

<footer class="muted">
  Evidence, not truth: tags and citations do not prove a requirement is implemented or its test passes.
  <div>Copyright &copy; 2026 Konstantin Pavlov, <a href="https://itstaff.ee">Staff OÜ</a>. All rights reserved. · <a href={docs} target="_blank" rel="noopener noreferrer">Documentation</a> · <a href="https://github.com/kpavlov/spec17/issues" target="_blank" rel="noopener noreferrer">Feedback</a></div>
</footer>
</div>
</div>

<style>
  .shell { display: grid; grid-template-columns: 12.5rem minmax(0, 1fr); min-height: calc(100dvh - var(--header-h)); }
  .shell.rail { grid-template-columns: 3.5rem minmax(0, 1fr); }
  /* the footer sits at the bottom of a short page and below a long one */
  .content { min-width: 0; display: flex; flex-direction: column; }
  .content > main { flex: 1; }
  main { padding: var(--gutter); max-width: 100%; overflow-x: clip; }
  footer { padding: 14px 16px; text-align: center; font-size: var(--text-meta); line-height: var(--leading-tight); color: var(--muted); }
  footer a { color: inherit; }
  .error { margin: 12px var(--gutter); padding: 10px 14px; border: 1px solid var(--bad); background: color-mix(in srgb, var(--bad) 10%, var(--card)); color: var(--bad); border-radius: var(--radius); }
  .banner { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; margin: 0; padding: 8px var(--gutter); background: color-mix(in srgb, var(--warn) 13%, var(--bg)); border-bottom: 1px solid color-mix(in srgb, var(--warn) 35%, var(--line)); font-size: var(--text-sm); }
  @media (max-width: 48rem) { .shell, .shell.rail { grid-template-columns: minmax(0, 1fr); grid-template-rows: auto 1fr; } }
</style>

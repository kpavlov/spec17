<script module lang="ts">
  import type { Sort } from "$lib/sort";
  // the sort survives a tab switch
  const modSort = $state<Sort>({ key: "", dir: 1 });
</script>

<script lang="ts">
  import Graph from "$lib/views/Graph.svelte";
  import Filters from "$lib/ui/Filters.svelte";
  import Seg from "$lib/ui/Seg.svelte";
  import Pill from "$lib/ui/Pill.svelte";
  import IdLink from "$lib/ui/IdLink.svelte";
  import ReqRef from "$lib/requirements/ReqRef.svelte";
  import { sorted, toggleSort } from "$lib/sort";
  import { stored, save, plural } from "$lib/store";
  import { call } from "$lib/transport";
  import { nav, navigate, u, req, filter } from "$lib/nav.svelte";
  import { data, load, guard } from "$lib/data.svelte";
  import { filters, changedSeg } from "$lib/requirements/filterState.svelte";

  const route = $derived(nav.route);
  const modules = $derived(data.modules);
  const tests = $derived(data.tests);
  const changes = $derived(data.changes);
  const known = $derived(data.known);
  const changedMods = $derived(data.changedMods);
  const query = $derived(filters.query);
  const onlyChanged = $derived(filters.onlyChanged);
  const showChanged = $derived(!!changes?.files.length || onlyChanged);
  const pickChanged = (k: string) => filter({ changed: k === "changed" ? "1" : undefined });
  const VIEWS = [{ key: "list", label: "list" }, { key: "graph", label: "graph" }];
  let showHidden = $state(stored("spec17.hidden") === "1");
  let modView = $state(stored("spec17.modules") === "graph" ? "graph" : "list");

  // a module's area is its symbol count; a green ring says its tests were detected
  const moduleGraph = $derived((modules?.modules ?? []).map((m: any) => ({ id: m.path as string, size: (Object.values(m.symbols) as number[]).reduce((a, b) => a + b, 0), ring: m.tests > 0, links: m.depends_on.map((d: any) => ({ to: d.module as string, weight: d.edges as number })), data: m })));
  const moduleTip = (n: { id: string; size: number; deg: number; data: any }) => ({ title: n.id, detail: `${n.size} symbols · ${n.data.tests} tests detected · ${n.deg} links` });
  const symbolsOf = (m: any) => Object.values<number>(m.symbols).reduce((a, b) => a + b, 0);
  // the modules table sorts here; the requirement, test and spec lists sort inside DataTable
  const modVal = (m: any, k: string) => (k === "languages" ? m.languages.join(", ") : k === "symbols" ? symbolsOf(m) : k === "depends" ? m.depends_on.length : (m[k] ?? ""));
  const mods = $derived<any[]>(sorted((modules?.modules ?? []).filter((m: any) => (!onlyChanged || changedMods.has(m.path)) && (!query || m.path.toLowerCase().includes(query.toLowerCase()))), modSort, modVal));
  const hiddenMods = $derived<string[]>((modules?.hidden ?? []).filter((m: string) => !query || m.toLowerCase().includes(query.toLowerCase())));
  const modOf = $derived(modules?.modules.find((m: any) => m.path === route.module));
  const usedBy = $derived((modules?.modules ?? []).filter((m: any) => modOf && m.depends_on.some((d: any) => d.module === modOf.path)));
  const testsOf = (path: string) => tests?.modules.find((m: any) => m.module === path);

  // (WEB-10, WEB-11) hide a module (after a confirm) or show it again via .spec17ignore, then rescan
  async function setHidden(path: string, hidden: boolean) {
    if (hidden && !confirm(`Add /${path}/ to .spec17ignore? spec17 will leave the module out of every scan.`)) return;
    try {
      await guard(call("module", { path, hidden }));
      await load();
    } catch { /* guard shows it */ }
  }
</script>

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
        <li><a class="mono" href={u({ tab: "code", path: f.file, line: t.line })} title={t.qualified}>{t.name}</a> {#each t.ids as id}<ReqRef {id} to={known.has(id) ? req(id) : { tab: "requirements", q: id }} />{" "}{/each}{#if !t.ids.length}<span class="muted">untagged</span>{/if}</li>
      {/each}
    </ul>
  {/each}
{/snippet}

{#if route.module}
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
{:else}
  <div class="bar">
    <input name="filter" aria-label="filter modules" placeholder="Filter modules" value={query} oninput={(e) => filter({ q: e.currentTarget.value })} />
    <Filters active={Number(onlyChanged) + Number(showHidden && modView === "list")}>
      {#if showChanged}<Seg label="Changes" options={changedSeg(changedMods.size)} value={onlyChanged ? "changed" : "all"} onpick={pickChanged} />{/if}
      {#if modView === "list" && modules?.hidden?.length}
        <label class="muted nowrap"><input type="checkbox" checked={showHidden} onchange={(e) => save("spec17.hidden", (showHidden = e.currentTarget.checked) ? "1" : "0")} /> show hidden ({modules.hidden.length})</label>
      {/if}
    </Filters>
    <span class="muted">{modules?.modules.length ?? 0} modules</span>
    <Seg label="View" nowrap options={VIEWS} value={modView} onpick={(k) => save("spec17.modules", (modView = k))} />
  </div>
  {#if modules && !modules.index}<p class="note-line muted">{modules.note}</p>{/if}
  {#if modView === "graph"}
    {#if !modules}<p class="muted">Loading…</p>
    {:else if !modules.index}<!-- the note above says why there is no graph -->
    {:else if !modules.modules.some((m: any) => m.depends_on.length)}<p class="muted empty">No dependencies between modules in the index.</p>
    {:else}<Graph items={moduleGraph} label="module dependency graph" legend="Arrow: the module uses the one it points to (width: edge count). Area: symbols. Colour: folder. Green ring: tests detected (not run)." tip={moduleTip} q={query} open={(p) => navigate({ tab: "modules", module: p })} />{/if}
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
{/if}

<style>
  tr.hidden-row { opacity: 0.6; }
  button.ignore { padding: 1px 8px; font-size: var(--text-sm); }
  a.more { font-size: var(--text-sm); font-weight: 400; margin-left: 8px; }
  h3.file { margin: 14px 0 4px; font-size: var(--text-body); }
</style>

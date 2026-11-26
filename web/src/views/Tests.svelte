<script lang="ts">
  import FilterMenu from "$lib/requirements/FilterMenu.svelte";
  import Seg from "$lib/ui/Seg.svelte";
  import Pill from "$lib/ui/Pill.svelte";
  import DataTable from "$lib/ui/DataTable.svelte";
  import Split from "$lib/layout/Split.svelte";
  import SplitToggle from "$lib/layout/SplitToggle.svelte";
  import ReqLink from "$lib/requirements/ReqLink.svelte";
  import TestPanel from "$lib/views/TestPanel.svelte";
  import { changeTone } from "$lib/tone";
  import { nav, navigate, u, req, filter } from "$lib/nav.svelte";
  import { data, changeMark } from "$lib/data.svelte";
  import { filters, changedSeg, openFilterMenu } from "$lib/requirements/filterState.svelte";

  const route = $derived(nav.route);
  const tests = $derived(data.tests);
  const modules = $derived(data.modules);
  const changes = $derived(data.changes);
  const known = $derived(data.known);
  const fileOf = $derived(data.fileOf);
  const stateFilter = $derived(filters.stateFilter);
  const query = $derived(filters.query);
  const onlyChanged = $derived(filters.onlyChanged);
  const showChanged = $derived(!!changes?.files.length || onlyChanged);
  const pickChanged = (k: string) => filter({ changed: k === "changed" ? "1" : undefined });
  const testOpts = $derived(["all", "tagged", "untagged"].map((k) => ({ key: k, label: k, count: tcounts[k] })));
  // (WEB-62) one chip per active filter
  const chips = $derived([
    ...(stateFilter !== "all" ? [{ label: stateFilter, remove: () => filter({ state: "all" }) }] : []),
    ...(onlyChanged ? [{ label: "changed", remove: () => filter({ changed: undefined }) }] : []),
    ...(route.module ? [{ label: route.module, remove: () => filter({ module: undefined }) }] : []),
  ]);
  const activeTestFilters = $derived(chips.length);
  const testRequirement = $derived(route.requirementId?.toUpperCase());
  const allCases = $derived(data.detectedCases.filter((c) => (!route.module || c.module === route.module) && (!testRequirement || c.t.ids.includes(testRequirement))));
  const tcounts = $derived({ all: allCases.length, tagged: allCases.filter((c) => c.t.ids.length).length, untagged: allCases.filter((c) => !c.t.ids.length).length } as Record<string, number>);
  const matched = $derived(allCases.filter((c) => (stateFilter === "all" || (stateFilter === "tagged") === c.t.ids.length > 0) && (!onlyChanged || fileOf.has(c.file)) && (!query || `${c.file} ${c.t.name} ${c.t.ids.join(" ")}`.toLowerCase().includes(query.toLowerCase()))));
  // list rows: a test is told apart by file and line, which is also how the URL selects it (`path` + `line`)
  const testRows = $derived(matched.map((c) => ({ ...c, key: `${c.file}:${c.t.line}` })));
  const testCols = [{ key: "name", label: "test", cls: "wrap" }, { key: "ids", label: "requirements", cls: "test-ids" }, { key: "file", label: "location", cls: "mono muted wrap" }];
  const testVal = (c: any, k: string) => (k === "file" ? `${c.file}:${String(c.t.line).padStart(6, "0")}` : k === "ids" ? c.t.ids.join(" ") : c.t.name);
  const selTest = $derived(route.tab === "tests" && route.path ? `${route.path}:${route.line}` : undefined);
  const taggedN = $derived(tests ? tests.modules.reduce((n: number, m: any) => n + m.files.reduce((k: number, f: any) => k + f.tests.filter((t: any) => t.ids.length).length, 0), 0) : 0);
</script>

<div class="page">
<!-- (WEB-62) -->
<div class="bar req-toolbar">
  <input name="filter" aria-label="filter tests" placeholder="Filter test, file, requirement ID   ( / )" value={query} oninput={(e) => filter({ q: e.currentTarget.value })} />
  <FilterMenu active={activeTestFilters} title="Filter tests">
    <Seg label="Tagged" options={testOpts} value={stateFilter} onpick={(k) => filter({ state: k })} />
    {#if showChanged}<Seg label="Changes" options={changedSeg(allCases.filter((c) => fileOf.has(c.file)).length)} value={onlyChanged ? "changed" : "all"} onpick={pickChanged} />{/if}
    <select aria-label="module" title={route.module} value={route.module ?? ""} onchange={(e) => filter({ module: e.currentTarget.value || undefined })}>
      <option value="">all modules</option>
      {#each modules?.modules ?? [] as m}<option value={m.path}>{m.path}</option>{/each}
    </select>
  </FilterMenu>
  <span class="found muted nowrap" aria-live="polite">{testRows.length} of {allCases.length}</span>
  {#if activeTestFilters || query}<a class="clear-filters" href={u({ ...route, state: undefined, module: undefined, changed: undefined, q: undefined })} aria-label="Clear test filters">Clear</a>{/if}
  {#if testRequirement}
    <a href={u({ ...route, requirementId: undefined })} aria-label="Clear requirement filter {testRequirement}">{testRequirement} ×</a>
  {/if}
  {#if route.path}<SplitToggle />{/if}
</div>
{#if chips.length}
  <div class="filter-chips" role="group" aria-label="Active test filters">
    {#each chips as c (c.label)}
      <span class="filter-chip">
        <button type="button" title="Edit filter" aria-label="Edit filter, {c.label}" onclick={() => openFilterMenu()}>{c.label}</button>
        <button type="button" title="Remove {c.label}" aria-label="Remove filter {c.label}" onclick={() => { c.remove(); document.querySelector<HTMLButtonElement>(".req-toolbar .filter-toggle")?.focus(); }}>×</button>
      </span>
    {/each}
  </div>
{/if}
{#if tests === null}<p class="muted">Loading…</p>
{:else}
  <p class="muted found">{tests.total} tests detected in {tests.modules.length} modules, {taggedN} tagged with a requirement. {tests.note}</p>
  <Split showPreview={!!route.path}>
    {#snippet list()}
      <DataTable rows={testRows} columns={testCols} rowKey={(c) => c.key} selected={selTest} onselect={(k, how) => { const c = testRows.find((x) => x.key === k); navigate({ ...route, path: c?.file, line: c?.t.line }, how === "key"); }} value={testVal} groupOf={(c) => c.module} storageKey="spec17.collapsed.tests">
        {#snippet cell(c, k)}
          {#if k === "name"}<span class="mono" title={c.t.qualified}>{c.t.name}</span>{#if changeMark(c.file)}{" "}<Pill tone={changeTone(changeMark(c.file))} title="its file is {changeMark(c.file)} in the working tree">✎ {changeMark(c.file)}</Pill>{/if}
          {:else if k === "file"}<a href={u({ tab: "code", path: c.file, line: c.t.line })}>{c.file}:{c.t.line}</a>
          {:else}<div class="test-links">{#each c.t.ids as id}<ReqLink {id} to={known.has(id) ? req(id) : { tab: "requirements", q: id }} />{/each}{#if !c.t.ids.length}<span class="muted">untagged</span>{/if}</div>{/if}
        {/snippet}
        {#snippet empty()}<p class="muted empty">No test matches. <a href="/tests">Clear filters</a></p>{/snippet}
      </DataTable>
    {/snippet}
    {#snippet preview()}<TestPanel {allCases} />{/snippet}
  </Split>
{/if}
</div>

<style>
  .test-links { display: flex; flex-wrap: wrap; gap: 4px; }
</style>

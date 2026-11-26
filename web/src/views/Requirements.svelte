<script lang="ts">
  import Graph from "$lib/views/Graph.svelte";
  import Split from "$lib/layout/Split.svelte";
  import DataTable from "$lib/ui/DataTable.svelte";
  import ViewSeg from "$lib/requirements/ViewSeg.svelte";
  import Toolbar from "$lib/requirements/Toolbar.svelte";
  import ReqCell from "$lib/requirements/ReqCell.svelte";
  import TracePanel from "$lib/views/TracePanel.svelte";
  import { cycles } from "$lib/requirements/cycles";
  import { idPrefix } from "$lib/requirements/layout";
  import { reqCols, reqVal } from "$lib/requirements/columns";
  import { edgeTone } from "$lib/tone";
  import { plain } from "$lib/store";
  import { nav, navigate, u, req, filter, DOCS as docs } from "$lib/nav.svelte";
  import { data } from "$lib/data.svelte";
  import { filters } from "$lib/requirements/filterState.svelte";

  const route = $derived(nav.route);
  const rows = $derived(data.rows);
  const noReqs = $derived(data.noReqs);
  const query = $derived(filters.query);
  const shown = $derived(filters.shown);
  const requirementCycles = $derived(cycles(rows));
  const requirementGraph = $derived(rows.map((r) => ({ id: r.id, size: 1, links: r.references.map((to) => ({ to, weight: 1 })), data: plain(r.text) })));
  const requirementTip = (n: { id: string; dependents: number; data: string }) => ({ title: n.id, meta: n.dependents ? `${n.dependents} ${n.dependents === 1 ? "dependent" : "dependents"}` : undefined, detail: n.data });
  const prefixOf = (id: string) => id.replace(/-.*$/, "");
</script>

{#snippet reqCell(r: any, k: string)}<ReqCell {r} {k} />{/snippet}

{#if noReqs}
  <p class="muted empty">No requirements found. <a href="/settings">Settings</a> says what spec17 looks for; <a href="{docs}/specs/#requirement-styles" target="_blank" rel="noopener noreferrer">the docs</a> show the formats.</p>
{:else}
  <div class="page">
  {#if route.view === "graph"}
    <div class="bar req-toolbar"><input name="filter" aria-label="Filter requirements" placeholder="Filter requirements" value={query} oninput={(e) => filter({ q: e.currentTarget.value })} /><ViewSeg /></div>
    <p class="muted">References are detected from requirement text and may not mean a dependency.</p>
    {#if requirementCycles.length}
      <p><b>Circular reference groups:</b> {#each requirementCycles as group}<span class="cycle-group">{"{"}{#each group as id, i}{#if i}{", "}{/if}<a href={u(req(id))}>{id}</a>{/each}{"}"}</span>{/each}</p>
    {:else}<p class="muted">No circular references detected.</p>{/if}
    <Graph items={requirementGraph} label="requirement reference graph" legend="Arrow: the requirement references the one it points to. Highlighted arrows and rings mark circular references. Colour: ID prefix." tip={requirementTip} group={idPrefix} q={query} cycleGroups={requirementCycles} open={(id) => navigate(req(id))} />
  {:else}
  <Toolbar />
  <Split showPreview={!!route.id}>
    {#snippet list()}
      <div class="req-list">
      <DataTable rows={shown} columns={reqCols} cell={reqCell} rowKey={(r) => r.id} selected={route.id} onselect={(k, how) => navigate({ ...route, id: k }, how === "key")} value={reqVal} groupOf={(r) => prefixOf(r.id)} tone={(r) => edgeTone(r.state)} storageKey="spec17.collapsed">
        {#snippet empty()}<p class="muted empty">No requirement matches. <a href={u({ tab: "requirements" })}>Clear filters</a></p>{/snippet}
      </DataTable>
      </div>
    {/snippet}
    {#snippet preview()}<TracePanel />{/snippet}
  </Split>
  {/if}
  </div>
{/if}

<script lang="ts">
  import { untrack } from "svelte";
  import PaneHead from "$lib/ui/PaneHead.svelte";
  import Who from "$lib/ui/Who.svelte";
  import ReqRef from "$lib/requirements/ReqRef.svelte";
  import ViewSwitch from "$lib/layout/ViewSwitch.svelte";
  import DiffView from "$lib/views/DiffView.svelte";
  import { preview } from "$lib/layout/layout.svelte";
  import { nav, u, req } from "$lib/nav.svelte";
  import { data, blameKey, wantBlame } from "$lib/data.svelte";
  import { codeLines } from "$lib/lines";
  import { plain } from "$lib/store";

  // the preview of the selected test: the lines of it, a few before it and the start of its body
  let { allCases }: { allCases: any[] } = $props();
  const TEST_BEFORE = 3, TEST_AFTER = 24;
  const route = $derived(nav.route);
  const known = $derived(data.known);
  const byId = $derived(data.byId);
  const blame = $derived(data.blame);
  const fileOf = $derived(data.fileOf);
  const gen = $derived(data.gen);
  const code = $derived(data.code);
  const lines = $derived(codeLines(data.code));
  const diffOn = (changed: boolean) => changed && preview.view === "diff";
  $effect(() => { void data.gen; if (nav.route.tab === "tests" && nav.route.path && nav.route.line) untrack(() => wantBlame([{ file: nav.route.path!, line: nav.route.line! }])); });
</script>

{#if route.path}
  {@const c = allCases.find((x) => x.file === route.path && x.t.line === route.line)}
  {@const lo = Math.max(1, (route.line ?? 1) - TEST_BEFORE)}
  <PaneHead title={c?.t.name ?? route.path} close={u({ ...route, path: undefined, line: undefined })} />
  <p class="muted mono nowrap"><a href={u({ tab: "code", path: route.path, line: route.line })}>{route.path}:{route.line}</a>{#if c}{" "}in <a href={u({ tab: "modules", module: c.module })}>{c.module}</a>{/if}</p>
  <Who commit={route.path && route.line ? blame[blameKey(route.path, route.line)] : undefined} note />
  {#if fileOf.has(route.path)}<p><ViewSwitch /></p>{/if}
  {#if diffOn(fileOf.has(route.path))}
    <DiffView paths={[route.path]} {gen} />
  {:else}
  {#if c}
    <h3 class="preview-section">Requirements</h3>
    {#if c.t.ids.length}
      <ul class="refs">{#each c.t.ids as id}<li><ReqRef {id} to={known.has(id) ? req(id) : { tab: "requirements", q: id }} /> <span class="muted">{byId.get(id) ? plain(byId.get(id)!.text).slice(0, 140) : "not in any spec"}</span></li>{/each}</ul>
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

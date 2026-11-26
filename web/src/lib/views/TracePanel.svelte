<script lang="ts">
  import { untrack } from "svelte";
  import PaneHead from "$lib/ui/PaneHead.svelte";
  import Markdown from "$lib/ui/Markdown.svelte";
  import Who from "$lib/ui/Who.svelte";
  import RequirementBadges from "$lib/requirements/RequirementBadges.svelte";
  import ViewSwitch from "$lib/layout/ViewSwitch.svelte";
  import DiffView from "$lib/views/DiffView.svelte";
  import { preview } from "$lib/layout/layout.svelte";
  import { nav, u, doc } from "$lib/nav.svelte";
  import { data, blameKey, wantBlame } from "$lib/data.svelte";

  // the trace of the selected requirement; `float` is the card beside the Overlap page, outside a split
  let { float = false }: { float?: boolean } = $props();
  const route = $derived(nav.route);
  const trace = $derived(data.trace);
  const byId = $derived(data.byId);
  const commits = $derived(data.commits);
  const blame = $derived(data.blame);
  const gen = $derived(data.gen);
  // (WEB-25) the preview of a changed item shows the diff when the remembered view is "diff"
  const diffOn = (changed: boolean) => changed && preview.view === "diff";
  // the changed files behind a requirement: its spec file when the bullet was edited, and its tests and citing sources that changed
  const reqFiles = (t: any, kinds: string[] = []) => [...new Set<string>([...(kinds.includes("spec") && t.spec ? [t.spec.file] : []), ...(t.tests ?? []).map((x: any) => x.file), ...(t.source_cites ?? []).map((x: any) => x.file)].filter((f) => data.fileOf.has(f)))];
  const reqKinds = $derived(data.reqKinds);
  $effect(() => { void data.gen; const t = data.trace?.data?.tests; if (t?.length) untrack(() => wantBlame(t)); });
</script>

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
      <Who commit={commits[trace.data.id]} note />
      <Markdown source={sp.text} />
    {:else}<p class="muted">Not in any spec: cited only in tests or code.</p>{/if}
    {#if trace.data.tests?.length}
      <h3 class="preview-section">Tests ({trace.data.tests.length})</h3>
      <ul class="refs">{#each trace.data.tests as t}<li><a class="mono nowrap" href={u({ tab: "code", path: t.file, line: t.line })}>{t.file}:{t.line}</a> {t.text} <Who commit={blame[blameKey(t.file, t.line)]} /></li>{/each}</ul>
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

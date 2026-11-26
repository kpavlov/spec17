<script lang="ts">
  import Seg from "$lib/ui/Seg.svelte";
  import FileTree from "$lib/ui/FileTree.svelte";
  import Markdown from "$lib/ui/Markdown.svelte";
  import ViewSwitch from "$lib/layout/ViewSwitch.svelte";
  import ViewSeg from "$lib/requirements/ViewSeg.svelte";
  import DiffView from "$lib/views/DiffView.svelte";
  import { preview } from "$lib/layout/layout.svelte";
  import { edgeTone } from "$lib/tone";
  import { stored, save } from "$lib/store";
  import { nav, u, doc, filter, DOCS as docs } from "$lib/nav.svelte";
  import { data, changeMark } from "$lib/data.svelte";
  import { filters, changedSeg } from "$lib/requirements/filterState.svelte";

  const route = $derived(nav.route);
  const specFiles = $derived(data.specFiles);
  const specMd = $derived(data.specMd);
  const docFile = $derived(data.docFile);
  const fileOf = $derived(data.fileOf);
  const byId = $derived(data.byId);
  const known = $derived(data.known);
  const changes = $derived(data.changes);
  const gen = $derived(data.gen);
  const onlyChanged = $derived(filters.onlyChanged);
  const showChanged = $derived(!!changes?.files.length || onlyChanged);
  const pickChanged = (k: string) => filter({ changed: k === "changed" ? "1" : undefined });
  const docFiles = $derived((specFiles ?? []).filter((file) => !onlyChanged || fileOf.has(file) || file === docFile));
  const diffOn = (changed: boolean) => changed && preview.view === "diff";
  let treeOpen = $state(stored("spec17.tree") !== "0");
  const toggleTree = () => { treeOpen = !treeOpen; save("spec17.tree", treeOpen ? "1" : "0"); };
</script>

{#if specFiles === null}<p class="muted">Loading…</p>
{:else if !specFiles.length}<p class="muted empty">No spec files found. <a href="/settings">Settings</a> says what spec17 looks for; <a href="{docs}/specs/#requirement-styles" target="_blank" rel="noopener noreferrer">the docs</a> show the formats.</p>
{:else}
  {@const f = docFile!}
  <div class="page">
    <div class="bar">
      <ViewSeg />
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

<script lang="ts">
  import RequirementBadges from "$lib/requirements/RequirementBadges.svelte";
  import Pill from "$lib/ui/Pill.svelte";
  import { levelPills } from "$lib/requirements/levelPills";
  import { inline } from "$lib/markdown";
  import { verdictMark } from "$lib/tone";
  import HelpPill from "$lib/ui/HelpPill.svelte";
  import { plain } from "$lib/store";
  import { nav, u, doc, req } from "$lib/nav.svelte";
  import { data, isReviewed, reviewTip } from "$lib/data.svelte";
  import { setReviewed } from "$lib/reviewed.svelte";
  import Reasons from "$lib/requirements/Reasons.svelte";
  import FilePath from "$lib/ui/FilePath.svelte";

  // one cell of the requirement table (Requirements and Changes); `showDiff` opens a file's diff on the Changes page
  let { r, k, onChanges = false, showDiff = () => {} }: { r: any; k: string; onChanges?: boolean; showDiff?: (path: string) => void } = $props();
  const route = $derived(nav.route);
  const commits = $derived(data.commits);
  const known = $derived(data.known);
  const testedIds = $derived(data.testedIds);
  const changeOf = $derived(data.changeOf);
  const changes = $derived(data.changes);
  const view = $derived(data.view);
  const scope = $derived(data.scope);
  // one pill per kind of note, the files it happened in as its title
  const noteGroups = (notes: { role: string; action: string; file: string }[]) => {
    const g = new Map<string, Set<string>>();
    for (const n of notes) g.set(`${n.role} ${n.action}`, (g.get(`${n.role} ${n.action}`) ?? new Set()).add(n.file));
    return [...g].map(([k, v]) => [k, [...v]] as const);
  };
</script>

{#snippet reason(what: string, file: string, named: boolean)}
  {#snippet pill()}<Pill tone={r.deleted ? "bad" : "accent"} title={file}>✎ {what}{named ? ` · ${file}` : ""}</Pill>{/snippet}
  {#if onChanges && (scope.commit || view?.files?.find((f: any) => f.path === file)?.change === "deleted")}
    <button type="button" class="note-link" aria-label="Show diff: {what}: {file}" onclick={() => showDiff(file)}>{@render pill()}</button>
  {:else if changes?.files?.find((f: any) => f.path === file)?.change === "deleted"}
    {@render pill()}
  {:else}
    <a href={u(what.startsWith("spec ") ? doc(file, r.id) : { tab: "code", path: file })} aria-label="{what}: {file}">{@render pill()}</a>
  {/if}
{/snippet}

{#if k === "id"}{#if commits[r.id]}<label class="rv"><input type="checkbox" aria-label="reviewed" title={reviewTip(r.id)} checked={isReviewed(r.id)} onchange={(e) => setReviewed(r.id, e.currentTarget.checked ? commits[r.id].sha : undefined)} /></label>{:else if Object.keys(commits).length}<span class="rv muted" title="not committed yet: commit it to review it">·</span>{/if}{#if r.deleted || !known.has(r.id)}{r.id}{:else}<a href={u(route.tab === "requirements" ? { ...route, id: r.id } : req(r.id))}>{r.id}</a>{/if}
{:else if k === "status"}<div class="status"><RequirementBadges level={r.level} state={r.state} priority={r.priority} effort={r.effort} deleted={r.deleted} />{#if r.done}{" "}<HelpPill label={r.verdict} {...verdictMark(r.verdict)} />{/if}</div>
{:else if k === "text"}
  {@const reasons = noteGroups(r.notes ?? changeOf.get(r.id)?.notes ?? []).flatMap(([what, files]) => files.map((file) => ({ what, file, named: files.length > 1 })))}
  <div class="line"><div class="txt" title={plain(r.text)} {@attach levelPills(r.text)}>{@html inline(r.text)}</div>
    <div class="notes">
      {#if !r.deleted && testedIds.has(r.id)}
        <a class="tests" href={u({ tab: "tests", requirementId: r.id })} aria-label="View tagged tests for {r.id}">tests</a>
      {/if}
      <!-- (WEB-04) -->
      {#if reasons.length > 1}
        <Reasons count={reasons.length} tone={r.deleted ? "bad" : "accent"} id={r.id}>{#each reasons as x}{@render reason(x.what, x.file, true)}{/each}</Reasons>
      {:else if reasons.length}{@render reason(reasons[0].what, reasons[0].file, reasons[0].named)}{/if}
    </div>
  </div>
{:else}<FilePath path={r.file} line={r.line || undefined} href={r.line ? u(doc(r.file, r.id)) : undefined} />{/if}

<style>
  .note-link { all: unset; display: inline-flex; align-items: center; min-height: 24px; cursor: pointer; }
  .note-link:focus-visible { outline: var(--focus-ring); }
  .status { display: flex; align-items: center; gap: 6px; }
  .line { display: flex; align-items: center; gap: 8px; }
  .txt { flex: 1 1 0; min-width: 8ch; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .notes { display: flex; flex: none; align-items: center; gap: 6px; margin-inline-start: auto; font-size: var(--text-sm); white-space: nowrap; }
  .notes :global(.pill) { margin: 0; }
  .tests { display: inline-flex; align-items: center; justify-content: center; min-width: 24px; min-height: 24px; }
  /* the label makes the checkbox a 24px target */
  .rv { display: inline-grid; place-items: center; width: 24px; height: 24px; margin-inline-end: 4px; vertical-align: middle; }
  label.rv { cursor: pointer; }
  .rv input { margin: 0; }
</style>

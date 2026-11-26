<script module lang="ts">
  // the open diffs survive a tab switch
  const diffs = $state({ open: [] as string[] });
</script>

<script lang="ts">
  import { tick } from "svelte";
  import Seg from "$lib/ui/Seg.svelte";
  import Section from "$lib/ui/Section.svelte";
  import Pill from "$lib/ui/Pill.svelte";
  import DataTable from "$lib/ui/DataTable.svelte";
  import Toolbar from "$lib/requirements/Toolbar.svelte";
  import ReqCell from "$lib/requirements/ReqCell.svelte";
  import DiffView from "$lib/views/DiffView.svelte";
  import { changeTone, edgeTone } from "$lib/tone";
  import { reqCols, reqVal } from "$lib/requirements/columns";
  import { plural } from "$lib/store";
  import { nav, navigate, u, doc, req, filter } from "$lib/nav.svelte";
  import { data, isDirect, moduleOfFile, type Row } from "$lib/data.svelte";
  import { filters } from "$lib/requirements/filterState.svelte";

  const route = $derived(nav.route);
  const history = $derived(data.history);
  const scope = $derived(data.scope);
  const view = $derived(data.view);
  const gen = $derived(data.gen);
  const known = $derived(data.known);
  const directReqs = $derived(data.directReqs);
  const shown = $derived(filters.shown);
  const filtering = $derived(filters.filtering);
  const reqFilterRows = $derived(filters.rows);
  const toggleDiff = (p: string) => (diffs.open = diffs.open.includes(p) ? diffs.open.filter((x) => x !== p) : [...diffs.open, p]);
  const openDiffs = $derived(diffs.open);
  async function showDiff(path: string) {
    if (!diffs.open.includes(path)) diffs.open = [...diffs.open, path];
    await tick();
    const row = document.getElementById(`changed-file-${path}`);
    const section = row?.closest("details");
    if (section) section.open = true;
    row?.scrollIntoView({ block: "center" });
  }
  // (WEB-33) the short hash of the explained commit, for the section titles
  const at = $derived(scope.commit ? ` (${history?.commits.find((c: any) => c.sha === scope.commit || c.short === scope.commit)?.short ?? scope.commit.slice(0, 7)})` : "");
  const nOf = (kind: string) => (view?.files ?? []).filter((f: any) => f.kind === kind).length;
  const viewMods = $derived(new Set<string>((view?.files ?? []).map((f: any) => moduleOfFile(f.path))));
</script>

{#snippet reqCell(r: any, k: string)}<ReqCell {r} {k} onChanges {showDiff} />{/snippet}

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
    <div class="req-list"><DataTable {rows} columns={reqCols} cell={reqCell} rowKey={(r) => r.id} onselect={(id, how) => how === "click" && id && known.has(id) && navigate(req(id))} value={reqVal} tone={(r: any) => (r.deleted ? "red" : edgeTone(r.state))} keys={false} /></div>
  {/snippet}
  <Section id="changes-requirements" title={`Requirements that may be affected${at}`} count={direct.length} tone={direct.length ? "accent" : undefined}>
    {#if reqFilterRows.length || filtering}<Toolbar />{/if}
    {#if filtering && !shown.length}<p class="muted">No affected requirement matches the filters.</p>
    {:else if !direct.length && !filtering}<p class="muted">No requirement's bullet, tagged test or cited code differs from {view.commit ? "its parent" : view.reference}.</p>
    {:else if !direct.length}<p class="muted">No directly affected requirement matches the filters.</p>
    {:else}{@render reqTable(direct)}{/if}
    {#if indirect.length}<details class="indirect"><summary class="muted">{indirect.length} more reached only through tests that run the changed code (not edited themselves)</summary>{@render reqTable(indirect)}</details>{/if}
    <p class="muted note">A changed file may affect a requirement; it does not say the requirement is done or its test passes. Evidence, not truth.</p>
  </Section>
  <Section id="changes-files" title={`Files${at}`} count={view.files.length} tone="accent">
    <p class="note"><button onclick={() => (diffs.open = view.files.map((f: any) => f.path))}>Expand all diffs</button> <button onclick={() => (diffs.open = [])} disabled={!openDiffs.length}>Collapse all diffs</button></p>
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
  <Section id="changes-other" title="Tests, specs and modules">
    <p>
      <a href={u({ tab: "tests", changed: "1" })}>{plural(nOf("test"), "test file")}</a> ·
      <a href={u(doc(undefined, undefined, "1"))}>{plural(nOf("spec"), "spec file")}</a> ·
      <a href={u({ tab: "modules", changed: "1" })}>{plural(viewMods.size, "module")}</a>
      <span class="muted"> changed ({plural(nOf("src"), "source file")}, {plural(nOf("other"), "other file")} not scanned)</span>
    </p>
  </Section>
{/if}

<style>
  /* capped until the section is given a height of its own */
  .commits { max-height: 13rem; overflow: auto; }
  :global(.body.sized) .commits { max-height: none; }
  .icon { display: inline-flex; align-items: center; position: relative; padding: 6px 9px; line-height: 1.2; }
  tr.sel { background: var(--tint-sel); }
  .diffrow > td { max-width: 0; }
  .hidediff { position: sticky; top: 0; float: right; z-index: 1; }
  details.indirect { margin-top: 10px; }
  details.indirect summary { cursor: pointer; font-size: var(--text-sm); }
</style>

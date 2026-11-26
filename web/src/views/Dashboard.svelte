<script lang="ts">
  import Section from "$lib/ui/Section.svelte";
  import Pill from "$lib/ui/Pill.svelte";
  import ReqLink from "$lib/requirements/ReqLink.svelte";
  import ReqRef from "$lib/requirements/ReqRef.svelte";
  import { changeTone } from "$lib/tone";
  import { problemGroups, type Problem } from "$lib/problems";
  import { plain, plural } from "$lib/store";
  import { u, doc, req, DOCS as docs } from "$lib/nav.svelte";
  import { data } from "$lib/data.svelte";

  const report = $derived(data.report);
  const noReqs = $derived(data.noReqs);
  const modules = $derived(data.modules);
  const changes = $derived(data.changes);
  const directReqs = $derived(data.directReqs);
  const sum = (key: string) => (modules?.modules ?? []).reduce((n: number, m: any) => n + m[key], 0);
  const noTestN = $derived(directReqs.filter((r: any) => r.state === "none" || r.state === "src-only").length);
  const groups = $derived(report ? problemGroups(report.problems) : []);
  const CHANGES = ["added", "modified", "renamed", "deleted"];
  const tallyBy = (files: any[]) => CHANGES.map((c) => [c, files.filter((f) => f.change === c).length] as const).filter(([, n]) => n);
</script>

{#if noReqs}
  <section class="hero card">
    <h2>No requirements yet</h2>
    <p>spec17 found none. Write them as <code>- **ABC-12** MUST — text</code> in <code>specs/01-name.md</code> or <code>SPEC.md</code>, tag tests <code>[ABC-12]</code> and cite <code>// (ABC-12)</code> in code. <a href="/settings">Settings</a> installs the skill so your agent drafts them with you. <a href="{docs}/specs/#requirement-styles" target="_blank" rel="noopener noreferrer">Requirement formats</a>.</p>
    {#if modules}<p class="facts muted"><a href={u({ tab: "modules" })}>{plural(sum("src_files"), "source file")}</a> · {plural(sum("test_files"), "test file")} · <a href={u({ tab: "tests" })}>{plural(sum("tests"), "test")} detected</a></p>{/if}
  </section>
{:else}
<!-- (WEB-67) counts first, the percentage second -->
<section class="hero card" aria-label="Summary">
  <ul class="strip">
    <li>
      <a href={u({ tab: "requirements" })}><b>{report.requirements}</b> {report.requirements === 1 ? "requirement" : "requirements"}</a>
      <span class="sub muted">in {plural(report.spec_files, "spec")}</span>
    </li>
    <li class="coverage">
      {#if report.due}
        <span><b>{report.tested} of {report.due}</b> due with a tagged test</span>
        <span class="sub muted">{report.tested_pct}%</span>
        <span class="meter" aria-hidden="true"><i style="width:{report.tested_pct}%"></i></span>
      {:else}<span class="muted">No requirement is due, so tagged-test coverage is not measured</span>{/if}
    </li>
    <li data-tone={groups[0]?.tone ?? "muted"}>
      <span class="total"><b>{report.total_problems}</b> {report.total_problems === 1 ? "problem" : "problems"}</span>
      <span class="sub muted">{#if groups.length}{groups.map((g) => `${g.n} in ${g.title.toLowerCase()}`).join(" · ")}{:else}none found by the scan{/if}</span>
    </li>
  </ul>
  {#if report.manual || report.later || report.refs}
    <p class="facts muted">
      {#if report.manual}<a href={u({ tab: "requirements", state: "manual" })}>{report.manual} manual</a>{/if}
      {#if report.later}<a href={u({ tab: "requirements", state: "later" })}>{report.later} later</a>{/if}
      {#if report.refs}<a href={u({ tab: "requirements", state: "ref" })}>{plural(report.refs, "reference doc")}</a>{/if}
    </p>
  {/if}
</section>
{/if}
{#if !report.index}<p class="note-line muted">No CodeGraph index: test-to-code links and uncovered code are not checked.</p>{/if}
<!-- (WEB-67) errors open, warnings collapsed; nothing here when the scan finds no problem -->
{#snippet ids(list: string[])}<p class="ids">{#each list as id}<ReqLink {id} to={req(id)} />{/each}</p>{/snippet}
{#snippet what()}<p class="muted"><a href="{docs}/specs/#tests-and-citations" target="_blank" rel="noopener noreferrer">What these mean</a></p>{/snippet}
{#snippet problem(p: Problem)}
  <h3>{p.title} <span class="muted">({p.n})</span></h3>
  {#if p.key === "unimplemented"}{@render ids(report.unimplemented)}
  {:else if p.key === "untested"}{@render ids(report.untested)}
  {:else if p.key === "later_but_built"}{@render ids(report.built)}
  {:else if p.key === "unproven_done"}
    <ul>{#each report.unproven as x}<li><ReqRef id={x.id} to={req(x.id)} /> <code>{x.verdict}</code> <a href={u(doc(x.file, x.id))}>{x.file}:{x.line}</a> <span class="muted">{plain(x.text).slice(0, 80)}</span></li>{/each}</ul>
  {:else if p.key === "invalid_tests"}
    {@render what()}
    {#each Object.entries(report.invalid_tests) as [kind, list]}
      {#if (list as any[]).length}<p><code>{kind}</code> {#each list as any[] as t}<ReqLink id={t.id} to={req(t.id)} /> <a class="muted mono" href={u({ tab: "code", path: t.file, line: t.line })}>{t.file}:{t.line}</a> {/each}</p>{/if}
    {/each}
  {:else if p.key === "spec_issues"}
    <ul>{#each report.spec_issues as x}<li class="mono">{x}</li>{/each}</ul>
  {:else if p.key === "dangling_citations"}
    {@render what()}
    <p class="mono">{#if report.dangling_tests.length}tests: {report.dangling_tests.join(", ")}<br />{/if}{#if report.dangling_sources.length}sources: {report.dangling_sources.join(", ")}{/if}</p>
  {:else if p.key === "specs_to_compare"}
    <p><a href={u({ tab: "overlap" })}>Compare them on the Overlap page</a></p>
  {:else}<p class="muted">Run <code>spec17 report</code> to list them.</p>{/if}
{/snippet}
{#each groups as g (g.id)}
  <Section id="dashboard-{g.id}" title={g.title} count={g.n} tone={g.tone} open={g.open}>
    {#each g.list as p (p.key)}<div>{@render problem(p)}</div>{/each}
  </Section>
{/each}

{#if changes}
  <Section id="dashboard-changes" title="Changes" count={changes.files.length} tone={changes.files.length ? "accent" : undefined}>
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
  <Section id="dashboard-modules" title="Modules" count={modules.modules.length}>
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

<style>
  .hero { margin-bottom: 14px; }
  .strip { display: grid; grid-template-columns: repeat(auto-fit, minmax(13rem, 1fr)); gap: 14px 24px; margin: 0; padding: 0; list-style: none; }
  .strip li { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
  .strip b { font-size: var(--text-page); font-weight: var(--weight-bold); line-height: 1.2; font-variant-numeric: tabular-nums; }
  .strip a { align-self: flex-start; min-height: 24px; color: inherit; text-decoration-color: var(--line-strong); }
  .strip [data-tone] .total { color: var(--tone); }
  .sub { font-size: var(--text-sm); }
  .meter { height: 6px; margin-top: 4px; border-radius: 3px; background: var(--raised); overflow: hidden; }
  .meter i { display: block; height: 100%; background: var(--accent); border-radius: 3px; }
  .facts { display: flex; flex-wrap: wrap; gap: 4px 16px; margin: 14px 0 0; font-size: var(--text-sm); }
  .facts a { min-height: 24px; }
  b[data-tone] { color: var(--tone); }
  h3 { margin: 0 0 6px; font-size: var(--text-body); }
  .ids { display: flex; flex-wrap: wrap; gap: 6px; margin-bottom: 0; }
</style>

<script lang="ts">
  import Section from "$lib/ui/Section.svelte";
  import Pill from "$lib/ui/Pill.svelte";
  import ReqLink from "$lib/requirements/ReqLink.svelte";
  import ReqRef from "$lib/requirements/ReqRef.svelte";
  import { changeTone, type Tone } from "$lib/tone";
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
  const invalidN = $derived(report ? Object.values(report.invalid_tests).reduce((n: number, v: any) => n + v.length, 0) : 0);
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
<section class="hero card">
  <div class="pct"><b>{report.tested_pct}%</b><span>of due requirements have a tagged test <span class="muted metric-count">({report.tested}/{report.due})</span></span></div>
  <div class="meter" role="img" aria-label="{report.tested_pct}% of due requirements have a tagged test"><i style="width:{report.tested_pct}%"></i></div>
  <p class="facts muted">
    <a href={u({ tab: "requirements" })}>{report.requirements} requirements</a> in {report.spec_files} specs
    {#if report.manual} · <a href={u({ tab: "requirements", state: "manual" })}>{report.manual} manual</a>{/if}
    {#if report.later} · <a href={u({ tab: "requirements", state: "later" })}>{report.later} later</a>{/if}
    {#if report.refs} · <a href={u({ tab: "requirements", state: "ref" })}>{report.refs} reference docs</a>{/if}
    · <b data-tone={report.total_problems ? "bad" : "ok"}>{report.total_problems ? `${report.total_problems} problems` : "no problems found"}</b>
  </p>
</section>
{/if}
{#if !report.index}<p class="note-line muted">No CodeGraph index: test-to-code links and uncovered code are not checked.</p>{/if}
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

{#snippet ids(key: string, title: string, list: string[], sev: Tone)}
  {#if list.length}
    <Section id="dashboard-{key}" {title} count={list.length} tone={sev}>
      <p class="ids">{#each list as id}<ReqLink {id} to={req(id)} />{/each}</p>
    </Section>
  {/if}
{/snippet}
{@render ids("unimplemented", "Unimplemented: due, no test and no code", report.unimplemented, "bad")}
{@render ids("untested", "Untested: due, cited in code but no test", report.untested, "warn")}
{@render ids("built", "Marked later but already has tests or code", report.built, "warn")}

{#if report.unproven.length}
  <Section id="dashboard-unproven" title="Check-marked done without proof" count={report.unproven.length} tone="warn">
    <ul>{#each report.unproven as x}<li><ReqRef id={x.id} to={req(x.id)} /> <code>{x.verdict}</code> <a href={u(doc(x.file, x.id))}>{x.file}:{x.line}</a> <span class="muted">{plain(x.text).slice(0, 80)}</span></li>{/each}</ul>
  </Section>
{/if}
{#if invalidN}
  <Section id="dashboard-invalid" title="Invalid tests" count={invalidN} tone="bad">
    <p class="muted"><a href="{docs}/specs/#tests-and-citations" target="_blank" rel="noopener noreferrer">What these mean</a></p>
    {#each Object.entries(report.invalid_tests) as [kind, list]}
      {#if (list as any[]).length}<p><code>{kind}</code> {#each list as any[] as p}<ReqLink id={p.id} to={req(p.id)} /> <a class="muted mono" href={u({ tab: "code", path: p.file, line: p.line })}>{p.file}:{p.line}</a> {/each}</p>{/if}
    {/each}
  </Section>
{/if}
{#if report.spec_issues.length}
  <Section id="dashboard-issues" title="Spec issues" count={report.spec_issues.length} tone="bad">
    <ul>{#each report.spec_issues as x}<li class="mono">{x}</li>{/each}</ul>
  </Section>
{/if}
{#if report.dangling_tests.length + report.dangling_sources.length}
  <Section id="dashboard-dangling" title="Dangling citations" count={report.dangling_tests.length + report.dangling_sources.length} tone="warn">
    <p class="muted"><a href="{docs}/specs/#tests-and-citations" target="_blank" rel="noopener noreferrer">What these mean</a></p>
    <p class="mono">{#if report.dangling_tests.length}tests: {report.dangling_tests.join(", ")}<br />{/if}{#if report.dangling_sources.length}sources: {report.dangling_sources.join(", ")}{/if}</p>
  </Section>
{/if}

<style>
  .hero { margin-bottom: 14px; }
  .pct { display: flex; align-items: baseline; gap: 12px; flex-wrap: wrap; }
  .pct b { font-size: var(--text-metric); font-weight: var(--weight-bold); line-height: 1.1; letter-spacing: -0.02em; }
  .metric-count { font-size: var(--text-sm); }
  .meter { height: 8px; margin: 10px 0 8px; border-radius: 4px; background: var(--raised); overflow: hidden; }
  .meter i { display: block; height: 100%; background: var(--accent); border-radius: 4px; }
  .facts { margin: 0; font-size: var(--text-sm); }
  b[data-tone] { color: var(--tone); }
  .ids { display: flex; flex-wrap: wrap; gap: 6px; margin-bottom: 0; }
</style>

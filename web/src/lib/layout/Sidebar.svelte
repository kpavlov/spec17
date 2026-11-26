<script lang="ts">
  import { MediaQuery } from "svelte/reactivity";
  import Icon from "$lib/ui/Icon.svelte";
  import { nav, u, reqView } from "$lib/nav.svelte";
  import { data } from "$lib/data.svelte";
  import { stored } from "$lib/store";
  import type { Route } from "$lib/router";

  // (WEB-18) labels + icons, or an icon rail (remembered; narrow screens start as a rail, phones always are)
  let { collapsed = $bindable(false) }: { collapsed?: boolean } = $props();
  const compactNav = new MediaQuery("(max-width: 75rem)");
  const mobileNav = new MediaQuery("(max-width: 48rem)");
  let rail = $state(stored("spec17.nav") ? stored("spec17.nav") === "rail" : compactNav.current);
  const toggleNav = () => { rail = !rail; try { localStorage.setItem("spec17.nav", rail ? "rail" : "full"); } catch { /* storage blocked */ } };
  const NAV: { name: string; tabs: string[] }[] = [
    { name: "Overview", tabs: ["dashboard", "changes"] },
    { name: "Browse", tabs: ["requirements", "modules", "tests"] },
    { name: "Gaps", tabs: ["overlap", "uncovered"] },
  ];
  $effect(() => { collapsed = rail || mobileNav.current; });

  const route = $derived(nav.route);
  const report = $derived(data.report);
  const directReqs = $derived(data.directReqs);
</script>

{#snippet navLink(t: string)}
  <a class="tab" class:on={route.tab === t} aria-current={route.tab === t ? "page" : undefined} href={u({ tab: t as Route["tab"], view: t === "requirements" && reqView.value !== "list" ? reqView.value : undefined })} title={t} aria-label={t}>
    <span class="ico"><Icon name={t as "dashboard"} /></span><span class="lbl">{t}</span>
    {#if t === "dashboard" && report?.total_problems}<span class="badge" data-tone="bad" title="{report.total_problems} problems">{report.total_problems}</span>{/if}
    {#if t === "changes" && directReqs.length}<span class="badge" data-tone="accent" title="{directReqs.length} requirements may be affected">{directReqs.length}</span>{/if}
  </a>
{/snippet}

<nav aria-label="views">
  {#each NAV as g}
    <div class="group">
      <span class="gname">{g.name}</span>
      {#each g.tabs as t}{@render navLink(t)}{/each}
    </div>
  {/each}
  <span class="grow"></span>
  <div class="group">
    <button class="collapse" onclick={toggleNav} aria-label={rail ? "Expand the sidebar" : "Collapse the sidebar"} title={rail ? "Expand the sidebar" : "Collapse the sidebar"}><span class="ico"><Icon name={rail ? "expand" : "collapse"} /></span><span class="lbl">Collapse</span></button>
  </div>
</nav>

<style>
  nav { position: sticky; top: var(--header-h); align-self: start; height: calc(100dvh - var(--header-h)); display: flex; flex-direction: column; gap: 2px; padding: 10px 8px; border-right: 1px solid var(--line); overflow-y: auto; scrollbar-width: none; }
  .group { display: flex; flex-direction: column; gap: 2px; }
  .group + .group, .grow + .group { border-top: 1px solid var(--line); margin-top: 6px; padding-top: 6px; }
  .grow { flex: 1; }
  .gname { padding: 6px 10px 2px; font-size: var(--text-sm); font-weight: var(--weight-semibold); color: var(--muted); }
  a.tab, .collapse { display: flex; align-items: center; gap: 10px; position: relative; padding: var(--nav-pad); white-space: nowrap; background: transparent; color: var(--fg); border-radius: var(--radius); text-decoration: none; }
  .collapse { color: var(--muted); font-size: var(--text-label); }
  a.tab:hover, .collapse:hover:not(:disabled) { background: var(--tint-btn); color: var(--accent); }
  a.tab.on { background: var(--tint-on); color: var(--accent); }
  .ico { display: flex; flex-shrink: 0; }
  .badge { margin-left: auto; }
  :global(.shell.rail) :is(.gname, .lbl) { display: none; }
  :global(.shell.rail) nav { align-items: stretch; }
  :global(.shell.rail) :is(a.tab, .collapse) { justify-content: center; padding: 7px 0; }
  :global(.shell.rail) .badge { position: absolute; top: 0; right: 2px; margin: 0; min-width: 14px; padding: 0 3px; font-size: var(--text-meta); }
  @media (max-width: 48rem) {
    nav, :global(.shell.rail) nav { position: static; height: auto; flex-direction: row; align-items: center; padding: 4px 8px; border-right: 0; border-bottom: 1px solid var(--line); overflow-x: auto; }
    .group, .group + .group, .grow + .group { flex-direction: row; border: 0; margin: 0; padding: 0; }
    .grow, .collapse { display: none; }
    a.tab, :global(.shell.rail) a.tab { padding: 7px 10px; }
  }
</style>

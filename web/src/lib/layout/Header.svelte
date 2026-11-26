<script lang="ts">
  import Icon from "$lib/ui/Icon.svelte";
  import { ago } from "$lib/ago";
  import { togglePalette } from "$lib/layout/Palette.svelte";
  import { prefs, setPref } from "$lib/prefs.svelte";
  import { nav, u } from "$lib/nav.svelte";
  import { data } from "$lib/data.svelte";
  import { setupState, rescan } from "$lib/setup.svelte";

  // one button cycles system → light → dark; it shows the current state
  const themes: { v: string; icon: "system" | "sun" | "moon"; name: string }[] = [{ v: "auto", icon: "system", name: "system" }, { v: "light", icon: "sun", name: "light" }, { v: "dark", icon: "moon", name: "dark" }];
  const curTheme = $derived(themes.find((t) => t.v === prefs.theme)!);
  const nextTheme = $derived(themes[(themes.indexOf(curTheme) + 1) % themes.length]);

  const route = $derived(nav.route);
  const up = $derived(data.up);
  const root = $derived(data.root);
  // (WEB-66) repo · branch · scan age
  const repo = $derived(root.replace(/[\\/]+$/, "").split(/[\\/]/).pop() ?? "");
  let now = $state(Date.now());
  $effect(() => { const t = setInterval(() => (now = Date.now()), 30_000); return () => clearInterval(t); });
  const age = $derived(data.loadedAt ? `scanned ${ago(Math.max(now, data.loadedAt) - data.loadedAt)}` : "");
  const busy = $derived(setupState.busy);
  const setupBadge = $derived(setupState.setupBadge);
</script>

<header>
  <h1><a href="/">spec17</a></h1>
  <span class="dot" data-tone={up === true ? "ok" : up === false ? "bad" : undefined} role="status" title={up === null ? "server: connecting" : up ? "server: connected" : "server: disconnected, retrying"}></span>
  {#if repo}<span class="where mono muted"><span title={root}>{repo}</span>{#if data.branch}{" · "}<span title="current git branch">{data.branch}</span>{/if}</span>{/if}
  <span class="grow"></span>
  {#if data.loadedAt}<span class="age muted" title="The dashboard loaded the scan at {new Date(data.loadedAt).toLocaleTimeString()}">{age}</span>{/if}
  <button class="icon" onclick={togglePalette} title="Command palette (⌘K or Ctrl+K)" aria-label="command palette" aria-haspopup="dialog"><Icon name="search" /></button>
  <button class="icon" disabled={busy} onclick={rescan} title="Rescan the repo (syncs the CodeGraph index first){age ? `, ${age}` : ''}" aria-label="rescan{age ? `, ${age}` : ''}"><Icon name="rescan" /></button>
  <button class="icon" onclick={() => setPref("theme", nextTheme.v)} title="Theme: {curTheme.name} (click for {nextTheme.name})" aria-label="Theme: {curTheme.name}, switch to {nextTheme.name}"><Icon name={curTheme.icon} /></button>
  <a class="tab icon" class:on={route.tab === "settings"} aria-current={route.tab === "settings" ? "page" : undefined} href={u({ tab: "settings" })} title={setupBadge ? "Settings (needs attention)" : "Settings"} aria-label="Settings"><Icon name="settings" />{#if setupBadge}<span class="badge" data-tone={setupBadge} title="needs attention: open Settings">!</span>{/if}</a>
</header>

<style>
  header { position: sticky; top: 0; z-index: 3; display: flex; align-items: center; gap: var(--gap); height: var(--header-h); padding: 0 var(--gutter); background: color-mix(in srgb, var(--bg) 92%, transparent); backdrop-filter: blur(8px); border-bottom: 1px solid var(--line); }
  header > :not(.where) { flex-shrink: 0; }
  .dot { width: 8px; height: 8px; border-radius: 50%; background: var(--tone, var(--muted)); margin-left: -4px; }
  .icon { display: inline-flex; align-items: center; justify-content: center; position: relative; min-width: 32px; min-height: 32px; padding: 6px 9px; line-height: 1.2; }
  .where { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .age { font-size: var(--text-sm); white-space: nowrap; }
  h1 { margin: 0; font-size: var(--text-section); font-weight: var(--weight-bold); letter-spacing: -0.01em; }
  h1 a { display: inline-block; min-height: 24px; padding-block: 2px; color: inherit; text-decoration: none; }
  a.tab.icon { padding: 6px 9px; }
  a.tab.icon .badge { position: absolute; top: -4px; right: -4px; margin: 0; min-width: 14px; padding: 0 3px; font-size: var(--text-meta); }
  @media (max-width: 48rem) {
    header { padding: 0 8px; }
    .age { display: none; }
    .icon { padding: 4px 6px; }
  }
</style>

<script lang="ts">
  import { untrack, type Snippet } from "svelte";
  import type { Tone } from "$lib/tone";
  import { loadSection, saveSection } from "$lib/layout/layout.svelte";
  // a collapsible block with a title and a count; its tone colours the left edge, the tint and the count.
  // With an `id` (WEB-34) it also remembers per browser whether it is open and the height dragged on the gap below it.
  let { title, count, tone, open = true, id, children }: { title: string; count?: number; tone?: Tone; open?: boolean; id?: string; children: Snippet } = $props();
  // `id` and the default are read once: what was remembered takes over from there
  const initial = untrack(() => open);
  const kept = untrack(() => (id ? loadSection(id, initial) : null));
  let isOpen = $state(kept ? kept.open : initial);
  let height = $state<number | null>(kept?.h ?? null);
  let body = $state<HTMLElement>();
  $effect(() => { if (id) saveSection(id, { open: isOpen, h: height }, initial); });
  const MIN = 48, STEP = 24;
  // the body's height now: what the separator reports, and where a drag or a key starts from
  let shownHeight = $state(0);
  const resize = (h: number) => (height = Math.max(MIN, Math.round(h)));
  let from: { y: number; h: number } | null = null;
  const down = (e: PointerEvent) => {
    if (e.button !== 0) return;
    e.preventDefault(); // no text selection while dragging
    const gap = e.currentTarget as HTMLElement;
    gap.focus();
    gap.setPointerCapture(e.pointerId);
    from = { y: e.clientY, h: body!.offsetHeight };
  };
  // only a real move fixes the height: a plain click leaves it automatic
  const move = (e: PointerEvent) => { if (from && (height !== null || e.clientY !== from.y)) resize(from.h + e.clientY - from.y); };
  const keys = (e: KeyboardEvent) => {
    const step: Record<string, number> = { ArrowUp: -STEP, ArrowDown: STEP };
    if (e.key === "Home") height = null;
    else if (step[e.key]) resize(body!.offsetHeight + step[e.key]);
    else return;
    e.preventDefault();
  };
  const auto = (e: Event) => { e.preventDefault(); height = null; };
</script>

<details class="section" data-tone={tone} bind:open={isOpen}>
  <summary><span>{title}</span>{#if height !== null}<button class="auto" title="Back to the automatic height" aria-label="Back to the automatic height of {title}" onclick={auto}>↕</button>{/if}{#if count !== undefined}<b class="count">{count}</b>{/if}</summary>
  {#if id}
    <div class="body" class:sized={height !== null} bind:this={body} bind:offsetHeight={shownHeight} style:max-height={height === null ? null : `${height}px`}>{@render children()}</div>
    <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
    <div class="gap" role="separator" tabindex="0" aria-orientation="horizontal" aria-label="resize {title}" aria-valuemin={MIN} aria-valuemax={Math.max(2000, shownHeight)} aria-valuenow={shownHeight} aria-valuetext="{shownHeight} pixels{height === null ? ', automatic' : ''}" title="Drag to resize; double-click or Home to reset" onpointerdown={down} onpointermove={move} onpointerup={() => (from = null)} onpointercancel={() => (from = null)} ondblclick={() => (height = null)} onkeydown={keys}></div>
  {:else}
    {@render children()}
  {/if}
</details>

<style>
  .section { background: var(--card); border: 1px solid var(--line); border-left: 4px solid var(--line); border-radius: var(--radius); padding: 10px 14px; margin-bottom: var(--gap); }
  .section[data-tone] { border-left-color: var(--tone); background: color-mix(in srgb, var(--tone) 5%, var(--card)); }
  summary { cursor: pointer; display: flex; align-items: center; gap: 8px; }
  summary > span { font-size: var(--text-section); line-height: 1.3; font-weight: var(--weight-semibold); }
  summary span { flex: 1; }
  .section > :global(:not(summary)) { margin-top: var(--gap); }
  .body { overflow: auto; }
  /* the space below the block is its resize handle; the bar shows on hover and keyboard focus */
  .section { position: relative; }
  .section > .gap { position: absolute; left: -4px; right: -1px; top: 100%; height: var(--gap); margin: 1px 0 0; touch-action: none; cursor: ns-resize; }
  .gap::before { content: ""; position: absolute; inset: 3px 0; border-radius: 2px; }
  .gap:hover::before, .gap:focus-visible::before { background: var(--accent); }
  .gap:focus-visible { outline: none; }
  .body > :global(* + *) { margin-top: var(--gap); }
  /* the section spaces its body: the outer margins of what is inside would add to it */
  .body > :global(:first-child) { margin-top: 0; }
  .body > :global(:last-child) { margin-bottom: 0; }
  .auto { all: unset; cursor: pointer; font-weight: 400; color: var(--muted); padding: 0 6px; }
  .auto:hover, .auto:focus-visible { color: var(--fg); }
  .section :global(ul) { margin: 0; padding-left: 18px; }
  .section :global(li) { margin: 4px 0; }
  .count { font-size: var(--text-sm); padding: 0 7px; border-radius: 2px; color: var(--card); background: var(--tone, var(--muted)); }
</style>

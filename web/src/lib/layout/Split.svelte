<script lang="ts">
  import type { Snippet } from "svelte";
  import { layout, listPct, setPos, sideBySide } from "./layout.svelte";

  // `name` keeps its own remembered size, `def` is the list pane's percent beside the preview
  let { list, preview, showPreview = true, name = "", def = 60 }: { list: Snippet; preview: Snippet; showPreview?: boolean; name?: string; def?: number } = $props();
  const pct = $derived(listPct(name, def));

  const drag = (e: PointerEvent) => {
    const g = e.currentTarget as HTMLElement, box = g.parentElement!.getBoundingClientRect();
    if (e.type === "pointerdown") g.setPointerCapture(e.pointerId);
    else if (!g.hasPointerCapture(e.pointerId)) return;
    setPos(name, sideBySide() ? ((e.clientX - box.left) / box.width) * 100 : ((e.clientY - box.top) / box.height) * 100, false);
  };
  const keys = (e: KeyboardEvent) => {
    const step: Record<string, number> = sideBySide() ? { ArrowLeft: -5, ArrowRight: 5 } : { ArrowUp: -5, ArrowDown: 5 };
    if (e.key === "Home") setPos(name, null);
    else if (step[e.key]) setPos(name, pct + step[e.key]);
    else return;
    e.preventDefault();
  };
</script>

<!-- two panes that scroll on their own; the list on the left on wide screens (or on top when chosen), a draggable splitter between them -->
<div class="split" class:single={!showPreview} class:h={layout.split === "h"} style="--a: {pct}fr; --b: {100 - pct}fr">
  <div class="list">{@render list()}</div>
  {#if showPreview}
  <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
  <div class="gutter" role="separator" tabindex="0" aria-label="resize list and preview" aria-orientation={sideBySide() ? "vertical" : "horizontal"} aria-valuemin="15" aria-valuemax="85" aria-valuenow={pct} title="Drag to resize; double-click or Home to reset" onpointerdown={drag} onpointermove={drag} onpointerup={() => setPos(name, pct)} ondblclick={() => setPos(name, null)} onkeydown={keys}></div>
  <div class="preview">{@render preview()}</div>
  {/if}
</div>

<style>
  .split { display: grid; grid-template: minmax(0, var(--a)) 14px minmax(0, var(--b)) / minmax(0, 1fr); flex: 1; min-height: 360px; }
  @media (min-width: 1000px) { .split:not(.h) { grid-template: minmax(0, 1fr) / minmax(0, var(--a)) 14px minmax(0, var(--b)); } }
  .split.single { grid-template: minmax(0, 1fr) / minmax(0, 1fr); }
  .list { overflow: auto; min-height: 0; }
  .preview { min-height: 0; overflow: auto; background: var(--card); border: 1px solid var(--line); border-radius: var(--radius); padding: 10px 16px 14px; box-shadow: var(--shadow); }
  .gutter { position: relative; touch-action: none; cursor: row-resize; }
  .gutter::before { content: ""; position: absolute; inset: 6px 0; border-radius: 2px; background: var(--line); }
  @media (min-width: 1000px) { .split:not(.h) .gutter { cursor: col-resize; } .split:not(.h) .gutter::before { inset: 0 6px; } }
  .gutter:hover::before, .gutter:focus-visible::before { background: var(--accent); }
  .gutter:focus-visible { outline: none; }
</style>

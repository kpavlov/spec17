<script lang="ts">
  import { tick } from "svelte";
  import type { Tone } from "$lib/tone";

  type Opt = { key: string; label: string; count?: number; tone?: Tone; title?: string };
  // Single-choice groups select with arrows. Multi-choice groups use toggle buttons; a click with Shift, Ctrl or ⌘ held says `additive`.
  let { label, keycap, hint, options, value, selected, nowrap = false, onpick }: { label: string; keycap?: string; hint?: string; options: Opt[]; value?: string; selected?: string[]; nowrap?: boolean; onpick: (key: string, additive?: boolean) => void } = $props();
  const picked = (key: string) => selected ? selected.includes(key) : value === key;
  let bar = $state<HTMLElement>();

  function arrow(e: KeyboardEvent) {
    const d = e.key === "ArrowRight" ? 1 : e.key === "ArrowLeft" ? -1 : 0;
    if (!d || selected) return;
    e.preventDefault();
    const keys = options.map((o) => o.key);
    onpick(keys[(keys.indexOf(value ?? "") + d + keys.length) % keys.length]);
    tick().then(() => bar?.querySelector<HTMLElement>("[aria-checked=true]")?.focus());
  }
</script>

<div class="seg" class:nowrap role={selected ? "group" : "radiogroup"} aria-label={label} title={hint} bind:this={bar}>
  <span class="name" aria-hidden="true">{label}{#if keycap}{" "}<kbd>{keycap}</kbd>{/if}</span>
  {#each options as o (o.key)}
    <button type="button" role={selected ? undefined : "radio"} class:zero={o.count === 0 && !picked(o.key)} aria-pressed={selected ? picked(o.key) : undefined} aria-checked={selected ? undefined : picked(o.key)} tabindex={selected || picked(o.key) ? 0 : -1} title={o.title} onclick={(e) => onpick(o.key, e.shiftKey || e.ctrlKey || e.metaKey)} onkeydown={arrow}>{#if selected}<span class="check" aria-hidden="true">✓</span>{/if}{#if o.tone}<i class="dot" data-tone={o.tone}></i>{/if}<span class="text" data-text={o.label}>{o.label}</span>{#if o.count !== undefined}{" "}<b>{o.count}</b>{/if}</button>
  {/each}
</div>

<style>
  .seg { display: inline-flex; flex-wrap: wrap; align-items: center; row-gap: 4px; }
  .nowrap { flex-wrap: nowrap; }
  .name { color: var(--muted); font-size: var(--text-sm); margin-right: 8px; }
  button { min-height: 28px; padding: 2px var(--segment-padding-inline, 10px); font-size: var(--text-sm); border-radius: 0; border: 1px solid var(--line-strong); margin-left: -1px; white-space: nowrap; background: var(--card); }
  .name + button { border-radius: var(--radius) 0 0 var(--radius); margin-left: 0; }
  button:last-child { border-radius: 0 var(--radius) var(--radius) 0; }
  /* the dot keeps its colour when the segment is selected */
  button:is([aria-checked=true], [aria-pressed=true]) { position: relative; z-index: 1; background: var(--tint-sel); border-color: var(--accent); color: var(--fg); font-weight: var(--weight-semibold); }
  button:is([aria-checked=true], [aria-pressed=true]):hover:not(:disabled) { background: var(--tint-sel-hover); color: var(--fg); }
  button[aria-pressed=true] { background: var(--tint-sel-hover); box-shadow: inset 0 -2px var(--accent); }
  /* the selected label is bold: an invisible bold copy keeps the button as wide as its bold self, so selecting does not move its neighbours */
  .text { display: inline-flex; flex-direction: column; align-items: center; }
  .text::after { content: attr(data-text); height: 0; overflow: hidden; visibility: hidden; font-weight: var(--weight-semibold); }
  .check { display: inline-block; margin-right: 5px; color: var(--accent); visibility: hidden; }
  button[aria-pressed=true] .check { visibility: visible; }
  button.zero { color: var(--muted); }
  b { font-weight: var(--weight-medium); font-variant-numeric: tabular-nums; margin-left: 2px; }
  .dot { display: inline-block; width: 8px; height: 8px; border-radius: 50%; margin: 0 6px 1px 0; background: var(--tone, var(--muted)); }
</style>

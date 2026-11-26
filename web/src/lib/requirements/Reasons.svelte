<script lang="ts">
  import Pill from "$lib/ui/Pill.svelte";
  import { place } from "$lib/ui/place";
  import type { Snippet } from "svelte";
  import type { Tone } from "$lib/tone";

  // (WEB-33) several change reasons fold into one pill with their count; click, Enter or Space opens the list, Escape or a click outside closes it
  let { count, tone, id: requirement, children }: { count: number; tone: Tone; id: string; children: Snippet } = $props();
  const id = $props.id();
  let trigger: HTMLButtonElement;
  const toggled = (e: ToggleEvent) => { if (e.newState === "open") place(e.currentTarget as HTMLElement, trigger); };
</script>

<button bind:this={trigger} type="button" class="count" popovertarget={id} aria-label="{count} change reasons for {requirement}"><Pill {tone}>✎ {count} reasons</Pill></button>
<div {id} popover="auto" ontoggle={toggled}>{@render children()}</div>

<style>
  .count { display: inline-flex; align-items: center; min-height: 24px; padding: 0; background: transparent; border: 0; }
  .count:hover:not(:disabled) { background: transparent; }
  .count :global(.pill) { margin: 0; }
  [popover] { position: fixed; inset: auto; margin: 0; max-width: calc(100vw - 16px); max-height: 50dvh; overflow: auto; padding: 6px 10px; background: var(--card); color: var(--fg); border: 1px solid var(--muted); border-radius: var(--radius); white-space: nowrap; }
  [popover]:popover-open { display: flex; flex-direction: column; align-items: flex-start; gap: 6px; }
</style>

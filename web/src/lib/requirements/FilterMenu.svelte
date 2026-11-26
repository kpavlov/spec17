<script lang="ts">
  import { place } from "$lib/ui/place";
  import type { Snippet } from "svelte";

  // (WEB-59) the "+ Filter" button and its menu. A native popover: closed on each load; Escape and a click outside close it
  let { active, disabled = false, title, children }: { active: number; disabled?: boolean; title: string; children: Snippet } = $props();
  let trigger: HTMLButtonElement;
  let open = $state(false);
  const toggled = (e: ToggleEvent) => { open = e.newState === "open"; if (open) place(e.currentTarget as HTMLElement, trigger); };
</script>

<button bind:this={trigger} type="button" class="filter-toggle" popovertarget="filter-menu" {disabled} {title} aria-expanded={open} aria-label={active ? `+ Filter, ${active} active` : "+ Filter"}>+ Filter{#if active}<span class="n">{active}</span>{/if}</button>
<div class="req-options" id="filter-menu" popover="auto" role="group" aria-label="Filters" ontoggle={toggled}>{@render children()}</div>

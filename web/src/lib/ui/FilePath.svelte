<script lang="ts">
  import { middle } from "$lib/truncate";
  import { place } from "$lib/ui/place";

  // (WEB-65) a location in a list row: cut in the middle, the whole of it on hover and focus, and a button that copies it
  let { path, line, href }: { path: string; line?: number; href?: string } = $props();
  const id = $props.id();
  const full = $derived(line ? `${path}:${line}` : path);
  const short = $derived(middle(full));
  let wrap: HTMLElement;
  let tip: HTMLElement;
  let copied = $state(false);
  const show = () => { if (short === full) return; tip.showPopover(); place(tip, wrap); };
  const hide = () => { if (tip.matches(":popover-open")) tip.hidePopover(); };
  const escape = (e: KeyboardEvent) => { if (e.key === "Escape") hide(); };
  const copy = () => navigator.clipboard.writeText(full).then(() => { copied = true; setTimeout(() => (copied = false), 1200); }).catch(() => { /* clipboard blocked: the tooltip still shows the path */ });
</script>

<svelte:window onkeydown={escape} onscrollcapture={hide} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<span class="path" bind:this={wrap} onpointerenter={show} onpointerleave={hide} onfocusin={show} onfocusout={hide}>
  {#if href}<a {href} aria-label={full} aria-describedby={short === full ? undefined : id}>{short}</a>{:else}<span>{short}</span>{/if}
  <button type="button" class="copy" aria-label="Copy path {full}" title="Copy path" onclick={copy}>{copied ? "✓" : "⧉"}</button>
</span>
<span bind:this={tip} {id} popover="manual" role="tooltip">{full}</span>

<style>
  .path { display: inline-flex; align-items: center; gap: 2px; white-space: nowrap; }
  /* the button keeps its place, so the row does not shift: it shows on the row's hover and focus */
  .copy { min-width: 24px; min-height: 24px; padding: 0; background: transparent; color: var(--muted); opacity: 0; }
  :global(tr:hover) .copy, .path:focus-within .copy { opacity: 1; }
  @media (hover: none) { .copy { opacity: 1; } }
  [popover] { position: fixed; inset: auto; margin: 0; max-width: calc(100vw - 16px); overflow-wrap: anywhere; padding: 6px 10px; background: var(--tooltip-bg); color: var(--fg); border: 1px solid var(--muted); border-radius: var(--radius); font: var(--text-sm)/1.5 var(--mono); }
</style>

<script lang="ts">
  import { tick } from "svelte";
  import Pill from "./Pill.svelte";
  import { place } from "./place";
  import Icon, { type IconName } from "./Icon.svelte";
  import type { Tone } from "$lib/tone";

  // with an icon the label is not drawn: it stays in the accessible name and the tooltip
  let { label, title, tone, icon }: { label: string; title: string; tone?: Tone; icon?: IconName } = $props();
  const id = $props.id();
  let tip: HTMLElement;
  let trigger: HTMLButtonElement;
  let pinned = false;
  let open = $state(false);

  function close() {
    open = false;
    pinned = false;
    tip.hidePopover();
  }
  $effect(() => {
    if (!open) return;
    const escape = (e: KeyboardEvent) => { if (e.key === "Escape") close(); };
    window.addEventListener("keydown", escape);
    window.addEventListener("scroll", close, true);
    window.addEventListener("resize", close);
    return () => {
      window.removeEventListener("keydown", escape);
      window.removeEventListener("scroll", close, true);
      window.removeEventListener("resize", close);
    };
  });

  async function show() {
    open = true;
    tip.showPopover();
    await tick();
    place(tip, trigger);
  }
</script>

<button bind:this={trigger} type="button" aria-label={title.split(" — ")[0]} aria-describedby={id} onfocus={show} onblur={close}
  onpointerleave={() => { if (!pinned && document.activeElement !== trigger) close(); }}
  onpointerenter={(e) => { if (e.pointerType !== "touch") show(); }}
  onclick={() => { if (pinned) close(); else { pinned = true; show(); } }}>
  <Pill {tone}>{#if icon}<Icon name={icon} />{:else}{label}{/if}</Pill>
</button>
<span bind:this={tip} {id} popover="manual" role="tooltip">{title}</span>

<style>
  button { display: inline-flex; align-items: center; justify-content: center; min-width: 24px; min-height: 24px; line-height: 1.25; padding: 0; border: 0; background: transparent; font-family: var(--sans); cursor: help; }
  button > :global(span) { margin-right: 0; }
  button > :global(span:has(> svg)) { display: grid; place-items: center; min-width: 24px; min-height: 24px; padding: 0 3px; }
  button:hover:not(:disabled) { background: transparent; }
  [popover] { position: fixed; inset: auto; margin: 0; max-width: min(20rem, calc(100vw - 16px)); white-space: normal; padding: 6px 10px; background: var(--tooltip-bg); color: var(--fg); border: 1px solid var(--muted); border-radius: var(--radius); font: var(--text-sm)/1.5 var(--sans); }
</style>

<script lang="ts">
  import { onDestroy, tick } from "svelte";
  import IdLink from "$lib/ui/IdLink.svelte";

  let { id, href, getText }: { id: string; href: string; getText: (id: string) => Promise<string> } = $props();
  const tipId = $props.id();
  let trigger: HTMLSpanElement;
  let tip: HTMLElement;
  let text = $state("");
  let open = $state(false);
  let timer: ReturnType<typeof setTimeout>;
  let sequence = 0;

  function close() {
    clearTimeout(timer);
    sequence++;
    open = false;
    if (tip?.matches(":popover-open")) tip.hidePopover();
  }
  function schedule() {
    clearTimeout(timer);
    if (open) return;
    const current = ++sequence;
    timer = setTimeout(async () => {
      text = "Loading…";
      open = true;
      tip.showPopover();
      await tick();
      const box = trigger.getBoundingClientRect();
      tip.style.left = Math.max(8, Math.min(box.left, innerWidth - tip.offsetWidth - 8)) + "px";
      tip.style.top = (box.bottom + tip.offsetHeight + 4 <= innerHeight ? box.bottom : Math.max(4, box.top - tip.offsetHeight)) + "px";
      const result = await getText(id);
      if (sequence === current) {
        text = result;
        await tick();
        tip.style.left = Math.max(8, Math.min(box.left, innerWidth - tip.offsetWidth - 8)) + "px";
        tip.style.top = (box.bottom + tip.offsetHeight + 4 <= innerHeight ? box.bottom : Math.max(4, box.top - tip.offsetHeight)) + "px";
      }
    }, 500);
  }
  function leave() { clearTimeout(timer); timer = setTimeout(close, 100); }
  onDestroy(close);
  $effect(() => {
    if (!open) return;
    const escape = (e: KeyboardEvent) => { if (e.key === "Escape") close(); };
    window.addEventListener("keydown", escape);
    window.addEventListener("scroll", close, true);
    window.addEventListener("resize", close);
    return () => { window.removeEventListener("keydown", escape); window.removeEventListener("scroll", close, true); window.removeEventListener("resize", close); };
  });
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<span bind:this={trigger} onpointerenter={(e) => { if (e.pointerType !== "touch") schedule(); }} onpointerleave={() => { if (!trigger.contains(document.activeElement)) leave(); }} onfocusin={schedule} onfocusout={close}>
  <IdLink {href} describedby={tipId}>{id}</IdLink>
</span>
<!-- svelte-ignore a11y_no_static_element_interactions -->
<span bind:this={tip} id={tipId} popover="manual" role="tooltip" onpointerenter={() => clearTimeout(timer)} onpointerleave={leave}><span class="preview">{text}</span></span>

<style>
  .preview { display: -webkit-box; -webkit-box-orient: vertical; -webkit-line-clamp: 3; overflow: hidden; overflow-wrap: anywhere; }
  [popover] { position: fixed; inset: auto; margin: 0; max-width: min(24rem, calc(100vw - 16px)); white-space: normal; padding: 6px 10px; background: var(--tooltip-bg); color: var(--fg); border: 1px solid var(--muted); border-radius: var(--radius); font: var(--text-sm)/1.5 system-ui, sans-serif; }
</style>

<script module lang="ts">
  export const togglePalette = () => {
    const d = document.getElementById("palette") as HTMLDialogElement | null;
    d?.open ? d.close() : d?.showModal();
  };
</script>

<script lang="ts">
  import { search } from "$lib/palette";
  import { navigate, req } from "$lib/nav.svelte";
  import { TABS, type Route } from "$lib/router";
  import { data } from "$lib/data.svelte";
  import { setupState, rescan } from "$lib/setup.svelte";
  import { setPref, DISPLAY } from "$lib/prefs.svelte";
  import { plain } from "$lib/store";
  import { saved, applyView } from "$lib/requirements/viewStore.svelte";

  // (WEB-69) a native modal dialog: it traps focus, takes Escape and gives focus back
  type Command = { label: string; hint: string; run: () => void };
  const go = (route: Route) => () => navigate(route);
  const commands: Command[] = [
    ...TABS.map((tab) => ({ label: tab[0].toUpperCase() + tab.slice(1), hint: "go to view", run: go({ tab }) })),
    { label: "Requirement graph", hint: "go to view", run: go({ tab: "requirements", view: "graph" }) },
    { label: "Spec documents", hint: "go to view", run: go({ tab: "requirements", view: "docs" }) },
    { label: "Rescan the repo", hint: "action: syncs the CodeGraph index first", run: () => { if (!setupState.busy) rescan(); } },
    ...DISPLAY.flatMap((f) => f.options.map((o) => ({ label: `${f.label}: ${o.label.toLowerCase()}`, hint: "display", run: () => setPref(f.key, o.key) }))),
  ];

  let dialog: HTMLDialogElement;
  let query = $state("");
  let active = $state(0);
  let open = $state(false);
  // built only while open: a large repo has thousands of IDs
  const entries = $derived<Command[]>(open ? [...commands, ...saved.list.map((v) => ({ label: `View: ${v.name}`, hint: `saved view on ${v.tab}`, run: () => applyView(v) })), ...data.rows.map((r) => ({ label: r.id, hint: plain(r.text), run: go(req(r.id)) }))] : []);
  const found = $derived(search(entries, query));
  $effect(() => { void found; document.getElementById(`palette-${active}`)?.scrollIntoView({ block: "nearest" }); });
  const run = (c: Command | undefined) => { if (!c) return; dialog.close(); c.run(); };
  const reset = () => { query = ""; active = 0; open = false; };
  function keys(e: KeyboardEvent) {
    const step = e.key === "ArrowDown" ? 1 : e.key === "ArrowUp" ? -1 : 0;
    if (step) { e.preventDefault(); if (found.length) active = (active + step + found.length) % found.length; }
    else if (e.key === "Enter" && !e.isComposing) { e.preventDefault(); run(found[active]); }
  }
</script>

<!-- a click on the backdrop targets the dialog itself -->
<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
<dialog id="palette" bind:this={dialog} aria-label="Command palette" onclose={reset} onclick={(e) => e.target === dialog && dialog.close()} ontoggle={(e) => (open = e.newState === "open")}>
  <div class="box">
    <!-- svelte-ignore a11y_autofocus -->
    <input autofocus role="combobox" aria-expanded="true" aria-controls="palette-list" aria-activedescendant={found.length ? `palette-${active}` : undefined} aria-label="Search views, actions and requirements" placeholder="Search views, actions and requirements…" autocomplete="off" spellcheck="false" bind:value={query} oninput={() => (active = 0)} onkeydown={keys} />
    <ul id="palette-list" role="listbox" aria-label="Results">
      {#each found as c, i (c.label)}
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <li id="palette-{i}" role="option" aria-selected={i === active} onclick={() => run(c)} onpointermove={() => (active = i)}><span class="label">{c.label}</span><span class="hint">{c.hint}</span></li>
      {/each}
    </ul>
    <p class="muted" role="status">{open && !found.length ? "No match" : ""}</p>
  </div>
</dialog>

<style>
  dialog { width: min(40rem, calc(100vw - 16px)); max-height: min(32rem, calc(100dvh - 16px)); margin: 8dvh auto auto; padding: 0; background: var(--card); color: var(--fg); border: 1px solid var(--line-strong); border-radius: var(--radius); box-shadow: var(--shadow); }
  dialog[open] { display: flex; }
  dialog::backdrop { background: rgb(0 0 0 / .4); }
  .box { display: flex; flex-direction: column; width: 100%; min-height: 0; padding: 8px; gap: 8px; }
  ul { margin: 0; padding: 0; list-style: none; overflow: auto; min-height: 0; }
  li { display: flex; align-items: baseline; gap: 12px; min-height: var(--row-h); padding: var(--cell-pad-y) 10px; border-radius: var(--radius); cursor: pointer; font-size: var(--text-label); }
  li[aria-selected="true"] { background: var(--tint-sel); }
  .label { white-space: nowrap; }
  .hint { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--muted); font-size: var(--text-sm); }
  p { margin: 0; padding: 0 10px; font-size: var(--text-sm); }
  /* empty, it stays rendered, so that "No match" is announced; this takes back the gap */
  p:empty { margin-top: -8px; }
</style>

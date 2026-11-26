<script lang="ts">
  import { place } from "$lib/ui/place";
  import { saved, saveView, deleteView, applyView, currentQuery } from "$lib/requirements/viewStore.svelte";

  // (WEB-71) the "Views" button and its menu: apply, delete, and save what the toolbar shows now
  let trigger: HTMLButtonElement;
  let menu: HTMLDivElement;
  let open = $state(false);
  let name = $state("");
  const dirty = $derived(Object.keys(currentQuery()).length > 0);
  const toggled = (e: ToggleEvent) => { open = e.newState === "open"; if (open) place(menu, trigger); };
  const submit = (e: SubmitEvent) => { e.preventDefault(); if (!name.trim() || !dirty) return; saveView(name.trim()); name = ""; };
</script>

<button bind:this={trigger} type="button" popovertarget="saved-views" aria-expanded={open} title="Saved searches and filters">Views</button>
<div bind:this={menu} class="req-options" id="saved-views" popover="auto" role="group" aria-label="Saved views" ontoggle={toggled}>
  {#if saved.list.length}
    <ul>
      {#each saved.list as v (v.name)}
        <li>
          <button type="button" class="apply" onclick={() => { menu.hidePopover(); applyView(v); }}>{v.name}{#if v.tab === "changes"} <span class="muted">on Changes</span>{/if}</button>
          <button type="button" title="Delete {v.name}" aria-label="Delete saved view {v.name}" onclick={() => confirm(`Delete the saved view “${v.name}”?`) && deleteView(v.name)}>×</button>
        </li>
      {/each}
    </ul>
  {:else}<p class="muted">No saved view yet.</p>{/if}
  <form onsubmit={submit}>
    <input name="view-name" aria-label="Name for the current search and filters" placeholder="Name this view" maxlength="60" autocomplete="off" bind:value={name} disabled={!dirty} />
    <button type="submit" disabled={!dirty || !name.trim()}>Save</button>
  </form>
  <p class="muted">{dirty ? "An existing name is replaced." : "Search or filter first, then save it here."}</p>
</div>

<style>
  ul { margin: 0; padding: 0; list-style: none; display: grid; gap: 4px; }
  li, form { display: flex; gap: 6px; }
  .apply { flex: 1; text-align: left; overflow-wrap: anywhere; }
  li button, form button { min-width: 32px; min-height: 32px; }
  form input { flex: 1; min-width: 0; }
  p { margin: 0; font-size: var(--text-sm); }
</style>

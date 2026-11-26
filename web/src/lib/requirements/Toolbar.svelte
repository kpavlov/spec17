<script lang="ts">
  import Seg from "$lib/ui/Seg.svelte";
  import FilterMenu from "$lib/requirements/FilterMenu.svelte";
  import SplitToggle from "$lib/layout/SplitToggle.svelte";
  import ViewSeg from "$lib/requirements/ViewSeg.svelte";
  import SavedViews from "$lib/requirements/SavedViews.svelte";
  import { nav, u, filter } from "$lib/nav.svelte";
  import { filters, facets, facetLabels, facetShortcuts, clearReqFilters, selected, options, actionable, optionLabel, optionTone, pickFacet, clearFacet, editFacet, readQuery, applyQuery } from "$lib/requirements/filterState.svelte";

  // (WEB-59) the filter toolbar Requirements and Changes share
  const route = $derived(nav.route);
  const onChanges = $derived(filters.onChanges);
  const query = $derived(filters.query);
  const hasFilters = $derived(filters.hasFilters);
  const activeReqFilters = $derived(filters.active);
  const filtering = $derived(filters.filtering);
  const shown = $derived(filters.shown);
  const reqFilterRows = $derived(filters.rows);
  const pending = $derived(query.includes(":") && readQuery(query).picked.length > 0);
</script>

<div class="bar req-toolbar">
  <input name="filter" aria-label={onChanges ? "Search affected requirements" : "Search requirements"} placeholder="Search requirements…" title="Search by ID, text or file (/). Enter turns key:value words (state, level, priority, effort, changed, reviewed) into filters, e.g. state:none level:must" value={query} oninput={(e) => filter({ q: e.currentTarget.value })} onkeydown={(e) => e.key === "Enter" && !e.isComposing && applyQuery(e.currentTarget.value)} />
  <FilterMenu active={activeReqFilters} disabled={!hasFilters} title={hasFilters ? "Filter requirements (F)" : "Nothing to filter by: every requirement has the same values"}>
    {#each facets as key}
      {@const values = selected(key)}
      {@const choices = options(key)}
      {#if actionable(key, choices)}
      <div data-facet={key}>
        <Seg label={facetLabels[key]} keycap={facetShortcuts[key]} hint={`Click selects one; Shift, Ctrl or ⌘ + click adds or removes. ${facetShortcuts[key]} for next choice, Shift+${facetShortcuts[key]} for previous`} options={choices} selected={values.length ? values.map((value) => JSON.stringify(value)) : [""]} onpick={(value, additive) => value ? pickFacet(key, JSON.parse(value), additive) : values.length && clearFacet(key)} />
      </div>
      {/if}
    {/each}
  </FilterMenu>
  <SavedViews />
  <span class="found muted nowrap" aria-live="polite">{#if pending}Press Enter to apply the filter{:else}{shown.length} of {reqFilterRows.length}{/if}</span>
  {#if filtering}<a class="clear-filters" href={u({ ...route, ...clearReqFilters })} aria-label="Clear requirement filters" title="Clear requirement filters (0)">Clear</a>{/if}
  {#if route.tab === "requirements"}<ViewSeg />{/if}
  {#if !onChanges && route.id}<SplitToggle />{/if}
</div>
{#if activeReqFilters}
  <div class="filter-chips" role="group" aria-label="Active requirement filters">
    {#each facets as key}
      {#each new Set(selected(key)) as value (value)}
        {@const label = value === null ? `No ${facetLabels[key].toLowerCase()}` : optionLabel(key, value)}
        <span class="filter-chip" data-tone={optionTone(key, value)}>
          <button type="button" title="Edit {facetLabels[key]} filter" aria-label="Edit {facetLabels[key]} filter, {label}" onclick={() => editFacet(key)}>{label}</button>
          <button type="button" title="Remove {label}" aria-label="Remove {label} from {facetLabels[key]} filter" onclick={(e) => {
            const chip = e.currentTarget.parentElement;
            const next = (chip?.nextElementSibling ?? chip?.previousElementSibling)?.querySelector<HTMLButtonElement>("button:last-child");
            pickFacet(key, value);
            (next ?? document.querySelector<HTMLButtonElement>(".req-toolbar .filter-toggle"))?.focus();
          }}>×</button>
        </span>
      {/each}
    {/each}
  </div>
{/if}

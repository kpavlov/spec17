<script lang="ts" generics="T">
  import type { Snippet } from "svelte";
  import { sorted, toggleSort, type Sort } from "$lib/sort";
  import { layout } from "$lib/layout/layout.svelte";
  import { db, run } from "$lib/db.svelte";

  type Col = { key: string; label: string; cls?: string; sort?: boolean };
  // a keyboard stop: a row (by key) or a group header
  type Stop = { key?: string; group?: string };
  let { rows, columns, rowKey, selected, onselect, cell, value, groupOf, tone, keys = true, storageKey = "", cap = 500, headless = false, empty }: {
    rows: T[];
    columns: Col[];
    rowKey: (r: T) => string;
    selected?: string;
    // `key` is undefined when a group header is selected; "key" is the keyboard (replace the URL), "click" a pointer
    onselect: (key: string | undefined, how: "click" | "key") => void;
    cell: Snippet<[T, string]>;
    // sort value of a column; without it the headers do not sort
    value?: (r: T, key: string) => string | number;
    // rows with the same name form a collapsible group, shown when there are several
    groupOf?: (r: T) => string;
    // a Tone (tone.ts): colours the row's left edge
    tone?: (r: T) => string | undefined;
    // ↑/↓/←/→ move the selection (off where selecting does not preview, so the arrows scroll the page)
    keys?: boolean;
    // where collapsed groups are remembered
    storageKey?: string;
    cap?: number;
    headless?: boolean;
    empty?: Snippet;
  } = $props();

  const st = $state<Sort>({ key: "", dir: 1 });
  const groups = $derived.by(() => {
    const out: { name: string; rows: T[] }[] = [];
    for (const r of (value ? sorted(rows, st, value) : rows).slice(0, cap)) {
      const p = groupOf?.(r) ?? "";
      (out.find((g) => g.name === p) ?? out[out.push({ name: p, rows: [] }) - 1]).rows.push(r);
    }
    return out;
  });
  const grouped = $derived(!!groupOf && groups.length > 1);

  // (WEB-13, WEB-52) Collapsed groups are remembered per browser and root (IndexedDB); everything starts expanded.
  let collapsed = $state<string[]>([]);
  // toggles made before the saved groups have loaded (group → open): they win over the saved state, which is written once it is in
  const touched = new Map<string, boolean>();
  let loaded = false;
  const dbKey = () => `${db.root}\n${storageKey}`;
  const save = () => run("collapsed", "readwrite", (st) => st.put($state.snapshot(collapsed), dbKey())).catch(() => { /* storage blocked */ });
  $effect(() => {
    if (!storageKey || db.root === null) return;
    loaded = false;
    run<string[] | undefined>("collapsed", "readonly", (st) => st.get(dbKey()))
      .then((v) => {
        const shut = new Set((v ?? []).filter((g) => !touched.has(g)));
        for (const [g, open] of touched) if (!open) shut.add(g);
        collapsed = [...shut];
      })
      .catch(() => { /* storage blocked */ })
      .finally(() => { const dirty = touched.size > 0; touched.clear(); loaded = true; if (dirty) save(); });
  });
  const isOpen = (p: string) => !grouped || !collapsed.includes(p);
  const setOpen = (p: string, open: boolean) => {
    if (open === isOpen(p)) return;
    collapsed = open ? collapsed.filter((x) => x !== p) : [...collapsed, p];
    if (!storageKey) return;
    if (loaded) save(); else touched.set(p, open);
  };

  // the selection is a row (`selected`) or a group header (`selGroup`, only while no row is selected)
  let selGroup = $state<string | null>(null);
  const curGroup = () => (grouped && selected === undefined && groups.some((g) => g.name === selGroup) ? selGroup : null);
  const go = (s: Stop) => { selGroup = s.group ?? null; onselect(s.key, "key"); };
  const pick = (p: string) => { selGroup = p; onselect(undefined, "key"); setOpen(p, !isOpen(p)); };
  // the arrow-key order: every row of an expanded group, a collapsed group as one stop
  const seq = $derived<Stop[]>(groups.flatMap((g) => (isOpen(g.name) ? g.rows.map((r): Stop => ({ key: rowKey(r) })) : [{ group: g.name } as Stop])));
  // the stop at an end of group `i` (0 first, -1 last); undefined past the ends
  const stopOf = (i: number, end: 0 | -1): Stop | undefined => {
    const g = groups[i];
    return !g ? undefined : isOpen(g.name) ? { key: rowKey(g.rows.at(end)!) } : { group: g.name };
  };

  // ↑/↓ move through the stops, ←/→ collapse and expand the selected group (not while a field or a radio group has the keys)
  function onkey(e: KeyboardEvent) {
    if (!keys || (e.target !== document.body && !table?.contains(e.target as Node)) || (e.target as HTMLElement).closest("input, select, textarea, .gutter, .seg, [popover]") || e.metaKey || e.ctrlKey || e.altKey) return;
    const grp = curGroup();
    if ((e.key === "ArrowDown" || e.key === "ArrowUp") && seq.length) {
      e.preventDefault();
      const down = e.key === "ArrowDown";
      let to: Stop | undefined;
      if (grp) {
        // from a group: the next/previous group's first/last row, or the group itself when collapsed
        const gi = groups.findIndex((g) => g.name === grp);
        to = down && isOpen(grp) ? { key: rowKey(groups[gi].rows[0]) } : down ? stopOf(gi + 1, 0) : stopOf(gi - 1, -1);
      } else {
        const i = seq.findIndex((x) => x.key === selected);
        // up from the first row of a group selects the group itself, down from the last one the next group
        const gi = grouped ? groups.findIndex((g) => rowKey(g.rows.at(down ? -1 : 0)!) === selected) : -1;
        const edge = gi < 0 ? undefined : groups[down ? gi + 1 : gi];
        to = edge ? { group: edge.name } : seq[down ? Math.min(i + 1, seq.length - 1) : i < 0 ? 0 : Math.max(i - 1, 0)];
      }
      if (to) go(to);
    } else if ((e.key === "ArrowLeft" || e.key === "ArrowRight") && grp) {
      e.preventDefault();
      setOpen(grp, e.key === "ArrowRight");
    }
  }

  // centre the selection in whichever ancestor scrolls (the list pane), without moving the page
  let table = $state<HTMLElement>();
  const scroller = (el: HTMLElement | null | undefined) => { while (el && !/auto|scroll/.test(getComputedStyle(el).overflowY)) el = el.parentElement; return el; };
  $effect(() => {
    void [rows.length, selected, selGroup, layout.split];
    queueMicrotask(() => {
      const el = table?.querySelector<HTMLElement>("tr.sel"), pane = scroller(table);
      if (el && pane) pane.scrollTop += el.getBoundingClientRect().top - pane.getBoundingClientRect().top - (pane.clientHeight - el.getBoundingClientRect().height) / 2;
    });
  });
</script>

<svelte:document onkeydown={onkey} />

{#if !rows.length && empty}
  {@render empty()}
{:else}
  <table bind:this={table}>
    {#if !headless}
      <thead>
        <tr>
          {#each columns as c}
            <th class={c.cls} aria-sort={st.key === c.key ? (st.dir === 1 ? "ascending" : "descending") : "none"}>
              {#if value && c.sort !== false}
                <button type="button" class="sort" onclick={() => toggleSort(st, c.key)}>{c.label}<span class="arrow">{st.key === c.key ? (st.dir === 1 ? "▲" : "▼") : ""}</span></button>
              {:else}{c.label}{/if}
            </th>
          {/each}
        </tr>
      </thead>
    {/if}
    <tbody>
      {#each groups as g (g.name)}
        {#if grouped}
          <tr class="grp" class:sel={curGroup() === g.name}><td colspan={columns.length}><button type="button" aria-expanded={isOpen(g.name)} onclick={() => pick(g.name)}><span class="caret">{isOpen(g.name) ? "▾" : "▸"}</span> <b class="mono">{g.name}</b> <span class="count">{g.rows.length}</span></button></td></tr>
        {/if}
        {#if isOpen(g.name)}
          {#each g.rows as r (rowKey(r))}
            <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
            <tr class="row" class:sel={selected === rowKey(r)} data-tone={tone?.(r)} onclick={(e) => { if ((e.target as HTMLElement).closest("a, input, button, label, [popover]")) return; selGroup = null; onselect(rowKey(r), "click"); }}>
              {#each columns as c}<td class={c.cls}>{@render cell(r, c.key)}</td>{/each}
            </tr>
          {/each}
        {/if}
      {/each}
    </tbody>
  </table>
  {#if rows.length > cap}<p class="cap">First {cap} of {rows.length} shown; narrow the filter.</p>{/if}
{/if}

<style>
  thead th { position: sticky; top: 0; z-index: 1; background: var(--bg); }
  tr.row { cursor: pointer; }
  tr.row td:first-child { border-left: 3px solid transparent; }
  tr[data-tone] td:first-child { border-left-color: var(--tone); }
  tr.row:hover { background: var(--tint-hover); }
  tr.row.sel { background: var(--tint-sel); }
  tr.row.sel td:first-child { font-weight: var(--weight-semibold); }
  tr.grp td { padding: 0; background: var(--tint-group); }
  tr.grp button { all: unset; box-sizing: border-box; width: 100%; padding: 6px 10px; cursor: pointer; display: flex; gap: 8px; align-items: baseline; }
  tr.grp button:focus-visible { outline: 2px solid var(--accent); outline-offset: -2px; }
  tr.grp .caret { width: 10px; color: var(--muted); }
  tr.grp .count { color: var(--muted); }
  .cap { margin: 10px; font-size: var(--text-sm); color: var(--muted); }
</style>

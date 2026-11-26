// The requirement facets, shared by the Requirements and Changes pages and by the filter keys. The URL holds the selection.
import { nav, navigate } from "$lib/nav.svelte";
import { data, isDirect, isReviewed, type Row } from "$lib/data.svelte";
import { readValues, writeValues, pickValue, collapseAll, matches, type Value } from "$lib/requirements/filters";
import { requirementLevelTone, priorityTone, effortTone, stateTone, stateIcon, stateLabel, STATE_HINT } from "$lib/tone";

export const states = ["all", "none", "src-only", "tested", "manual", "later", "ref"];
const levels = ["MUST", "SHOULD", "MAY"];
export const clearReqFilters = { state: undefined, level: undefined, priority: undefined, effort: undefined, q: undefined, changed: undefined, reviewed: undefined };
// (WEB-59) each facet ignores its own selection when counting choices.
export type Facet = "state" | "level" | "priority" | "effort" | "changed" | "reviewed";
export const facets: Facet[] = ["state", "level", "priority", "effort", "changed", "reviewed"];
export const facetLabels: Record<Facet, string> = { state: "State", level: "Level", priority: "Priority", effort: "Effort", changed: "Changes", reviewed: "Reviewed" };
export const facetShortcuts: Record<Facet, string> = { state: "S", level: "L", priority: "P", effort: "E", changed: "C", reviewed: "R" };

export const selected = (key: Facet) => key === "state" && nav.route.state === "all" ? [] : readValues(nav.route[key]);
const levelOf = (r: Row) => levels.includes(r.level) ? r.level : "other";
const facetValue = (r: Row, key: Facet): Value => key === "level" ? levelOf(r) : key === "changed" ? (filters.onChanges ? isDirect(r) : data.reqKinds.has(r.id)) ? "1" : "0" : key === "reviewed" ? isReviewed(r.id) ? "1" : "0" : r[key] ?? null;
const facet = (skip?: Facet) => filters.rows.filter((r) => facets.every((key) => key === skip || matches(selected(key), facetValue(r, key))) && (!filters.query || `${r.id} ${r.text} ${r.file}`.toLowerCase().includes(filters.query.toLowerCase())));

class Filters {
  stateFilter = $derived(nav.route.state ?? "all");
  query = $derived(nav.route.q ?? "");
  onChanges = $derived(nav.route.tab === "changes");
  onlyChanged = $derived(nav.route.changed === "1");
  // (WEB-57) change records include their own metadata, even when the status list is truncated
  rows = $derived<Row[]>(this.onChanges ? data.view?.requirements ?? [] : data.rows);
  shown = $derived<Row[]>(facet());
  stateKeys = $derived(states.filter((st) => st === "all" || selected("state").includes(st) || this.rows.some((r) => r.state === st)));
  active = $derived(facets.filter((key) => selected(key).length).length);
  filtering = $derived(!!this.active || !!this.query);
  // (WEB-61) with no field to show, "+ Filter" is disabled and F does nothing
  hasFilters: boolean = $derived(facets.some((key) => actionable(key)));
}
export const filters = new Filters();

export const optionLabel = (key: Facet, v: Value) => v === null ? "Not set" : key === "state" ? stateLabel(v) : key === "level" ? v.toLowerCase() : key === "changed" ? filters.onChanges ? v === "1" ? "directly changed" : "reached through tests" : v === "1" ? "changed" : "unchanged" : key === "reviewed" ? v === "1" ? "reviewed" : "unreviewed" : v;
export const optionTone = (key: Facet, value: Value) => value === null ? undefined : key === "state" ? stateTone[value] : key === "level" ? requirementLevelTone[value.toUpperCase()] : key === "priority" ? priorityTone(value) : key === "effort" ? effortTone(value) : undefined;
export const options = (key: Facet) => {
  const present = new Set(filters.rows.map((row) => facetValue(row, key)));
  const order = key === "state" ? states.slice(1) : key === "level" ? [...levels, "other"] : ["1", "0"];
  const values = [...new Set([...present, ...selected(key)])];
  values.sort((a, b) => {
    if (a === null) return 1;
    if (b === null) return -1;
    if (key !== "effort" && key !== "priority") {
      const rank = (value: string) => order.includes(value) ? order.indexOf(value) : order.length;
      return rank(a) - rank(b);
    }
    return a.localeCompare(b, undefined, { numeric: true });
  });
  const eligible = facet(key);
  const counts = new Map<Value, number>();
  for (const row of eligible) {
    const value = facetValue(row, key);
    counts.set(value, (counts.get(value) ?? 0) + 1);
  }
  return [{ key: "", label: "All", count: eligible.length }, ...values.map((value) => ({ key: JSON.stringify(value), label: optionLabel(key, value), count: counts.get(value) ?? 0, tone: optionTone(key, value), icon: key === "state" && value ? stateIcon[value as keyof typeof stateIcon] : undefined, title: key === "state" && value ? STATE_HINT[value] : undefined }))];
};
// (WEB-61) a field is shown while it offers a choice (All plus two values) or holds a selection to clear; its shortcut is the same
export const actionable = (key: Facet, choices = options(key)) => choices.length > 2 || selected(key).length > 0;
// (WEB-60) a plain click selects only that value; a modifier click (and a chip's ×) adds or removes it; every value selected is All
export const pickFacet = (key: Facet, value: Value, additive = true) => navigate({ ...nav.route, [key]: writeValues(collapseAll(pickValue(selected(key), value, additive), options(key).slice(1).map((o) => JSON.parse(o.key)))) });
export const clearFacet = (key: Facet) => navigate({ ...nav.route, [key]: undefined });
// (WEB-59) open the menu and focus a control in it
export const openFilterMenu = (focus = "button") => { const menu = document.getElementById("filter-menu"); menu?.showPopover(); menu?.querySelector<HTMLElement>(focus)?.focus(); };
export const editFacet = (key: Facet) => openFilterMenu(`[data-facet="${key}"] button[aria-pressed=true]`);

// the "changed only" switch of the Documents, Modules and Tests pages (WEB-22)
export const changedSeg = (n: number) => [{ key: "all", label: "all" }, { key: "changed", label: "changed", count: n }];

<script lang="ts">
  import LevelPill from "./LevelPill.svelte";
  import StatePill from "./StatePill.svelte";
  import HelpPill from "$lib/ui/HelpPill.svelte";
  import { priorityTone, effortTone } from "$lib/tone";

  // level and evidence state of one requirement; the same pair on every page that shows it
  let { level, state: evidence, priority, effort, deleted = false }: { level?: string; state?: string; priority?: string | null; effort?: string | null; deleted?: boolean } = $props();
</script>

<div class="metadata" role="group" aria-label="Requirement metadata">
  {#if level}<LevelPill {level} />{/if}
  {#if evidence}<StatePill state={evidence} {deleted} />{/if}
  {#if priority}<HelpPill label={priority} title={`Priority: ${priority} — detected hint`} tone={priorityTone(priority)} />{/if}
  {#if effort}<HelpPill label={effort} title={`Effort: ${effort} — detected hint`} tone={effortTone(effort)} />{/if}
</div>

<style>
  .metadata { display: inline-flex; flex-wrap: wrap; gap: 4px; vertical-align: middle; --badge-padding: 1px 6px; }
</style>

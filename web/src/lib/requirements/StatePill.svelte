<script lang="ts">
  import HelpPill from "$lib/ui/HelpPill.svelte";
  import { stateTone, stateIcon, stateLabel, STATE_HINT } from "$lib/tone";

  // the one evidence-state pill (none, src-only, tagged test, …): same look and tooltip on every page
  let { state: evidence, deleted = false }: { state: string; deleted?: boolean } = $props();
  const shownAs = $derived(deleted ? "deleted" : evidence);
  const title = $derived(`Evidence: ${stateLabel(shownAs)}${STATE_HINT[shownAs] ? " — " + STATE_HINT[shownAs] : ""}`);
</script>

<HelpPill label={stateLabel(shownAs)} icon={stateIcon[shownAs as keyof typeof stateIcon] ?? "question"} {title} tone={stateTone[shownAs]} />

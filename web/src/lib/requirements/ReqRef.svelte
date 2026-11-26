<script lang="ts">
  import ReqLink from "$lib/requirements/ReqLink.svelte";
  import RequirementBadges from "$lib/requirements/RequirementBadges.svelte";
  import { data } from "$lib/data.svelte";
  import type { Route } from "$lib/router";

  // a requirement link followed by its badges, when a spec defines it
  let { id, to }: { id: string; to: Route } = $props();
  const row = $derived(data.byId.get(id));
</script>

<ReqLink {id} {to} />{#if row}{" "}<RequirementBadges level={row.level} state={row.state} priority={row.priority} effort={row.effort} />{/if}

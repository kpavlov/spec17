<script lang="ts">
  import RequirementLink from "$lib/requirements/RequirementLink.svelte";
  import { u } from "$lib/nav.svelte";
  import { requirementText } from "$lib/data.svelte";
  import type { Route } from "$lib/router";

  // (WEB-45) the requirement link with its text preview, for a route
  let { id, to }: { id: string; to: Route } = $props();
</script>

<RequirementLink {id} href={u(to)} getText={requirementText} />

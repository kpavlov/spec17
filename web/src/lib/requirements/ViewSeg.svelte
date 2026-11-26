<script lang="ts">
  import Seg from "$lib/ui/Seg.svelte";
  import { nav, navigate, reqView } from "$lib/nav.svelte";
  import { save } from "$lib/store";

  const VIEWS = [{ key: "list", label: "list" }, { key: "graph", label: "graph" }, { key: "docs", label: "documents" }];
  const route = $derived(nav.route);
</script>

<!-- (WEB-49, WEB-51) -->
<Seg label="View" nowrap options={VIEWS} value={route.view ?? "list"} onpick={(k) => { save("spec17.requirements", (reqView.value = k)); navigate({ ...route, view: k === "list" ? undefined : k, file: undefined, id: undefined }); }} />

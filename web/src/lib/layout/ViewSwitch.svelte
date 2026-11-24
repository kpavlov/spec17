<script lang="ts">
  import Seg from "$lib/ui/Seg.svelte";
  import { preview, setView } from "./layout.svelte";
  // actual / diff for the preview of a changed item; the choice is remembered (layout.svelte.ts)
  let { n = 1 }: { n?: number } = $props();
</script>

<Seg label="View" nowrap hint="Show the item as it is, or what changed against HEAD" options={[{ key: "actual", label: "actual" }, { key: "diff", label: n > 1 ? `diff · ${n} files` : "diff" }]} value={preview.view} onpick={setView} />

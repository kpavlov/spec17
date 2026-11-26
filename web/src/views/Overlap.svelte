<script lang="ts">
  import ReqRef from "$lib/requirements/ReqRef.svelte";
  import TracePanel from "$lib/views/TracePanel.svelte";
  import { u, withId } from "$lib/nav.svelte";
  import { data } from "$lib/data.svelte";

  const overlap = $derived(data.overlap);
</script>

{#if overlap === null}<p class="muted">Loading…</p>
{:else if overlap.length === 0}<p class="muted">No declaration is covered by two or more requirements (or no CodeGraph index).</p>
{:else}
  {#each overlap as r}
    <div class="card block">
      <div class="mono nowrap"><a href={u({ tab: "code", path: r.file, line: r.line })}>{r.file}:{r.line}</a> <b>{r.name}</b></div>
      {#each r.pairs as [a, b, linked]}
        <div><ReqRef id={a} to={withId("overlap", a)} /> {linked ? "~ linked" : "? UNLINKED"} <ReqRef id={b} to={withId("overlap", b)} /></div>
      {/each}
    </div>
  {/each}
  <TracePanel float />
{/if}

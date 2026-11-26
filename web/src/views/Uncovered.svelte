<script lang="ts">
  import { u } from "$lib/nav.svelte";
  import { data } from "$lib/data.svelte";

  const uncovered = $derived(data.uncovered);
</script>

{#if uncovered === null}<p class="muted">Loading…</p>
{:else if !uncovered.files}<p class="muted">Not available (no CodeGraph index).</p>
{:else}
  <p>{uncovered.units} units / {uncovered.lines} lines in {uncovered.files.length} files have no requirement. Candidates, not proof.</p>
  <table>
    <tbody>
      {#each uncovered.files as f}
        <tr>
          <!-- f.file is shown relative to the app folder; the viewer needs the repo-relative path every unit carries -->
          <td class="mono"><a href={u({ tab: "code", path: f.units[0].file })}>{f.file}</a></td>
          <td>{f.lines} lines</td>
          <td class="mono muted">{#each f.units as un}<a href={u({ tab: "code", path: un.file, line: un.lo })}>{un.name}:{un.lo}-{un.hi}</a>{" "}{/each}</td>
        </tr>
      {/each}
    </tbody>
  </table>
{/if}

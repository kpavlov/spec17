<script lang="ts">
  import Seg from "$lib/ui/Seg.svelte";
  import { prefs, setPref, DISPLAY } from "$lib/prefs.svelte";

  // (WEB-68)
</script>

<section class="card" aria-labelledby="display-title">
  <h2 id="display-title">Display</h2>
  {#each DISPLAY as f (f.key)}
    <div><Seg label={f.label} options={f.options} value={prefs[f.key]} onpick={(v) => setPref(f.key, v)} /></div>
  {/each}
  <p class="muted">Remembered in this browser. High legibility uses <a href="https://fonts.google.com/specimen/Atkinson+Hyperlegible+Next" target="_blank" rel="noopener noreferrer">Atkinson Hyperlegible Next</a> if it is installed on this computer, otherwise the system font: spec17 does not bundle or download it.</p>
</section>

<style>
  section { max-width: 64rem; margin: var(--gap) auto 0; display: grid; gap: var(--gap); }
  h2, p { margin: 0; }
  p { font-size: var(--text-sm); }
</style>

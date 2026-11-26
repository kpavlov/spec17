<script lang="ts">
  import DiffView from "$lib/views/DiffView.svelte";
  import ViewSwitch from "$lib/layout/ViewSwitch.svelte";
  import { preview } from "$lib/layout/layout.svelte";
  import { EDITORS } from "$lib/editors";
  import { stored, save } from "$lib/store";
  import { nav, u } from "$lib/nav.svelte";
  import { data } from "$lib/data.svelte";
  import { codeLines } from "$lib/lines";

  const route = $derived(nav.route);
  const code = $derived(data.code);
  const root = $derived(data.root);
  const gen = $derived(data.gen);
  const fileOf = $derived(data.fileOf);
  const lines = $derived(codeLines(data.code));
  const diffOn = (changed: boolean) => changed && preview.view === "diff";
  // code viewer: wrap, copy the path, open in an editor (URL scheme of the chosen IDE, remembered per browser)
  const savedEditor = stored("spec17.editor");
  let editor = $state(savedEditor in EDITORS ? savedEditor : "IntelliJ");
  let wrap = $state(stored("spec17.wrap") === "1");
  let copied = $state(false);
  const copyPath = (path: string) => navigator.clipboard.writeText(path).then(() => { copied = true; setTimeout(() => (copied = false), 1200); }).catch(() => {});
</script>

{#if !route.path}<p class="muted">No file selected: open one from a requirement's tests or citations.</p>
{:else if !code || code.path !== route.path}<p class="muted">Loading {route.path}…</p>
{:else}
  <p class="mono crumb">
    <span class="nowrap"><a href="/requirements">requirements</a> / {code.path}{route.line ? `:${route.line}` : ""}</span>
    <button class="small" title="Copy the file path" onclick={() => copyPath(code!.path)}>{copied ? "Copied" : "Copy path"}</button>
    {#if root}<a class="btn small" title="Open in {editor}" href={EDITORS[editor](`${root}/${code.path}`, route.line ?? 1)}>Open in</a>{/if}
    <select class="small" bind:value={editor} onchange={() => save("spec17.editor", editor)} aria-label="Editor">{#each Object.keys(EDITORS) as e}<option>{e}</option>{/each}</select>
    <button class="small" class:on={wrap} aria-pressed={wrap} onclick={() => { wrap = !wrap; save("spec17.wrap", wrap ? "1" : "0"); }}>Wrap</button>
    {#if fileOf.has(code.path)}<ViewSwitch />{/if}
  </p>
  {#if diffOn(fileOf.has(code.path))}<DiffView paths={[code.path]} {gen} />
  {:else}
  <table class="code" class:wrap>
    <tbody>
      {#each lines as l, i}
        <tr id={`L${i + 1}`} class:hl={route.line === i + 1}>
          <td class="ln"><a href={u({ tab: "code", path: code.path, line: i + 1 })}>{i + 1}</a></td>
          <td class="src">{@html l}</td>
        </tr>
      {/each}
    </tbody>
  </table>
  {/if}
{/if}

<style>
  .crumb .small { font-size: var(--text-sm); padding: 2px 8px; }
  a.btn { background: var(--raised); color: var(--fg); border-radius: var(--radius); text-decoration: none; }
  a.btn:hover { background: var(--tint-btn); color: var(--accent); }
</style>

<script lang="ts">
  import Icon from "$lib/ui/Icon.svelte";
  import Pill from "$lib/ui/Pill.svelte";
  import Seg from "$lib/ui/Seg.svelte";
  import { levelTone } from "$lib/tone";
  import { plural } from "$lib/store";
  import { DOCS as docs } from "$lib/nav.svelte";
  import { data } from "$lib/data.svelte";
  import { setupState, loadSetup, runIndex, runSetup } from "$lib/setup.svelte";

  const setup = $derived(setupState.setup);
  const setupError = $derived(setupState.setupError);
  const checks = $derived(setupState.checks);
  const checking = $derived(setupState.checking);
  const busy = $derived(setupState.busy);
  const indexAction = $derived(setupState.indexAction);
  const indexLog = $derived(setupState.indexLog);
  const globalScope = $derived(setupState.globalScope);
  const noReqs = $derived(data.noReqs);

  function launchCommand(launch?: { command: string; args: string[] } | null) {
    if (!launch) return undefined;
    return [launch.command, ...launch.args].map((arg) => /^[\w./:-]+$/.test(arg) ? arg : "'" + arg.replaceAll("'", "'\"'\"'") + "'").join(" ");
  }
</script>

    {#if !setup}
      {#if setupError}<p class="error">Could not check the setup: {setupError} <button onclick={loadSetup}>Retry</button></p>{:else}<p class="muted">Checking…</p>{/if}
    {:else}
      {@const cgBin = checks.find((k) => k.name === "codegraph binary")}
      {@const cgIdx = checks.find((k) => k.name === "codegraph index")}
      {@const cgMcp = setup.status.project.codegraph_mcp === "ok" ? "this repo" : setup.status.global.codegraph_mcp === "ok" ? "global" : ""}
      {@const passed = checks.filter(k => k.level === "ok")}
      {@const issues = checks.filter(k => k.level === "warn" || k.level === "fail")}
      {@const where = globalScope ? setup.status.global : setup.status.project}
      {@const scope = globalScope ? "Global" : "This repo"}
      {@const fails = issues.filter((k) => k.level === "fail").length}
      {@const tone = fails ? "bad" : issues.length ? "warn" : "ok"}
      <!-- (WEB-44, WEB-42) -->
      <div class="setup-page">
      <section class="card setup health" data-tone={tone} aria-labelledby="health-title">
        <div class="health-head">
          <span class="health-ico"><Icon name={fails ? "x-circle" : issues.length ? "alert" : "check-circle"} /></span>
          <div>
            <div class="eyebrow muted">Health{#if setup.version} · spec17 {setup.version}{/if}</div>
            <h2 id="health-title" role="status">{fails ? plural(fails, "check") + " failed" : issues.length ? plural(issues.length, "warning") : checks.length ? "All checks passed" : "No checks run yet"}</h2>
            <div class="muted">{passed.length} of {passed.length + issues.length} checks passed{#if issues.length} · {plural(issues.length, "issue")}{/if}</div>
          </div>
          <button class="end" disabled={checking || busy} onclick={loadSetup}>{checking ? "Checking…" : "Run checks"}</button>
        </div>
        {#if setupError}<p class="error" role="alert">Could not refresh checks: {setupError}</p>{/if}
        {#snippet checkRow(k: (typeof checks)[number])}
          <div class="check-row">
            <Pill tone={levelTone(k.level)}>{k.level === "ok" ? "Passed" : k.level === "warn" ? "Warning" : k.level === "info" ? "Info" : "Failed"}</Pill>
            <div><strong>{k.name}</strong> · {k.detail}{#if k.hint}<div class="muted">{k.hint}</div>{/if}</div>
          </div>
        {/snippet}
        {#each checks.filter((k) => k.level !== "ok") as k}{@render checkRow(k)}{/each}
        {#if passed.length}
          <details><summary>{issues.length ? `${plural(passed.length, "check")} passed` : "Show the checks"}</summary>
            {#each passed as k}{@render checkRow(k)}{/each}
          </details>
        {:else if !issues.length}<p class="muted">No check results available.</p>{/if}
      </section>
      <section class="card setup">
        <div class="setup-head">
          <h2>Agent setup</h2>
          <Pill tone={setup.suggestions.length ? "warn" : "ok"}>{setup.suggestions.length ? "Needs attention" : "Ready"}</Pill>
          <div class="end"><Seg label="Scope" options={[{key: "repo", label: "This repo"}, {key: "global", label: "Global"}]} value={globalScope ? "global" : "repo"} onpick={(k) => setupState.globalScope = k === "global"} /></div>
        </div>
        {#each setup.suggestions as suggestion}<p>{suggestion}</p>{/each}
        <!-- (WEB-38, WEB-40, WEB-41) -->
        {#snippet item(name: string, state: string, path: string, transport?: "stdio" | "http", connection?: string | null)}
          {@const status = state === "current" || state === "ok" ? "Installed" : state === "missing" ? "Not configured" : state === "other" ? "Custom entry" : state}
          {@const options = transport ? { only: "mcp" as const, transport } : { only: "skill" as const }}
          <div class="setup-item">
            <div>
              <strong>{name}</strong> <Pill tone={status === "Installed" ? "ok" : status === "Not configured" ? "muted" : "warn"}>{status}</Pill>
              {#if transport}<span class="muted transport-note">{transport === "stdio" ? "Starts a process per session" : "Connects to a running server"}</span>{/if}
              <div><code>{path}</code></div>
              {#if connection}<div><code>{connection}</code></div>{/if}
            </div>
            <div class="actions">
              {#if state === "missing" || state === "outdated"}
                <button disabled={busy} aria-label="{state === 'outdated' ? 'Update' : 'Install'} {name} ({scope})" onclick={() => runSetup("install", globalScope, options)}>{state === "outdated" ? "Update" : "Install"}</button>
              {/if}
              {#if state !== "missing"}
                <button disabled={busy} aria-label="Remove {name} ({scope})" onclick={() => confirm(`Remove ${name} (${scope}) from ${path}?`) && runSetup("uninstall", globalScope, options)}>Remove</button>
              {/if}
            </div>
          </div>
        {/snippet}
        {@render item("Agent skill", where.skill, where.skill_path)}
        {#if where.mcp_stdio !== undefined}
          {@render item("Stdio MCP", where.mcp_stdio, where.mcp_path, "stdio", launchCommand(where.mcp_details?.stdio))}
          {@render item("HTTP MCP", globalScope ? where.mcp_file : where.mcp, where.mcp_path, "http", where.mcp_details?.http_url)}
        {:else}<p>Update spec17 and restart the server to manage MCP transports.</p>{/if}
        <p class="muted"><a href="{docs}/agents/" target="_blank" rel="noopener noreferrer">Agent docs</a>: connect a client, the tools it gets.</p>
        {#if globalScope && where.mcp !== "missing"}
          <p class="muted">Claude user MCP entry {where.mcp === "ok" ? "installed" : "customised"} · <code>~/.claude.json</code> (read-only)</p>
        {/if}
      {#if noReqs}
        <section>
          <h3>No specs yet</h3>
          <p>spec17 found no requirements in this repo and works without them: Modules and Tests still list its sources and tests. {setup.suggestions.length ? "Finish the agent setup, then ask" : "Ask"} your agent to draft <code>specs/01-name.md</code> with you (lines like <code>- **ABC-12** MUST — text</code>). <a href="{docs}/specs/#requirement-styles" target="_blank" rel="noopener noreferrer">Requirement formats</a>.</p>
        </section>
      {/if}
      </section>
      <section class="card setup">
        <div class="setup-head"><h2>CodeGraph</h2><Pill tone="muted">Optional</Pill>
          {#if cgBin?.level === "ok"}
            {#if indexAction === "init"}<button class="end" disabled={busy} onclick={() => runIndex("init")}>Create index</button>
            {:else if cgIdx?.level === "ok" || indexAction === "sync"}<button class="end" disabled={busy} onclick={() => runIndex("sync")}>Sync index</button>{/if}
          {/if}
        </div>
        {#if indexLog}<pre class="log" aria-live="polite">{indexLog}</pre>{/if}
        <details open={cgBin?.level !== "ok" || cgIdx?.level !== "ok" || !cgMcp}>
          <summary>{cgBin?.level === "ok" ? "Installed" : "Installation needed"} · {cgMcp ? `Connected (${cgMcp})` : "Agent disconnected"} · {cgIdx?.level === "ok" ? "Index current" : "Check index"}</summary>
          <p class="muted">Enables test-to-code links, overlap and uncovered-code analysis. <a href="{docs}/usage/#codegraph" target="_blank" rel="noopener noreferrer">How spec17 uses it</a>.</p>
          {#if cgBin}<p><strong>Installation</strong> · {cgBin.detail}</p>{/if}
        {#if cgBin && cgBin.level !== "ok"}
          <pre class="log">curl -fsSL https://raw.githubusercontent.com/colbymchenry/codegraph/main/install.sh | sh
npm i -g @colbymchenry/codegraph
irm https://raw.githubusercontent.com/colbymchenry/codegraph/main/install.ps1 | iex   # Windows PowerShell</pre>
          <p class="muted">After installing, select Run checks in Health. <a href="https://colbymchenry.github.io/codegraph/getting-started/installation/" target="_blank" rel="noreferrer">Installation docs</a>. spec17 never installs it for you.</p>
        {/if}
        <p>
          <strong>Agent connection</strong> ·
          {#if setup.status.project.codegraph_mcp === "ok"}
            <button disabled={busy} onclick={() => confirm("Remove the CodeGraph server from this repo’s .mcp.json?") && runSetup("uninstall", false, { server: "codegraph" })}>Remove from this repo</button>
          {:else}
            <button disabled={busy} onclick={() => runSetup("install", false, { server: "codegraph" })}>Connect this repo</button>
          {/if}
        </p>
        {#if cgIdx}<p><strong>Index</strong> · {cgIdx.detail}</p>{/if}
        </details>
      </section>
      </div>
    {/if}

<style>
  .error { margin: 12px var(--gutter); padding: 10px 14px; border: 1px solid var(--bad); background: color-mix(in srgb, var(--bad) 10%, var(--card)); color: var(--bad); border-radius: var(--radius); }
  .setup-page { display: grid; gap: var(--gap); max-width: 64rem; margin-inline: auto; }
  .setup { min-width: 0; overflow-wrap: anywhere; }
  .setup-head, .actions { display: flex; align-items: center; flex-wrap: wrap; gap: var(--gap); }
  .setup-head h2 { margin: 0; }
  /* Health: the verdict first, tinted by its tone (green when every check passed) */
  .health { border-color: color-mix(in srgb, var(--tone) 45%, var(--line)); background: color-mix(in srgb, var(--tone) 9%, var(--card)); }
  .health-head { display: flex; align-items: center; flex-wrap: wrap; gap: 14px; }
  .health-head h2 { margin: 0; font-size: var(--text-title); color: var(--tone); }
  .health-head .end { margin-inline-start: auto; }
  .health-ico { display: flex; font-size: 2rem; color: var(--tone); }
  .eyebrow { font-size: var(--text-sm); }
  .setup button { font-size: var(--text-label); }
  .setup strong { font-weight: var(--weight-semibold); }
  .setup-head .end { margin-inline-start: auto; }
  .setup p, .setup details { margin: 6px 0; }
  .setup summary { cursor: pointer; }
  .setup-item, .check-row { display: grid; align-items: baseline; gap: var(--gap); padding-block: 8px; border-top: 1px solid var(--line); }
  .setup-item { grid-template-columns: minmax(0, 1fr) auto; margin-top: 4px; }
  .setup-item code, .transport-note, .check-row .muted { color: var(--muted); font-size: var(--text-sm); }
  .check-row { grid-template-columns: auto minmax(0, 1fr); }
  .setup .log { overflow-x: auto; }
</style>

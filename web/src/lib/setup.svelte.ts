// What `doctor` and `install_status` said, and the actions of the Settings page.
import { call } from "$lib/transport";
import { data, guard, load } from "$lib/data.svelte";

class Setup {
  setup = $state<{ status: any; suggestions: string[]; text: string; version?: string } | null>(null);
  globalScope = $state(false);
  checking = $state(false);
  setupError = $state("");
  busy = $state(false);
  checks = $state<{ name: string; level: string; detail: string; hint: string | null }[]>([]);
  // the Settings button flags a failed check (red) or an unfinished install (amber)
  setupBadge = $derived(this.checks.some((k) => k.level === "fail") ? "bad" : this.checks.some((k) => k.level === "warn") || this.setup?.suggestions.length ? "warn" : "");
  indexAction = $state<"init" | "sync" | null>(null);
  indexLog = $state("");
}
export const setupState = new Setup();

// (WEB-27) Setup needs only these two calls, never the scan; a failure is shown there with a retry, not left at "Checking…"
export async function loadSetup() {
  setupState.checking = true;
  try {
    const [r, d] = await Promise.all([call("install_status"), call("doctor")]);
    setupState.setup = { ...r.data, text: r.text };
    setupState.checks = d.data.checks;
    setupState.indexAction = d.data.index_action;
    setupState.setupError = "";
  } catch (e: any) {
    setupState.setupError = String(e.message ?? e);
  } finally {
    setupState.checking = false;
  }
}

// (WEB-30) the rescan button: sync the index (`refresh` syncs even beside a running codegraph), then reload; a failed sync is shown, the reload still happens
export async function rescan() {
  setupState.busy = true;
  let sync = "";
  try { await call("codegraph", { action: "refresh" }); } catch (e: any) { sync = String(e.message ?? e); }
  await Promise.allSettled([load(), loadSetup()]);
  if (sync) data.error = sync;
  setupState.busy = false;
}

// the only thing here that indexes: runs when the user clicks, never on its own
export async function runIndex(action: "init" | "sync") {
  if (!confirm(action === "init" ? "Run `codegraph init` in this repo? It creates .codegraph/ and can take a while." : "Run `codegraph sync` to refresh the index?")) return;
  setupState.busy = true;
  setupState.indexLog = `codegraph ${action} running…`;
  try {
    setupState.indexLog = (await call("codegraph", { action })).text;
    await Promise.all([loadSetup(), load()]);
  } catch (e: any) {
    setupState.indexLog = String(e.message ?? e);
  } finally {
    setupState.busy = false;
  }
}

// `server: "codegraph"` (WEB-29) only adds or removes the CodeGraph entry of .mcp.json
export async function runSetup(tool: "install" | "uninstall", global = false, options: { server?: "codegraph"; only?: "skill" | "mcp"; transport?: "stdio" | "http" } = {}) {
  setupState.busy = true;
  try {
    await guard(call(tool, { global, ...options }));
    await loadSetup(); // the status and the checks name what is still missing
  } finally {
    setupState.busy = false;
  }
}

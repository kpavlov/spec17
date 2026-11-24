#!/usr/bin/env node
// Retakes the docs screenshots in docs/static/img from a deliberately broken copy of this repo's working tree:
//   1. breaks specs/02-server.md, server/src/main.rs, server/src/ops.rs and core/src/report.rs (dashboard problems, plus
//      an edited and an added requirement for the Changes view),
//   2. serves the release binary on a free port and drives headless Chrome over CDP (no npm dependencies),
//   3. compresses the PNGs (oxipng from ImageOptim.app or PATH), restores those files, also on error or Ctrl-C.
// Usage: node scripts/screenshots.mjs [--no-build]   (make screenshots)   CHROME=/path/to/chrome overrides the browser.
// Needs Node >= 22, Chrome or Chromium, a current CodeGraph index, pnpm and cargo (unless --no-build).
import { execFileSync, spawn } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync, writeFileSync, existsSync } from "node:fs";
import { createServer } from "node:net";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const img = join(root, "docs/static/img");
const bin = join(root, "target/release/spec17");
const FILES = ["specs/02-server.md", "server/src/main.rs", "server/src/ops.rs", "core/src/report.rs"];
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const run = (cmd, args, cwd = root) => execFileSync(cmd, args, { cwd, stdio: "inherit" });

/** Replace `from` by `to` once in `file`; a miss means the repo moved on and this script must follow. */
function edit(file, from, to) {
  const p = join(root, file);
  const s = readFileSync(p, "utf8");
  if (!s.includes(from)) throw new Error(`${file}: anchor not found: ${from}`);
  writeFileSync(p, s.replace(from, to));
}

function breakThings() {
  // API-02 edited, API-90 added (unimplemented), API-03 twice (spec issue), API-91 check-marked without a test
  edit("specs/02-server.md", "(DNS rebinding).", "(DNS rebinding), and log each refused request without echoing its headers.");
  const api02 = readFileSync(join(root, "specs/02-server.md"), "utf8").split("\n").find((l) => l.startsWith("- **API-02**"));
  edit("specs/02-server.md", api02, `${api02}\n- **API-90** MUST — rate-limit \`/mcp\` per client so a runaway agent loop cannot starve the dashboard.`);
  const api03 = readFileSync(join(root, "specs/02-server.md"), "utf8").split("\n").find((l) => l.startsWith("- **API-03**"));
  edit("specs/02-server.md", api03, `${api03}\n${api03.replace("read spec", "also read spec")}\n- **API-91** SHOULD ✅ — drop idle MCP sessions after 15 minutes.`);
  // a test tagged with an unknown ID (API-07 loses its test), a source citing an unknown ID
  edit("server/src/main.rs", '#[doc = "[API-07]"]', '#[doc = "[API-97]"]');
  edit("server/src/ops.rs", "// (API-03)\n", "// (API-03) (API-77)\n");
  // Cross-spec citations in one declaration create an overlap; the index already has uncovered declarations.
  edit("core/src/report.rs", "// (SCAN-36) Comment tags", "// (SCAN-36, API-02) Comment tags");
}

const originals = new Map();
const restore = () => { for (const [file, content] of originals) writeFileSync(join(root, file), content); };

const freePort = () =>
  new Promise((res, rej) => {
    const s = createServer().once("error", rej).listen(0, "127.0.0.1", () => {
      const { port } = s.address();
      s.close(() => res(port));
    });
  });

function chromePath() {
  const c = [process.env.CHROME, "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome", "/Applications/Chromium.app/Contents/MacOS/Chromium", "google-chrome", "google-chrome-stable", "chromium", "chromium-browser"].filter(Boolean);
  return c.find((p) => !p.includes("/") || existsSync(p)) ?? c[0];
}

/** One headless Chrome tab driven over the DevTools protocol. */
async function openBrowser(dir) {
  const proc = spawn(chromePath(), ["--headless=new", "--remote-debugging-port=0", `--user-data-dir=${dir}`, "--no-first-run", "--hide-scrollbars", "about:blank"], { stdio: ["ignore", "ignore", "pipe"] });
  const port = await new Promise((res, rej) => {
    let buf = "";
    proc.on("error", rej);
    proc.on("exit", () => rej(new Error("chrome exited before it was ready")));
    proc.stderr.on("data", (d) => {
      buf += d;
      const m = buf.match(/DevTools listening on ws:\/\/127\.0\.0\.1:(\d+)/);
      if (m) res(m[1]);
    });
  });
  const tab = await (await fetch(`http://127.0.0.1:${port}/json/new?about:blank`, { method: "PUT" })).json();
  const ws = new WebSocket(tab.webSocketDebuggerUrl);
  await new Promise((res, rej) => ((ws.onopen = res), (ws.onerror = rej)));
  let n = 0;
  const pending = new Map();
  ws.onmessage = (e) => {
    const m = JSON.parse(e.data);
    const p = pending.get(m.id);
    if (!p) return;
    pending.delete(m.id);
    m.error ? p.rej(new Error(m.error.message)) : p.res(m.result);
  };
  const send = (method, params = {}) => new Promise((res, rej) => {
    const id = ++n;
    const timer = setTimeout(() => { pending.delete(id); rej(new Error(`Chrome timed out: ${method}`)); }, 30000);
    pending.set(id, { res: (value) => (clearTimeout(timer), res(value)), rej: (error) => (clearTimeout(timer), rej(error)) });
    ws.send(JSON.stringify({ id, method, params }));
  });
  const evaluate = async (expression) => (await send("Runtime.evaluate", { expression, returnByValue: true, awaitPromise: true })).result.value;
  return { proc, send, evaluate, close: () => (ws.close(), proc.kill()) };
}

async function shoot(b, base, { url, theme, file, full }) {
  const width = 1440;
  await b.send("Emulation.setDeviceMetricsOverride", { width, height: 900, deviceScaleFactor: 2, mobile: false });
  await b.send("Emulation.setEmulatedMedia", { features: [{ name: "prefers-color-scheme", value: theme }] });
  await b.send("Page.navigate", { url: base + url });
  for (let i = 0; i < 100 && !(await b.evaluate(`location.pathname + location.search === ${JSON.stringify(url)} && document.readyState === "complete" && !!document.querySelector("main")`)); i++) await sleep(100);
  await sleep(1500); // the views fill from /mcp after load
  // Not part of the picture: the setup banner and the "!" on the gear are only there because the port is not the one in .mcp.json.
  await b.evaluate(`document.querySelector('[title="Hide until the message changes"]')?.click(); [...document.querySelectorAll('a[href="/settings"] *')].find((e) => e.textContent.trim() === "!")?.remove()`);
  await sleep(300);
  const height = full ? await b.evaluate("document.documentElement.scrollHeight") : 900;
  const { data } = await b.send("Page.captureScreenshot", { format: "png", captureBeyondViewport: !!full, clip: { x: 0, y: 0, width, height, scale: 1 } });
  writeFileSync(join(img, file), Buffer.from(data, "base64"));
  console.log(`wrote docs/static/img/${file}`);
}

const SHOTS = [
  ...["light", "dark"].flatMap((theme) => [
    { url: "/", theme, file: `dashboard-${theme}.png`, full: true },
    { url: "/changes", theme, file: `changes-${theme}.png` },
    { url: "/requirements/API-02", theme, file: `requirements-${theme}.png` },
    { url: "/requirements?view=graph", theme, file: `requirements-graph-${theme}.png` },
    { url: "/tests", theme, file: `tests-${theme}.png` },
    { url: "/overlap", theme, file: `overlap-${theme}.png` },
    { url: "/uncovered", theme, file: `uncovered-${theme}.png` },
    { url: "/settings", theme, file: `settings-${theme}.png`, full: true },
  ]),
];

// Lossless recompression with oxipng: the copy bundled in ImageOptim.app, else one on PATH; skipped (with a note) when neither exists.
const OXIPNG = ["/Applications/ImageOptim.app/Contents/Frameworks/ImageOptimGPL.framework/Versions/A/Resources/oxipng", "oxipng"];
function optimize(files) {
  for (const bin of OXIPNG) {
    try {
      execFileSync(bin, ["-o", "max", "--strip", "safe", "--alpha", ...files], { stdio: "ignore" });
      return console.log(`compressed with ${bin}`);
    } catch (e) {
      if (e.code !== "ENOENT") throw e;
    }
  }
  console.log("oxipng not found (ImageOptim.app or PATH): screenshots left uncompressed");
}

async function main() {
  if (execFileSync("git", ["status", "--porcelain", "--", ...FILES], { cwd: root, encoding: "utf8" }).trim()) {
    throw new Error(`refusing to run: ${FILES.join(", ")} have uncommitted changes`);
  }
  if (!process.argv.includes("--no-build")) {
    run("pnpm", ["run", "build"], join(root, "web"));
    run("cargo", ["build", "--release", "-p", "spec17"]);
  }
  const dir = mkdtempSync(join(tmpdir(), "spec17-shots-"));
  let server, browser;
  const cleanup = () => {
    browser?.close();
    server?.kill();
    restore();
    rmSync(dir, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 });
  };
  process.on("SIGINT", () => (cleanup(), process.exit(130)));
  try {
    for (const file of FILES) originals.set(file, readFileSync(join(root, file)));
    breakThings();
    const overlap = JSON.parse(execFileSync(bin, ["overlap", "--json"], { cwd: root, encoding: "utf8" }));
    const uncovered = JSON.parse(execFileSync(bin, ["uncovered", "--json"], { cwd: root, encoding: "utf8" }));
    if (!overlap.length || !uncovered.units) throw new Error("screenshot data needs overlap and uncovered results; refresh the CodeGraph index");
    const port = await freePort();
    server = spawn(bin, ["--root", root, "serve", "--port", String(port), "--no-open"], { stdio: "ignore" });
    const base = `http://127.0.0.1:${port}`;
    for (let i = 0; ; i++) {
      if (await fetch(`${base}/api/info`).then((r) => r.ok, () => false)) break;
      if (i > 100) throw new Error("the server did not start");
      await sleep(100);
    }
    for (const [i, s] of SHOTS.entries()) {
      browser = await openBrowser(join(dir, `profile-${i}`));
      await shoot(browser, base, s);
      browser.close();
      browser = undefined;
    }
    optimize(SHOTS.map((s) => join(img, s.file)));
  } finally {
    cleanup();
  }
  console.log("restored:", FILES.join(", "));
}

main().catch((e) => {
  console.error(e.message);
  process.exit(1);
});

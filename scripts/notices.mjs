#!/usr/bin/env node
// Writes NOTICES (plain text): licence texts of the third-party code shipped in the spec17 binary.
//   Rust: crates reachable from the workspace through normal (not dev/build) dependencies, all targets (`cargo metadata`).
//   Web:  npm packages whose modules end up in web/dist/index.html (a Vite build with write: false records them).
// Each distinct licence text is printed once, after the list of packages that ship it.
// Usage: node scripts/notices.mjs   (make notices)   Needs cargo, and web/node_modules (pnpm install).
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { readdirSync, readFileSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
// The release targets (.github/workflows/prerelease.yml); crates only other targets use are not shipped.
const TARGETS = ["aarch64-apple-darwin", "x86_64-apple-darwin", "x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu", "x86_64-pc-windows-msvc"];
const LICENSE_FILE = /^(licen[cs]e|copying|notice|unlicense|copyright)/i;

const licenseTexts = (dir) =>
  readdirSync(dir, { withFileTypes: true })
    .filter((e) => e.isFile() && LICENSE_FILE.test(e.name))
    .sort((a, b) => a.name.localeCompare(b.name))
    .map((e) => readFileSync(join(dir, e.name), "utf8").replace(/\r\n/g, "\n").trim());

function rustPackages() {
  const meta = JSON.parse(execFileSync("cargo", ["metadata", "--format-version", "1", "--locked", ...TARGETS.flatMap((t) => ["--filter-platform", t])], { cwd: root, maxBuffer: 1 << 28, stdio: ["ignore", "pipe", "inherit"] }));
  const byId = new Map(meta.packages.map((p) => [p.id, p]));
  const nodes = new Map(meta.resolve.nodes.map((n) => [n.id, n]));
  const seen = new Set();
  const todo = [...meta.workspace_members];
  while (todo.length) {
    const id = todo.pop();
    if (seen.has(id)) continue;
    seen.add(id);
    for (const d of nodes.get(id).deps) if (d.dep_kinds.some((k) => k.kind === null)) todo.push(d.pkg);
  }
  const members = new Set(meta.workspace_members);
  return [...seen].filter((id) => !members.has(id)).map((id) => {
    const p = byId.get(id);
    return { name: p.name, version: p.version, license: p.license ?? "see licence file", url: p.repository ?? "", texts: licenseTexts(dirname(p.manifest_path)) };
  });
}

async function webPackages() {
  const web = join(root, "web");
  const { build } = await import(createRequire(join(web, "package.json")).resolve("vite"));
  const dirs = new Set();
  const collect = {
    name: "collect-packages",
    generateBundle(_, bundle) {
      for (const chunk of Object.values(bundle))
        for (const id of chunk.moduleIds ?? Object.keys(chunk.modules ?? {})) {
          const m = id.replace(/\\/g, "/").match(/^(.*\/node_modules\/(?:@[^/]+\/)?[^/]+)\//);
          if (m) dirs.add(m[1].replace(/^\0/, ""));
        }
    },
  };
  await build({ root: web, logLevel: "silent", plugins: [collect], build: { write: false } });
  return [...dirs].map((dir) => {
    const p = JSON.parse(readFileSync(join(dir, "package.json"), "utf8"));
    const repo = typeof p.repository === "string" ? p.repository : (p.repository?.url ?? "");
    return { name: p.name, version: p.version, license: p.license ?? "see licence file", url: repo, texts: licenseTexts(dir) };
  });
}

function section(title, pkgs) {
  pkgs.sort((a, b) => a.name.localeCompare(b.name) || a.version.localeCompare(b.version));
  const groups = new Map(); // licence text (whitespace-insensitive) -> packages
  for (const p of pkgs)
    for (const text of p.texts.length ? p.texts : [`No licence file in the package. Declared licence: ${p.license}.`]) {
      const key = createHash("sha256").update(text.replace(/\s+/g, " ")).digest("hex");
      if (!groups.has(key)) groups.set(key, { text, pkgs: [] });
      groups.get(key).pkgs.push(p);
    }
  const rule = "=".repeat(78);
  let out = `${rule}\n${title}\n${rule}\n\n`;
  for (const { text, pkgs: ps } of groups.values()) {
    for (const p of ps) out += `* ${p.name} ${p.version} (${p.license})${p.url ? ` ${p.url}` : ""}\n`;
    out += "\n" + text + "\n\n" + "-".repeat(78) + "\n\n";
  }
  return out;
}

// Copyleft (and source-available) licences: a dependency only under these would bind spec17's own licence.
const COPYLEFT = /^(A?GPL|LGPL|MPL|EPL|EUPL|CDDL|CPL|OSL|RPL|SSPL|BUSL|Elastic|CC-BY-(NC|SA|ND))/i;

// Can an SPDX expression be satisfied without a copyleft licence? `OR` (or the old `/`) needs one side, `AND` both.
// An undeclared licence fails, so a new package without one gets looked at.
function permissive(expr) {
  const tokens = expr.replace(/\//g, " OR ").match(/\(|\)|[^\s()]+/g) ?? [];
  let i = 0;
  const term = () => {
    const t = tokens[i++];
    if (t === "(") { const v = or(); i++; return v; }
    return !!t && !COPYLEFT.test(t);
  };
  const and = () => { let v = term(); while (tokens[i] === "AND") { i++; v = term() && v; } return v; };
  const or = () => { let v = and(); while (tokens[i] === "OR") { i++; v = and() || v; } return v; };
  return or() && i === tokens.length; // leftovers: not an SPDX expression
}
for (const [a, b] of [["MIT OR Apache-2.0", true], ["(MIT OR Apache-2.0) AND Unicode-3.0", true], ["MIT/GPL-2.0", true],
  ["GPL-3.0-only", false], ["MIT AND LGPL-2.1", false], ["see licence file", false]]) {
  if (permissive(a) !== b) throw new Error(`permissive(${a}) should be ${b}`);
}

const rust = rustPackages();
const web = await webPackages();
const bad = [...rust, ...web].filter((p) => !permissive(p.license));
if (bad.length) {
  for (const p of bad) console.error(`copyleft or undeclared licence: ${p.name} ${p.version} (${p.license})`);
  console.error("NOTICES not written: replace these dependencies, or review their licences and decide.");
  process.exit(1);
}

const out =
  "spec17 third-party notices\n\n" +
  "The spec17 binary includes the third-party components listed below, each under its own licence. " +
  "The spec17 licence (LICENSE) does not apply to them.\n\n" +
  section("Rust crates", rust) +
  section("Web dashboard (npm packages)", web);
writeFileSync(join(root, "NOTICES"), out.replace(/\n+$/, "\n"));
console.log("wrote NOTICES");

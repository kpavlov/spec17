import { test } from "node:test";
import assert from "node:assert/strict";
import { highlightLines } from "../src/lib/highlight.ts";

// [WEB-08] A Svelte component contains script, markup, template expressions and CSS.
test("highlights a Svelte component without changing its lines", () => {
  const source = `<script lang="ts">
let count: number = 1;
</script>
{#if count > 0}
<button>{count + 1}</button>
{/if}
<style>
button { color: red; }
</style>`;
  const lines = highlightLines("App.svelte", source);
  assert.equal(lines.length, source.split("\n").length);
  assert.match(lines[1], /hljs-keyword/);
  assert.match(lines[3], /hljs-keyword/);
  assert.match(lines[4], /hljs-name/);
  assert.match(lines[4], /hljs-number/);
  assert.match(lines[7], /hljs-attribute/);
  assert.equal(highlightLines("unknown.xyz", "<script>")[0], "&lt;script&gt;");
});

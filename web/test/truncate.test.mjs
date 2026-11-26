import test from "node:test";
import assert from "node:assert/strict";
import { middle } from "../src/lib/truncate.ts";

// [WEB-65]
test("a short path stays whole", () => {
  assert.equal(middle("specs/01-core.md:3"), "specs/01-core.md:3");
});
test("a long path keeps its start and its file name with the line", () => {
  const out = middle("web/src/lib/requirements/filterState.svelte.ts:42", 36);
  assert.equal(out.length, 36);
  assert.ok(out.startsWith("web/"));
  assert.ok(out.endsWith("…/filterState.svelte.ts:42"));
});
test("a long file name keeps both ends, with and without a folder", () => {
  for (const path of ["a/an-extremely-long-requirements-document.md:128", "an-extremely-long-requirements-document.md:128"]) {
    const out = middle(path, 20);
    assert.equal(out.length, 20, out);
    assert.ok(out.endsWith(".md:128"), out);
  }
});

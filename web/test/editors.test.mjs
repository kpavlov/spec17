import { test } from "node:test";
import assert from "node:assert/strict";
import { EDITORS } from "../src/lib/editors.ts";

// [WEB-08]
test("a path with URL characters stays one path in an editor link", () => {
  const url = new URL(EDITORS["VS Code"]("/repo/a#b?c d.ts", 12));
  assert.equal(url.hash + url.search, "");
  assert.equal(decodeURIComponent(url.pathname), "/repo/a#b?c d.ts:12");
});

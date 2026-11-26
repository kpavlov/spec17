import test from "node:test";
import assert from "node:assert/strict";
import { ago } from "../src/lib/ago.ts";

// [WEB-66]
test("the scan age rounds down to minutes, then hours", () => {
  assert.deepEqual([0, 59_999, 60_000, 3_599_999, 7_300_000].map(ago), ["just now", "just now", "1m ago", "59m ago", "2h ago"]);
});

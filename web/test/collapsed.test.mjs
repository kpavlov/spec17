import test from "node:test";
import assert from "node:assert/strict";
import { applyOpen } from "../src/lib/collapsed.ts";

// [WEB-13]
test("collapsing or expanding changes only the named groups, so another tab's stay", () => {
  const stored = ["API"];
  assert.deepEqual(applyOpen(stored, new Map([["GRAPH", false]])), ["API", "GRAPH"]);
  assert.deepEqual(applyOpen(["API", "GRAPH"], new Map([["GRAPH", true], ["API", false], ["WEB", true]])), ["API"]);
  assert.deepEqual(stored, ["API"]);
});

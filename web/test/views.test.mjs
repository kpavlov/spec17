import test from "node:test";
import assert from "node:assert/strict";
import { viewQuery, putView } from "../src/lib/requirements/views.ts";

// [WEB-71]
test("a saved view keeps only the search and filters, and a reused name replaces it in place", () => {
  assert.deepEqual(viewQuery({ tab: "requirements", id: "WEB-1", state: "all", level: "MUST", q: "x", effort: "", staged: "1" }), { level: "MUST", q: "x" });
  const list = [{ name: "a", tab: "requirements", query: {} }, { name: "b", tab: "requirements", query: {} }];
  const next = putView(list, { name: "a", tab: "changes", query: { q: "y" } });
  assert.deepEqual(next.map((v) => [v.name, v.tab]), [["a", "changes"], ["b", "requirements"]]);
  assert.equal(putView(next, { name: "c", tab: "requirements", query: {} }).length, 3);
});

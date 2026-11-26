import test from "node:test";
import assert from "node:assert/strict";
import { problemGroups } from "../src/lib/problems.ts";

// [WEB-67]
test("problem groups keep the report's order and add up to its total", () => {
  const groups = problemGroups({ unimplemented: 1, untested: 2, unproven_done: 0, invalid_tests: 3, specs_to_compare: 4, a_new_kind: 5 });
  assert.deepEqual(groups.map((g) => [g.id, g.open, g.n, g.list.map((p) => p.key)]), [
    ["errors", true, 4, ["unimplemented", "invalid_tests"]],
    ["warnings", false, 11, ["untested", "specs_to_compare", "a_new_kind"]],
  ]);
  assert.equal(groups[1].list.at(-1).title, "a new kind");
  assert.deepEqual(problemGroups({ unimplemented: 0 }), []);
});

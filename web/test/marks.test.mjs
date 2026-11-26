import test from "node:test";
import assert from "node:assert/strict";
import { applyMarks } from "../src/lib/marks.ts";

// [WEB-35]
test("an edit changes only the requirements it names, so another tab's marks stay", () => {
  const stored = { "A-1": "from-another-tab", "B-1": "old" };
  assert.deepEqual(applyMarks(stored, new Map([["C-1", "new"]])), { "A-1": "from-another-tab", "B-1": "old", "C-1": "new" });
  assert.deepEqual(applyMarks(stored, new Map([["B-1", undefined], ["C-1", "newer"], ["Z-1", undefined]])), { "A-1": "from-another-tab", "C-1": "newer" });
});

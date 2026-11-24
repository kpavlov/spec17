import { test } from "node:test";
import assert from "node:assert/strict";
import { cycles } from "../src/lib/requirements/cycles.ts";

// [WEB-47] Paths into and out of a cycle do not belong to it.
test("finds circular groups in directed requirement references", () => {
  assert.deepEqual(cycles([
    { id: "A", references: ["B"] }, { id: "B", references: ["C", "D"] },
    { id: "C", references: ["A"] }, { id: "D", references: [] },
    { id: "E", references: ["A"] }, { id: "F", references: ["G"] },
    { id: "G", references: ["F"] },
  ]), [["A", "B", "C"], ["F", "G"]]);
  assert.deepEqual(cycles([{ id: "A", references: ["B"] }, { id: "B", references: [] }]), []);
  assert.deepEqual(cycles([{ id: "A", references: ["A"] }]), [["A"]]);
});

// [WEB-47] Long chains do not overflow the call stack or create false cycles.
test("handles a large directed graph and preserves input order", () => {
  const rows = Array.from({ length: 10000 }, (_, i) => ({ id: String(i), references: i < 9999 ? [String(i + 1)] : ["9998", "missing"] }));
  assert.deepEqual(cycles(rows), [["9998", "9999"]]);
  assert.deepEqual(cycles([{ id: "B", references: ["A"] }, { id: "A", references: ["B"] }, { id: "D", references: ["D"] }]), [["B", "A"], ["D"]]);
});

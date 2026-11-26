import test from "node:test";
import assert from "node:assert/strict";
import { search } from "../src/lib/palette.ts";

const entries = [
  { label: "Dashboard", hint: "view" },
  { label: "Theme: dark", hint: "display" },
  { label: "WEB-12", hint: "move the selected row of the dashboard lists" },
  { label: "WEB-1", hint: "dark theme" },
];
const labels = (q, limit) => search(entries, q, limit).map((e) => e.label);

// [WEB-69]
test("every word must match, and entries that say it in their label come first", () => {
  assert.deepEqual(labels(""), ["Dashboard", "Theme: dark", "WEB-12", "WEB-1"]);
  assert.deepEqual(labels("dash"), ["Dashboard", "WEB-12"]);
  assert.deepEqual(labels("DARK theme"), ["Theme: dark", "WEB-1"]);
  assert.deepEqual(labels("web-1 dashboard"), ["WEB-12"]);
  assert.deepEqual(labels("nothing"), []);
  assert.deepEqual(labels("", 2), ["Dashboard", "Theme: dark"]);
});

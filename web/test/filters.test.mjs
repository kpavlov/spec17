import { test } from "node:test";
import assert from "node:assert/strict";
import { readValues, writeValues, toggleValue, pickValue, collapseAll, matches } from "../src/lib/requirements/filters.ts";
import { format, parse } from "../src/lib/router.ts";

// [WEB-59] Multiple values and unset round-trip without colliding with literal effort text.
test("filter values survive share links and can be removed", () => {
  for (const values of [["P0", "P1"], [null, "all", "3 days"], ["~literal"], ["all"]]) {
    const route = { tab: "requirements", effort: writeValues(values) };
    assert.deepEqual(readValues(parse(new URL(format(route), "http://localhost")).effort), values);
  }
  assert.deepEqual(readValues("all"), ["all"]);
  assert.deepEqual(toggleValue([null, "M"], null), ["M"]);
  assert.equal(writeValues([]), undefined);
});

// [WEB-60] A plain click replaces the selection, a modifier click adds or removes one value.
test("a plain click selects one value and a modifier click edits the set", () => {
  assert.deepEqual(pickValue([], "P1", false), ["P1"]);
  assert.deepEqual(pickValue(["P0"], "P1", false), ["P1"]);
  assert.deepEqual(pickValue(["P0", "P1"], "P1", false), ["P1"]);
  assert.deepEqual(pickValue(["P1"], "P1", false), []);
  assert.deepEqual(pickValue([null], null, false), []);
  assert.deepEqual(pickValue([], "P1", true), ["P1"]);
  assert.deepEqual(pickValue(["P0"], "P1", true), ["P0", "P1"]);
  assert.deepEqual(pickValue(["P0", "P1"], "P0", true), ["P1"]);
  assert.deepEqual(pickValue(["P1"], "P1", true), []);
  // selecting every choice is the same as All; a lone choice stays selectable
  assert.deepEqual(collapseAll(["P0", "P1"], ["P0", "P1"]), []);
  assert.deepEqual(collapseAll([null, "P0", "P1"], ["P0", "P1", null]), []);
  assert.deepEqual(collapseAll(["P0"], ["P0", "P1"]), ["P0"]);
  assert.deepEqual(collapseAll(["P0"], ["P0"]), ["P0"]);
});

// [WEB-59] Values within a facet are alternatives; different facets must all match.
test("facets combine alternatives and constrain one another", () => {
  const rows = [{ state: "none", priority: "P0" }, { state: "src-only", priority: "P1" }, { state: "tested", priority: "P0" }, { state: "none", priority: null }];
  const state = ["none", "src-only"], priority = ["P0", "P1"];
  assert.equal(rows.filter((r) => matches(state, r.state) && matches(priority, r.priority)).length, 2);
  assert.equal(rows.filter((r) => matches(state, r.state) && matches([null], r.priority)).length, 1);
  assert.equal(rows.filter((r) => matches(priority, r.priority) && r.state === "tested").length, 1);
});

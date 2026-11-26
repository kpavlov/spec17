import { test } from "node:test";
import assert from "node:assert/strict";
import { readValues, writeValues, toggleValue, pickValue, collapseAll, matches, parseQuery, mergePicked } from "../src/lib/requirements/filters.ts";
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

// [WEB-70]
test("search words that name a facet and a value become filters, the rest stays search text", () => {
  const choices = { state: ["none", "src-only", null], level: ["MUST", "SHOULD"], priority: ["P1", null], effort: ["S", null], changed: ["1", "0"], reviewed: ["1", "0"] };
  const parse = (text) => parseQuery(text, choices);
  assert.deepEqual(parse("state:none level:must open  port"), { picked: [["state", "none"], ["level", "MUST"]], rest: "open port" });
  assert.deepEqual(parse("Level:Must level:should").picked, [["level", "MUST"], ["level", "SHOULD"]]);
  assert.deepEqual(parse("priority:none effort:s").picked, [["priority", null], ["effort", "S"]]);
  assert.deepEqual(parse("changed:yes reviewed:no").picked, [["changed", "1"], ["reviewed", "0"]]);
  // unknown fields and values, bare colons and prototype names are text, kept as typed
  for (const text of ["state:nope", "owner:me", "level:", ":must", "constructor:x", "src/lib.rs:12  twice"]) assert.deepEqual(parse(text), { picked: [], rest: text });
});

// [WEB-70]
test("picked filter words add to what is already selected, and every choice selected reads as All", () => {
  const now = { level: ["MUST"], state: [], priority: ["P1"] };
  const all = { level: ["MUST", "SHOULD", "MAY"], state: ["none", "tested"], priority: ["P1", null] };
  const merge = (picked) => mergePicked(picked, (k) => now[k], (k) => all[k]);
  // an existing selection is kept, a word repeating it adds nothing, a field named twice takes both; other fields are left alone
  assert.deepEqual(readValues(merge([["level", "SHOULD"], ["level", "MUST"]]).level), ["MUST", "SHOULD"]);
  assert.deepEqual(Object.keys(merge([["level", "SHOULD"]])), ["level"]);
  assert.deepEqual(readValues(merge([["state", "none"]]).state), ["none"]);
  assert.deepEqual(merge([["priority", null], ["state", "none"], ["state", "tested"]]), { priority: undefined, state: undefined });
});

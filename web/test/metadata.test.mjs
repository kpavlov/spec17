import { test } from "node:test";
import assert from "node:assert/strict";
import { format, parse } from "../src/lib/router.ts";
import { priorityTone, effortTone, verdictMark } from "../src/lib/tone.ts";
import { metadataKey, metadataValue } from "../src/lib/requirements/metadata.ts";

// [WEB-57] An estimate named "all" remains distinct from the clear option.
test("metadata values do not collide with the all option", () => {
  assert.notEqual(metadataKey("all"), "all");
  assert.equal(metadataValue(metadataKey("all")), "all");
  assert.equal(metadataValue("all"), undefined);
});

// [WEB-57] Filter links keep both hints, including estimates with spaces.
test("requirement metadata filters round-trip in the URL", () => {
  const route = { tab: "requirements", priority: "P2", effort: "3 points", state: "manual" };
  const parsed = parse(new URL(format(route), "http://localhost"));
  assert.equal(parsed.priority, route.priority);
  assert.equal(parsed.effort, route.effort);
  assert.equal(parsed.state, route.state);
});

// [WEB-57] Known values have distinct colors; other estimates stay neutral.
test("priority and size colors follow their values", () => {
  assert.notEqual(priorityTone("P1"), priorityTone("P2"));
  assert.notEqual(effortTone("S"), effortTone("L"));
  assert.equal(effortTone("3 points"), "muted");
});

// [WEB-64]
test("a check mark without proof never looks like a proven one", () => {
  for (const verdict of ["NO-TEST", "NO-LINK"]) assert.deepEqual([verdictMark(verdict).icon, verdictMark(verdict).tone], ["check-alert", "orange"]);
  assert.match(verdictMark("NO-TEST").title, /no tagged test/);
  assert.equal(verdictMark("ok").icon, "check");
});

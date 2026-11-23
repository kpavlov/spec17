import { test, before, after } from "node:test";
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { createServer } from "vite";
import { additive, step } from "../src/lib/ui/seg.ts";

// Seg.svelte is rendered on the server through Vite (its own config, no port): this checks the markup and the ARIA state it produces.
// Events need a DOM, so what a click or an arrow decides lives in seg.ts and is tested below.
let vite, Seg, render;
before(async () => {
  vite = await createServer({ root: fileURLToPath(new URL("..", import.meta.url)), appType: "custom", logLevel: "silent", server: { middlewareMode: true, watch: null, hmr: false } });
  Seg = (await vite.ssrLoadModule("/src/lib/ui/Seg.svelte")).default;
  // the same Vite module graph as the component: a second copy of Svelte's server runtime cannot render it
  render = (await vite.ssrLoadModule("svelte/server")).render;
});
after(() => vite.close());

const attrs = (s) => Object.fromEntries([...s.matchAll(/([\w-]+)="([^"]*)"/g)].map(([, k, v]) => [k, v]));
const classes = (a) => (a.class ?? "").split(" ");
function seg(props) {
  const { body } = render(Seg, { props: { label: "Level", onpick() {}, ...props } });
  const root = attrs(body.match(/<div class="seg[^>]*>/)[0]);
  const buttons = [...body.matchAll(/<button\b([^>]*)>(.*?)<\/button>/g)].map(([, a, inner]) => ({
    attr: attrs(a),
    label: inner.match(/data-text="([^"]*)"/)?.[1],
    count: inner.match(/<b[^>]*>(.*?)<\/b>/)?.[1],
    tone: inner.match(/data-tone="([^"]*)"/)?.[1],
    check: inner.includes('class="check'),
  }));
  return { body, root, buttons };
}
const options = [
  { key: "", label: "All", count: 5 },
  { key: "MUST", label: "MUST", count: 3, tone: "must" },
  { key: "MAY", label: "MAY", count: 0 },
  { key: "other", label: "other", count: 2 },
];

// [WEB-59] Toggle groups mark every selected value as pressed, and only those.
test("a toggle group exposes which values are pressed", () => {
  const { root, buttons } = seg({ options, selected: ["MUST", "MAY"] });
  assert.deepEqual([root.role, root["aria-label"]], ["group", "Level"]);
  assert.deepEqual(buttons.map((b) => b.label), ["All", "MUST", "MAY", "other"]);
  assert.deepEqual(buttons.map((b) => b.attr["aria-pressed"]), ["false", "true", "true", "false"]);
  assert.ok(buttons.every((b) => b.attr.role === undefined && b.attr["aria-checked"] === undefined && b.attr.tabindex === "0" && b.attr.type === "button"));
  assert.ok(buttons.every((b) => b.check), "each toggle has a check slot, shown only when pressed");
  assert.deepEqual(buttons.map((b) => b.count), ["5", "3", "0", "2"]);
  assert.equal(buttons[1].tone, "must");
  assert.equal(buttons[0].tone, undefined);
});

// [WEB-59] A selected value with no matches is still clearly marked; an unselected one is muted.
test("a choice with no matches is muted unless it is selected", () => {
  const muted = (selected) => seg({ options, selected }).buttons.map((b) => classes(b.attr).includes("zero"));
  assert.deepEqual(muted(["MUST"]), [false, false, true, false]);
  assert.deepEqual(muted(["MAY"]), [false, false, false, false]);
});

// [WEB-59] The All choice is pressed while nothing else is.
test("All is pressed when it alone is selected", () => {
  assert.deepEqual(seg({ options, selected: [""] }).buttons.map((b) => b.attr["aria-pressed"]), ["true", "false", "false", "false"]);
});

test("a single-choice group is a radio group with one tab stop", () => {
  const { root, buttons } = seg({ options, value: "MAY" });
  assert.deepEqual([root.role, root["aria-label"]], ["radiogroup", "Level"]);
  assert.ok(buttons.every((b) => b.attr.role === "radio" && b.attr["aria-pressed"] === undefined && !b.check));
  assert.deepEqual(buttons.map((b) => b.attr["aria-checked"]), ["false", "false", "true", "false"]);
  assert.deepEqual(buttons.map((b) => b.attr.tabindex), ["-1", "-1", "0", "-1"]);
  // nothing checked: the first option is the tab stop, so the group stays reachable
  assert.deepEqual(seg({ options, value: "nothing here" }).buttons.map((b) => b.attr.tabindex), ["0", "-1", "-1", "-1"]);
});

// Selecting turns the label bold: an invisible bold copy of it (the data-text) keeps the button from changing width.
test("every label carries the text its bold copy is made from", () => {
  const { buttons } = seg({ options, value: "" });
  assert.deepEqual(buttons.map((b) => b.label), options.map((o) => o.label));
  // a label with markup characters stays inside its attribute: unescaped, it is the label again
  const tricky = 'a "quoted" <b> & more';
  const unescape = (s) => s.replace(/&quot;/g, '"').replace(/&lt;/g, "<").replace(/&gt;/g, ">").replace(/&amp;/g, "&");
  const { buttons: [one] } = seg({ options: [{ key: "a", label: tricky }], value: "" });
  assert.equal(unescape(one.label), tricky);
  assert.ok(!one.label.includes('"') && !one.label.includes("<"));
});

test("keycap, hint and nowrap are rendered when given", () => {
  const plain = seg({ options, value: "" });
  assert.ok(!plain.body.includes("<kbd>") && plain.root.title === undefined && !classes(plain.root).includes("nowrap"));
  const full = seg({ options, value: "", keycap: "L", hint: "L for next", nowrap: true });
  assert.ok(full.body.includes("<kbd>L</kbd>"));
  assert.equal(full.root.title, "L for next");
  assert.ok(classes(full.root).includes("nowrap"));
});

// [WEB-60] Shift, Ctrl or ⌘ makes a click additive; a plain click, or Alt alone, does not.
test("a click is additive with Shift, Ctrl or ⌘", () => {
  const key = (held) => additive({ shiftKey: false, ctrlKey: false, metaKey: false, altKey: false, ...held });
  assert.equal(key({}), false);
  assert.equal(key({ altKey: true }), false);
  for (const held of [{ shiftKey: true }, { ctrlKey: true }, { metaKey: true }, { shiftKey: true, metaKey: true }]) assert.equal(key(held), true, JSON.stringify(held));
});

test("arrows step through the options and wrap around", () => {
  const keys = ["list", "graph", "docs"];
  assert.equal(step(keys, "list", 1), "graph");
  assert.equal(step(keys, "docs", 1), "list");
  assert.equal(step(keys, "list", -1), "docs");
  assert.equal(step(keys, "graph", -1), "list");
  // nothing selected, or a value that is no longer offered: → starts at the first, ← at the last
  assert.equal(step(keys, undefined, 1), "list");
  assert.equal(step(keys, "gone", -1), "docs");
  // an empty key is a real option (All)
  assert.equal(step(["", "a"], undefined, 1), "a");
  assert.equal(step(["a"], "a", 1), "a");
});

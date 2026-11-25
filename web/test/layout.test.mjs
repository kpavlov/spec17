import { test } from "node:test";
import assert from "node:assert/strict";
import { components, homeFor, idPrefix,prefixForce, reduceCrossings, separate } from "../src/lib/requirements/layout.ts";

// [WEB-48] Two crossing edges can be untangled by swapping their endpoints.
test("reduces crossings without changing the set of node positions", () => {
  const nodes = [{ x: 0, y: 0 }, { x: 1, y: 1 }, { x: 0, y: 1 }, { x: 1, y: 0 }];
  const before = nodes.map(({ x, y }) => `${x},${y}`).sort();
  reduceCrossings(nodes, [{ source: nodes[0], target: nodes[1] }, { source: nodes[2], target: nodes[3] }]);
  assert.deepEqual(nodes.map(({ x, y }) => `${x},${y}`).sort(), before);
  const [a, b, c, d] = nodes;
  const side = (p, q, r) => (q.x - p.x) * (r.y - p.y) - (q.y - p.y) * (r.x - p.x);
  assert.ok(side(a, b, c) * side(a, b, d) >= 0 || side(c, d, a) * side(c, d, b) >= 0);
});

// [WEB-48] A swap that would make nodes of different sizes overlap is not made.
test("skips a swap that would overlap nodes", () => {
  // the big node fits only where it is: next to C's or D's place sits another node, so the crossing goes by swapping small ones
  const nodes = [{ x: 0, y: 0, r: 1.4 }, { x: 4, y: 4, r: 0.05 }, { x: 0, y: 4, r: 0.05 }, { x: 4, y: 0, r: 0.05 }, { x: 0, y: 5.3, r: 0.05 }, { x: 5.3, y: 0, r: 0.05 }];
  reduceCrossings(nodes, [{ source: nodes[0], target: nodes[1] }, { source: nodes[2], target: nodes[3] }], (n) => n.r);
  assert.deepEqual([nodes[0].x, nodes[0].y], [0, 0]);
  for (let i = 0; i < nodes.length; i++) for (let j = i + 1; j < nodes.length; j++) {
    const [a, b] = [nodes[i], nodes[j]];
    assert.ok(Math.hypot(a.x - b.x, a.y - b.y) >= a.r + b.r, `${i},${j}`);
  }
});

// [WEB-50] Direction does not split a cluster; disconnected nodes keep their own space.
test("groups linked nodes and separates the clusters with forces", () => {
  const nodes = [{ x: 0, y: 0, r: 5 }, { x: 20, y: 0, r: 5 }, { x: 0, y: 0, r: 5 }, { x: 0, y: 0, r: 5 }];
  const groups = components(nodes, [{ source: nodes[1], target: nodes[0] }, { source: nodes[2], target: nodes[2] }]);
  assert.deepEqual(groups, [[nodes[0], nodes[1]], [nodes[2]], [nodes[3]]]);
  const centers = separate(groups, 60);
  assert.equal(nodes[1].x - nodes[0].x, 20);
  assert.equal(centers.get(nodes[0]), centers.get(nodes[1]));
  for (let i = 0; i < groups.length; i++) for (let j = i + 1; j < groups.length; j++) {
    for (const a of groups[i]) for (const b of groups[j]) assert.ok(Math.hypot(a.x - b.x, a.y - b.y) >= a.r + b.r + 40);
  }
});

// [WEB-53] Prefix peers pull together; a different prefix gets no pull.
test("weakly attracts nodes sharing an ID prefix", () => {
  const nodes = [{ id: "API-01", x: -100, y: 0 }, { id: "API-02", x: 100, y: 0 }, { id: "WEB-01", x: 50, y: 0 }];
  const force = prefixForce((n) => [n.id.split(/[-_]/, 1)[0]]);
  force.initialize(nodes);
  force(1);
  assert.ok(nodes[0].vx > 0 && nodes[1].vx < 0);
  assert.equal(nodes[2].vx, 0);
  assert.ok(nodes[0].vx < 2);
});

// [WEB-50][WEB-53] Prefix attraction still leaves space around linked constellations.
test("separates linked clusters with matching prefixes", () => {
  const a = [{ id: "API-01", x: 0, y: 0, r: 8 }, { id: "API-02", x: 30, y: 0, r: 8 }];
  const b = [{ id: "API-03", x: 0, y: 0, r: 8 }, { id: "API-04", x: 30, y: 0, r: 8 }];
  separate([a, b], 60, (n) => n.id.split("-")[0]);
  for (const x of a) for (const y of b) assert.ok(Math.hypot(x.x - y.x, x.y - y.y) > x.r + y.r + 60);
});

// [WEB-50][WEB-53] Prefixes listed in turns still end up in one block each; a mixed linked cluster goes whole to its commonest prefix.
test("places each ID prefix in one block", () => {
  const node = (id, x = 0) => ({ id, x, y: 0, r: 20 });
  const singles = Array.from({ length: 30 }, (_, i) => node(`${"ABC"[i % 3]}-${i}`));
  const mixed = [node("B-90"), node("A-91", 50), node("A-92", 100)];
  const centers = separate([...singles.map((n) => [n]), mixed], 60, (n) => idPrefix(n.id));
  const nodes = [...singles, ...mixed];
  const blockOf = (n) => (mixed.includes(n) ? "A" : idPrefix(n.id));
  const distance = (a, b) => Math.hypot(a.x - b.x, a.y - b.y);
  for (const a of nodes) {
    const [nearest] = nodes.filter((b) => b !== a).sort((b, c) => distance(a, b) - distance(a, c));
    assert.equal(blockOf(nearest), blockOf(a), a.id);
    for (const b of nodes) if (blockOf(a) !== blockOf(b)) assert.ok(distance(a, b) >= a.r + b.r + 90, `${a.id} ${b.id}`);
  }
  for (const [i, x] of [[1, 50], [2, 100]]) assert.ok(Math.abs(mixed[i].x - mixed[0].x - x) < 1e-6 && Math.abs(mixed[i].y - mixed[0].y) < 1e-6);
  assert.equal(centers.get(mixed[0]), centers.get(mixed[2]));
  assert.ok(Math.abs(centers.get(mixed[0]).x - mixed[1].x) < 1e-6); // the centre moved with the block
});

// [WEB-53] A later addition joins its block even when a stray peer pulls the average elsewhere; a new prefix goes outside.
test("finds a place for a node added after the first layout", () => {
  const peers = [{ x: 0, y: 0 }, { x: 40, y: 0 }, { x: 20, y: 30 }, { x: 900, y: 0 }];
  assert.deepEqual(homeFor(peers, peers, 100), { x: 40, y: 0 });
  assert.deepEqual(homeFor([], peers, 100), { x: 1000, y: 0 });
  assert.deepEqual(homeFor([], [], 100), { x: 100, y: 0 });
});

// [WEB-53] A requirement ID belongs to the group of its prefix, with either separator.
test("takes the prefix of an ID", () => {
  assert.deepEqual(["AC-1.2", "REQ_001", "FR-001", "X"].map(idPrefix), ["AC", "REQ", "FR", "X"]);
});

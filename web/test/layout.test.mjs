import { test } from "node:test";
import assert from "node:assert/strict";
import { components, prefixForce, reduceCrossings, separate } from "../src/lib/requirements/layout.ts";

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

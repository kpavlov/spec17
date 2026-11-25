import { forceCollide, forceSimulation, forceX, forceY, type Force, type SimulationNodeDatum } from "d3-force";

type Point = { x: number; y: number };
type Edge<T> = { source: T; target: T };

// (WEB-53) The prefix of a requirement ID: `AC-1.2` and `REQ_001` belong to `AC` and `REQ`.
export const idPrefix = (id: string): string => id.split(/[-_]/, 1)[0];

// (WEB-53) Pull each node gently towards the center of its prefix peers.
export function prefixForce<T extends SimulationNodeDatum>(keys: (node: T) => string[]): Force<T, undefined> {
  let nodes: T[] = [];
  const force = (alpha: number) => {
    const totals = new Map<string, { x: number; y: number; n: number }>();
    for (const node of nodes) for (const key of keys(node)) {
      const t = totals.get(key) ?? { x: 0, y: 0, n: 0 };
      t.x += node.x!; t.y += node.y!; t.n++;
      totals.set(key, t);
    }
    for (const node of nodes) for (const key of keys(node)) {
      const t = totals.get(key)!;
      node.vx = (node.vx ?? 0) + (t.x / t.n - node.x!) * alpha * 0.015;
      node.vy = (node.vy ?? 0) + (t.y / t.n - node.y!) * alpha * 0.015;
    }
  };
  force.initialize = (next: T[]) => { nodes = next; };
  return force;
}

// (WEB-50) Links in either direction join the same visual cluster.
export function components<T>(nodes: T[], edges: Edge<T>[]): T[][] {
  const adjacent = new Map(nodes.map((node) => [node, new Set<T>()]));
  for (const { source, target } of edges) {
    adjacent.get(source)?.add(target);
    adjacent.get(target)?.add(source);
  }
  const unseen = new Set(nodes);
  const groups: T[][] = [];
  for (const node of nodes) {
    if (!unseen.delete(node)) continue;
    const group = [node];
    for (let i = 0; i < group.length; i++) {
      for (const next of adjacent.get(group[i]) ?? []) if (unseen.delete(next)) group.push(next);
    }
    groups.push(group);
  }
  return groups;
}

// Repel clusters by their bounding circles without changing their internal layout. A linked cluster (two or more nodes) gets
// `linkedMargin` more room around it; xPull and yPull draw the clusters to the middle.
function pack<T extends Point & { r: number }>(groups: T[][], gap: number, linkedMargin = 0, xPull = 0.05, yPull = 0.05): Map<T, Point> {
  const clusters = groups.filter((group) => group.length).map((group) => {
    const x = (Math.min(...group.map((n) => n.x - n.r)) + Math.max(...group.map((n) => n.x + n.r))) / 2;
    const y = (Math.min(...group.map((n) => n.y - n.r)) + Math.max(...group.map((n) => n.y + n.r))) / 2;
    const radius = Math.max(...group.map((n) => Math.hypot(n.x - x, n.y - y) + n.r)) + gap / 2 + (group.length > 1 ? linkedMargin : 0);
    return { group, oldX: x, oldY: y, radius, x: NaN, y: NaN };
  });
  forceSimulation(clusters).force("repel", forceCollide<typeof clusters[number]>((c) => c.radius).strength(1).iterations(4))
    .force("x", forceX(0).strength(xPull)).force("y", forceY(0).strength(yPull)).stop().tick(300);
  const centers = new Map<T, Point>();
  for (const cluster of clusters) {
    const center = { x: cluster.x, y: cluster.y };
    for (const node of cluster.group) { node.x += cluster.x - cluster.oldX; node.y += cluster.y - cluster.oldY; centers.set(node, center); }
  }
  return centers;
}

const commonest = (keys: string[]): string => {
  const count = new Map<string, number>();
  for (const key of keys) count.set(key, (count.get(key) ?? 0) + 1);
  return [...count].reduce((best, next) => (next[1] > best[1] ? next : best))[0]; // a tie goes to the first seen
};

// Space clusters apart and return each node's cluster center. (WEB-53) With a `prefix`, the clusters of one prefix are first
// packed into a block, then whole blocks are spaced farther apart, so another prefix cannot land among them.
export function separate<T extends Point & { r: number }>(groups: T[][], gap = 60, prefix?: (node: T) => string): Map<T, Point> {
  if (!prefix) return pack(groups, gap);
  const blocks = new Map<string, T[][]>();
  for (const group of groups) {
    if (!group.length) continue;
    const key = commonest(group.map(prefix));
    if (!blocks.has(key)) blocks.set(key, []);
    blocks.get(key)!.push(group);
  }
  const packed = [...blocks.values()].map((block) => {
    const inner = pack(block, gap * 0.4, gap * 0.4); // (WEB-50) linked clusters still stand apart, by less than blocks do
    const nodes = block.flat();
    return { inner, nodes, from: { x: nodes[0].x, y: nodes[0].y } };
  });
  pack(packed.map((block) => block.nodes), gap * 1.6, 0, 0.015, 0.09); // a wide band, like the canvas
  const centers = new Map<T, Point>();
  for (const { inner, nodes, from } of packed) {
    const [dx, dy] = [nodes[0].x - from.x, nodes[0].y - from.y]; // a block moves as one body
    for (const center of new Set(inner.values())) { center.x += dx; center.y += dy; }
    for (const [node, center] of inner) centers.set(node, center);
  }
  return centers;
}

// (WEB-53) Where a node added after the first layout goes: on the peer nearest the middle of its peers (always a place inside
// their block), or, for a group with no peers yet, `gap` to the right of everything placed.
export function homeFor(peers: Point[], placed: Point[], gap: number): Point {
  if (!peers.length) return { x: Math.max(0, ...placed.map((p) => p.x)) + gap, y: 0 };
  const [mx, my] = [peers.reduce((sum, p) => sum + p.x, 0) / peers.length, peers.reduce((sum, p) => sum + p.y, 0) / peers.length];
  const from = (p: Point) => Math.hypot(p.x - mx, p.y - my);
  const { x, y } = peers.reduce((best, next) => (from(next) < from(best) ? next : best));
  return { x, y };
}

function crosses(a: Point, b: Point, c: Point, d: Point): boolean {
  const side = (p: Point, q: Point, r: Point) => (q.x - p.x) * (r.y - p.y) - (q.y - p.y) * (r.x - p.x);
  return side(a, b, c) * side(a, b, d) < 0 && side(c, d, a) * side(c, d, b) < 0;
}

// (WEB-48) Swapping positions removes crossings. Nodes of different sizes may not fit each other's place, so a swap that would bring
// either node closer to another than the sum of their `radius` is skipped (a and b keep their distance and radii).
export function reduceCrossings<T extends Point>(nodes: T[], edges: Edge<T>[], radius: (node: T) => number = () => 0): void {
  // shortcut: bound quadratic crossing checks for dense graphs, revisit with a scalable layout algorithm
  if (nodes.length > 100 || edges.length > 120) return;
  const incident = new Map(nodes.map((node) => [node, [] as number[]]));
  edges.forEach((edge, i) => {
    incident.get(edge.source)?.push(i);
    if (edge.target !== edge.source) incident.get(edge.target)?.push(i);
  });
  const intersects = (i: number, j: number) => {
    const a = edges[i], b = edges[j];
    return a.source !== b.source && a.source !== b.target && a.target !== b.source && a.target !== b.target && crosses(a.source, a.target, b.source, b.target);
  };
  let total = 0;
  for (let i = 0; i < edges.length; i++) for (let j = i + 1; j < edges.length; j++) if (intersects(i, j)) total++;
  for (let pass = 0; pass < 2 && total; pass++) {
    let improved = false;
    for (let i = 0; i < nodes.length && total; i++) for (let j = i + 1; j < nodes.length && total; j++) {
      const a = nodes[i], b = nodes[j];
      const affected = new Set([...(incident.get(a) ?? []), ...(incident.get(b) ?? [])]);
      const count = () => {
        let n = 0;
        for (const edge of affected) for (let other = 0; other < edges.length; other++) {
          if (other !== edge && (!affected.has(other) || other > edge) && intersects(edge, other)) n++;
        }
        return n;
      };
      const before = count();
      [a.x, b.x] = [b.x, a.x];
      [a.y, b.y] = [b.y, a.y];
      const clear = (n: T) => nodes.every((c) => c === a || c === b || Math.hypot(n.x - c.x, n.y - c.y) >= radius(n) + radius(c));
      const after = clear(a) && clear(b) ? count() : before;
      if (after < before) { total += after - before; improved = true; }
      else { [a.x, b.x] = [b.x, a.x]; [a.y, b.y] = [b.y, a.y]; }
    }
    if (!improved) break;
  }
}

import { forceCollide, forceSimulation, forceX, forceY, type Force, type SimulationNodeDatum } from "d3-force";

type Point = { x: number; y: number };
type Edge<T> = { source: T; target: T };

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

// Repel clusters by their bounding circles without changing their internal layout.
export function separate<T extends Point & { r: number }>(groups: T[][], gap = 60, prefix?: (node: T) => string): Map<T, Point> {
  const clusters = groups.filter((group) => group.length).map((group) => {
    const x = (Math.min(...group.map((n) => n.x - n.r)) + Math.max(...group.map((n) => n.x + n.r))) / 2;
    const y = (Math.min(...group.map((n) => n.y - n.r)) + Math.max(...group.map((n) => n.y + n.r))) / 2;
    const radius = Math.max(...group.map((n) => Math.hypot(n.x - x, n.y - y) + n.r)) + gap / 2 + (prefix && group.length > 1 ? gap : 0);
    return { group, oldX: x, oldY: y, radius, x: NaN, y: NaN, prefixes: prefix ? [...new Set(group.map(prefix))] : [] };
  });
  forceSimulation(clusters).force("repel", forceCollide<typeof clusters[number]>((c) => c.radius).strength(1).iterations(4))
    .force("x", forceX(0).strength(prefix ? 0.015 : 0.05)).force("y", forceY(0).strength(prefix ? 0.09 : 0.05))
    .force("prefix", prefix ? prefixForce((c) => c.prefixes) : null).stop().tick(300);
  const centers = new Map<T, Point>();
  for (const cluster of clusters) {
    const center = { x: cluster.x, y: cluster.y };
    for (const node of cluster.group) { node.x += cluster.x - cluster.oldX; node.y += cluster.y - cluster.oldY; centers.set(node, center); }
  }
  return centers;
}

function crosses(a: Point, b: Point, c: Point, d: Point): boolean {
  const side = (p: Point, q: Point, r: Point) => (q.x - p.x) * (r.y - p.y) - (q.y - p.y) * (r.x - p.x);
  return side(a, b, c) * side(a, b, d) < 0 && side(c, d, a) * side(c, d, b) < 0;
}

// (WEB-48) Swapping positions preserves the simulation's spacing while removing crossings.
export function reduceCrossings<T extends Point>(nodes: T[], edges: Edge<T>[]): void {
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
      const after = count();
      if (after < before) { total += after - before; improved = true; }
      else { [a.x, b.x] = [b.x, a.x]; [a.y, b.y] = [b.y, a.y]; }
    }
    if (!improved) break;
  }
}

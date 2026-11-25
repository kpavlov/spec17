<script lang="ts" generics="T">
  // A force-directed graph: d3-force lays it out, a plain 2D canvas draws it. It does not know what a node is. The caller gives the
  // nodes (area by `size`, an outline by `ring`), the arrows (width by `weight`) and the words (label, legend, tooltip).
  import { untrack } from "svelte";
  import { forceCollide, forceLink, forceManyBody, forceSimulation, forceX, forceY, type Simulation, type SimulationNodeDatum } from "d3-force";
  import { components, homeFor, prefixForce, reduceCrossings, separate } from "$lib/requirements/layout";

  type Item = { id: string; size: number; ring?: boolean; links: { to: string; weight: number }[]; data: T };
  type N = SimulationNodeDatum & { id: string; data: T; r: number; size: number; ring: boolean; deg: number; dependents: number; hue: number; x: number; y: number };
  type L = { source: string | N; target: string | N; weight: number };

  let { items, open, q = "", label, legend, tip, group, cycleGroups = [] }: {
    items: Item[];
    open: (id: string) => void;
    q?: string;
    /** The canvas's accessible name, and the sentence under it that explains arrows, area and colour. */
    label: string;
    legend: string;
    /** What hovering a node says: a bold title, an optional note after it, and a detail clamped to five lines. */
    tip: (node: { id: string; size: number; deg: number; dependents: number; data: T }) => { title: string; meta?: string; detail: string };
    /** The group a node belongs to. Given, nodes of one group attract each other and start in one block (WEB-53), and crossings are reduced (WEB-48). */
    group?: (id: string) => string;
    /** Ids in circular groups: their nodes and the arrows between them are drawn in the warning colour. */
    cycleGroups?: string[][];
  } = $props();

  let w = $state(0);
  let h = $state(0);
  let canvas: HTMLCanvasElement;
  let view = { k: 1, x: 0, y: 0 }; // zoom and pan; the origin is the canvas centre
  let nodes: N[] = [];
  let links: L[] = [];
  let hover = $state.raw<N | null>(null);
  let mouse = $state.raw({ x: 0, y: 0 });
  let tipHeight = $state(0); // measured, so a tall tooltip near the bottom edge moves up instead of being cut off
  let sim: Simulation<N, undefined> | undefined;
  let raf = 0;
  let needsFit = $state(false);
  const pos = new Map<string, { x: number; y: number }>(); // survives a reload, so the layout does not jump
  let centers = new Map<string, { x: number; y: number }>();
  let cycleOf = $derived(new Map(cycleGroups.flatMap((group, i) => group.map((id) => [id, i] as const))));
  let colors = { fg: "", muted: "", ok: "", warn: "", mono: "monospace", labelSize: 12 };

  // a colour per folder, so nodes that live together look alike
  // green (90–180°) is skipped: the ring is green, so no fill should be
  const hueAt = (t: number) => (180 + t * 270) % 360;
  const hueOf = (path: string) => hueAt([...(path.includes("/") ? path.slice(0, path.lastIndexOf("/")) : path)].reduce((a, c) => (a * 31 + c.charCodeAt(0)) % 360, 7) / 360);
  const ends = (l: L) => [l.source as N, l.target as N] as const; // d3 swaps the ids for the nodes
  // connected hubs need more room (WEB-46); like charges repel, more for a bigger node
  const distance = (l: L) => 50 + (l.source as N).r + (l.target as N).r + 8 * Math.sqrt((l.source as N).deg + (l.target as N).deg);
  const charge = (n: N) => -(80 + 0.5 * n.r * n.r);

  $effect(() => {
    const max = Math.max(1, ...items.map((m) => m.size));
    const rmax = Math.max(14, Math.min(64, 320 / Math.sqrt(items.length))); // the biggest node; the rest by area
    const first = untrack(() => pos.size === 0);
    // (WEB-53) groups take evenly spaced colours in order of appearance: hashing short prefixes made several look alike
    const groups = group ? [...new Set(items.map((m) => group(m.id)))] : [];
    const hue = (id: string) => (group ? hueAt(groups.indexOf(group(id)) / groups.length) : hueOf(id));
    const ns: N[] = items.map((m) => ({ id: m.id, data: m.data, size: m.size, r: Math.max(4, rmax * Math.sqrt(m.size / max)), ring: !!m.ring, deg: 0, dependents: 0, hue: hue(m.id), x: NaN, y: NaN, ...pos.get(m.id) }));
    ns.sort((a, b) => b.r - a.r); // big first, so small nodes are drawn (and hit) on top
    const byId = new Map(ns.map((n) => [n.id, n]));
    const ls: L[] = items.flatMap((m) => m.links.filter((d) => byId.has(d.to)).map((d) => ({ source: m.id, target: d.to, weight: d.weight })));
    for (const l of ls) for (const e of [l.source, l.target]) byId.get(e as string)!.deg++;
    for (const l of ls) byId.get(l.target as string)!.dependents++; // (WEB-54)
    sim?.stop();
    // (WEB-53) a node added since the first layout has no place yet: with nothing repelling, the origin would drop it in any block
    if (!first && group) for (const n of ns) {
      if (centers.has(n.id)) continue;
      const known = ns.filter((m) => centers.has(m.id));
      const at = (list: N[]) => list.map((m) => centers.get(m.id)!);
      const home = homeFor(at(known.filter((m) => group(m.id) === group(n.id))), at(known), 8 * n.r);
      centers.set(n.id, home);
      [n.x, n.y] = [home.x + 1, home.y + 1]; // just off the peer it shares the place with, so collision can part them
    }
    // seed new nodes with d3's spread instead of piling them at the origin
    sim = forceSimulation(ns)
      .force("link", forceLink<N, L>(ls).id((n) => n.id).distance(distance).strength(0.5))
      // gravity pulls everything to the middle (or to its cluster's centre), collision is a hard core: atoms never overlap.
      // Grouped, nothing repels: all-pairs repulsion would blow the tight blocks apart on the first drag (WEB-53)
      .force("charge", group ? null : forceManyBody<N>().strength(charge).distanceMin(20))
      .force("gravity-x", forceX<N>((n) => centers.get(n.id)?.x ?? 0).strength(0.12))
      .force("gravity-y", forceY<N>((n) => centers.get(n.id)?.y ?? 0).strength(0.12))
      .force("collide", forceCollide<N>((n) => n.r + 12).strength(1).iterations(3))
      .stop();
    if (!first) sim.tick(250); // settled before the first paint: no animation on load
    else { // (WEB-50) each cluster is laid out alone and then placed, so a run over the whole graph would be thrown away
      sim.alpha(sim.alphaMin()); // but left as cool as after that run, or the first drag would reshuffle everything
      const edges = ls as { source: N; target: N; weight: number }[];
      const clusters = components(ns, edges); // connected nodes, whatever the direction
      const clusterOf = new Map(clusters.flatMap((cluster, i) => cluster.map((node) => [node, i] as const)));
      const edgesByCluster = clusters.map(() => [] as typeof edges);
      for (const edge of edges) edgesByCluster[clusterOf.get(edge.source)!].push(edge);
      for (const [i, cluster] of clusters.entries()) {
        if (cluster.length === 1) continue; // nothing to arrange
        const own = forceSimulation(cluster)
          .force("link", forceLink<N, L>(edgesByCluster[i]).id((n) => n.id).distance(distance).strength(0.5))
          .force("charge", forceManyBody<N>().strength(charge).distanceMin(20))
          .force("x", forceX(0).strength(0.12)).force("y", forceY(0).strength(0.12))
          .force("group", group ? prefixForce<N>((n) => [group(n.id)]) : null) // (WEB-53)
          .force("collide", forceCollide<N>((n) => n.r + 12).strength(1).iterations(3)).stop();
        own.tick(250);
        own.alpha(1).tick(200); // reheated once: a large cluster ends with fewer crossings than after a single run
        if (group) reduceCrossings(cluster, edgesByCluster[i], (n) => n.r + 12); // (WEB-48) the collide radius: atoms never overlap
      }
      const placed = separate(clusters, 60, group && ((n: N) => group(n.id)));
      // grouped, gravity holds each node where it was placed; else at its cluster's centre
      centers = new Map(ns.map((n) => [n.id, group ? { x: n.x, y: n.y } : placed.get(n)!]));
    }
    sim.nodes(ns).force("gravity-x", forceX<N>((n) => centers.get(n.id)?.x ?? 0).strength(0.12))
      .force("gravity-y", forceY<N>((n) => centers.get(n.id)?.y ?? 0).strength(0.12));
    nodes = ns;
    links = ls;
    const publish = () => { ns.forEach((n) => pos.set(n.id, { x: n.x, y: n.y })); schedule(); };
    publish();
    if (first && ns.length) needsFit = true; // (WEB-55)
    sim.on("tick", publish);
    return () => { sim?.stop(); cancelAnimationFrame(raf); raf = 0; };
  });

  // a size or filter change repaints, and so does a theme change
  $effect(() => {
    void [w, h];
    readStyle();
  });
  $effect(() => {
    void [w, h, q, cycleOf];
    if (needsFit) fit();
    schedule();
  });
  $effect(() => {
    const repaint = () => { readStyle(); schedule(); };
    const mq = matchMedia("(prefers-color-scheme: dark)");
    const mo = new MutationObserver(repaint);
    mo.observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme"] });
    mq.addEventListener("change", repaint);
    return () => { mo.disconnect(); mq.removeEventListener("change", repaint); };
  });

  function schedule() {
    if (!raf) raf = requestAnimationFrame(() => { raf = 0; draw(); });
  }

  function fit() {
    if (!nodes.length || w <= 80 || h <= 140) return;
    needsFit = false;
    let [x0, x1, y0, y1] = [Infinity, -Infinity, Infinity, -Infinity];
    for (const n of nodes) [x0, x1, y0, y1] = [Math.min(x0, n.x - n.r), Math.max(x1, n.x + n.r), Math.min(y0, n.y - n.r), Math.max(y1, n.y + n.r)];
    const k = Math.min(2, (w - 80) / Math.max(1, x1 - x0), (h - 140) / Math.max(1, y1 - y0));
    view = { k, x: (-(x0 + x1) / 2) * k, y: (-(y0 + y1) / 2) * k - 25 };
    schedule();
  }
  function zoom(f: number, px = 0, py = 0) { // around a point relative to the centre
    const k = Math.min(8, Math.max(0.01, view.k * f));
    view = { k, x: px - ((px - view.x) * k) / view.k, y: py - ((py - view.y) * k) / view.k };
    schedule();
  }

  function readStyle() {
    if (!canvas) return;
    const css = getComputedStyle(canvas);
    const mono = css.getPropertyValue("--mono").trim() || "monospace";
    const labelSize = parseFloat(css.fontSize);
    // Resolve light-dark() values through the browser before passing them to canvas.
    const c = (v: string) => { canvas.style.color = `var(${v})`; return getComputedStyle(canvas).color; };
    const [fg, muted, ok, warn] = [c("--fg"), c("--muted"), c("--ok"), c("--warn")];
    colors = { fg, muted, ok, warn, mono, labelSize };
  }

  function draw() {
    const ctx = canvas?.getContext("2d");
    if (!ctx) return;
    const dpr = devicePixelRatio || 1;
    if (canvas.width !== Math.round(w * dpr) || canvas.height !== Math.round(h * dpr)) { canvas.width = Math.round(w * dpr); canvas.height = Math.round(h * dpr); }
    const { fg, muted, ok, warn, mono, labelSize } = colors;
    const needle = q.toLowerCase();
    const lit = (n: N) => !q || n === hover || n.id.toLowerCase().includes(needle); // (WEB-56)

    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, w, h);
    ctx.translate(w / 2 + view.x, h / 2 + view.y);
    ctx.scale(view.k, view.k);

    for (const l of links) {
      const [a, b] = ends(l);
      const cyclic = cycleOf.has(a.id) && cycleOf.get(a.id) === cycleOf.get(b.id);
      if (a === b) {
        ctx.globalAlpha = 0.6;
        ctx.strokeStyle = ctx.fillStyle = cyclic ? warn : muted;
        ctx.lineWidth = (cyclic ? 2 : 1) / view.k;
        const radius = Math.max(8 / view.k, a.r * 0.6);
        ctx.beginPath();
        ctx.arc(a.x, a.y - a.r - radius, radius, 0.25 * Math.PI, 2.75 * Math.PI);
        ctx.stroke();
        ctx.beginPath();
        ctx.moveTo(a.x - radius * 0.7, a.y - a.r - radius * 0.3);
        ctx.lineTo(a.x - radius, a.y - a.r - radius * 0.45);
        ctx.lineTo(a.x - radius * 0.8, a.y - a.r - radius * 0.65);
        ctx.fill();
        continue;
      }
      const [dx, dy] = [b.x - a.x, b.y - a.y];
      const d = Math.hypot(dx, dy) || 1;
      const [ux, uy] = [dx / d, dy / d];
      const [px, py, g] = [uy * 3, -ux * 3, 2]; // a pair A→B, B→A runs side by side, not on top of each other
      const [sx, sy, ex, ey] = [a.x + ux * a.r + px, a.y + uy * a.r + py, b.x - ux * (b.r + g) + px, b.y - uy * (b.r + g) + py];
      const lw = Math.min(2.5, 0.75 + 0.25 * Math.log2(Math.max(1, l.weight))) / view.k;
      ctx.globalAlpha = 0.6;
      ctx.strokeStyle = ctx.fillStyle = cyclic ? warn : muted;
      ctx.lineWidth = cyclic ? Math.max(lw, 2 / view.k) : lw;
      const hl = Math.min((5 + 2 * lw * view.k) / view.k, d / 3); // scale the head with the shaft, including on short links
      ctx.beginPath();
      ctx.moveTo(sx, sy);
      ctx.lineTo(ex - ux * hl * 0.8, ey - uy * hl * 0.8);
      ctx.stroke();
      ctx.beginPath();
      ctx.moveTo(ex, ey);
      ctx.lineTo(ex - ux * hl + uy * hl * 0.4, ey - uy * hl - ux * hl * 0.4);
      ctx.lineTo(ex - ux * hl * 0.8, ey - uy * hl * 0.8);
      ctx.lineTo(ex - ux * hl - uy * hl * 0.4, ey - uy * hl + ux * hl * 0.4);
      ctx.closePath();
      ctx.fill();
    }

    ctx.textAlign = "center";
    ctx.font = `${labelSize / view.k}px ${mono}`;
    ctx.lineJoin = "round";
    const labels: { x: number; y: number; w: number; h: number }[] = [];
    for (const n of nodes) {
      ctx.globalAlpha = lit(n) ? 1 : 0.18;
      ctx.beginPath();
      ctx.arc(n.x, n.y, n.r, 0, 2 * Math.PI);
      ctx.fillStyle = `hsl(${n.hue} 45% 55% / ${n === hover ? 0.5 : 0.26})`;
      ctx.shadowColor = `hsl(${n.hue} 60% 60%)`;
      ctx.shadowBlur = n === hover ? 14 : 0;
      ctx.fill();
      ctx.shadowBlur = 0;
      ctx.strokeStyle = cycleOf.has(n.id) ? warn : n.ring ? ok : `hsl(${n.hue} 45% 62% / 0.9)`;
      ctx.lineWidth = (cycleOf.has(n.id) ? 2.5 : n.ring ? 1.75 : 1) / Math.sqrt(view.k);
      ctx.stroke();
      if (n.r >= 14 || view.k > 1.3 || (q && lit(n))) {
        // the baseline sits just far enough down to clear the node: any lower and the name reads as the next node's
        const label = { x: n.x, y: n.y + n.r + (labelSize + 1) / view.k, w: ctx.measureText(n.id).width, h: labelSize / view.k };
        if ((q && lit(n)) || !labels.some((p) => Math.abs(p.x - label.x) < (p.w + label.w) / 2 + 4 / view.k && Math.abs(p.y - label.y) < Math.max(p.h, label.h))) {
          labels.push(label);
          ctx.fillStyle = fg;
          ctx.globalAlpha *= n === hover ? 1 : 0.85; // light grey on dark, dark grey on light
          ctx.fillText(n.id, label.x, label.y);
        }
      }
    }
  }

  // zoom around the cursor; a non-passive listener so the page does not scroll too
  $effect(() => {
    const onwheel = (e: WheelEvent) => {
      e.preventDefault();
      const r = canvas.getBoundingClientRect();
      zoom(Math.exp(-e.deltaY * 0.0015), e.clientX - r.left - w / 2, e.clientY - r.top - h / 2);
    };
    canvas.addEventListener("wheel", onwheel, { passive: false });
    return () => canvas.removeEventListener("wheel", onwheel);
  });

  const rel = (e: PointerEvent) => { const r = canvas.getBoundingClientRect(); return { x: e.clientX - r.left, y: e.clientY - r.top }; };
  const at = (p: { x: number; y: number }) => {
    const [x, y] = [(p.x - w / 2 - view.x) / view.k, (p.y - h / 2 - view.y) / view.k];
    for (let i = nodes.length - 1; i >= 0; i--) if (Math.hypot(nodes[i].x - x, nodes[i].y - y) <= nodes[i].r + 2) return nodes[i];
    return null;
  };

  // drag: a node moves (and pins while held), the background pans; a press that did not move is a click
  let dragging = false;
  function down(e: PointerEvent) {
    if (e.button !== 0) return;
    const n = at(rel(e));
    const [sx, sy, vx, vy] = [e.clientX, e.clientY, view.x, view.y];
    const [nx, ny] = [n?.x ?? 0, n?.y ?? 0];
    let moved = false;
    dragging = true;
    canvas.setPointerCapture(e.pointerId);
    const move = (m: PointerEvent) => {
      const [dx, dy] = [m.clientX - sx, m.clientY - sy];
      if (!moved && Math.hypot(dx, dy) > 3) { moved = true; if (n) sim?.alphaTarget(0.3).restart(); }
      if (!moved) return;
      if (n) { n.fx = nx + dx / view.k; n.fy = ny + dy / view.k; } else { view = { ...view, x: vx + dx, y: vy + dy }; schedule(); }
    };
    const up = () => {
      canvas.removeEventListener("pointermove", move);
      canvas.removeEventListener("pointerup", up);
      dragging = false;
      if (n) { n.fx = n.fy = null; sim?.alphaTarget(0); if (!moved) open(n.id); }
    };
    canvas.addEventListener("pointermove", move);
    canvas.addEventListener("pointerup", up);
  }
  function over(e: PointerEvent) {
    if (dragging) return;
    mouse = rel(e);
    hover = at(mouse);
    canvas.style.cursor = hover ? "pointer" : "";
    schedule();
  }
  function leave() {
    hover = null;
    schedule();
  }
</script>

<div class="graph" bind:clientWidth={w} bind:clientHeight={h}>
  <canvas bind:this={canvas} style:width="{w}px" style:height="{h}px" aria-label={label} onpointerdown={down} onpointermove={over} onpointerleave={leave}></canvas>
  {#if hover}
    <!-- (WEB-54) -->
    {@const t = tip(hover)}
    <div class="tip" role="tooltip" bind:offsetHeight={tipHeight} style:left="{Math.max(4, Math.min(mouse.x + 14, w - Math.min(384, w - 8) - 4))}px" style:top="{Math.max(4, Math.min(mouse.y + 14, h - tipHeight - 4))}px"><b>{t.title}</b>{#if t.meta} · {t.meta}{/if}<span>{t.detail}</span></div>
  {/if}
  <p class="legend muted">{legend} Scroll to zoom, drag to move, click to open.</p>
  <div class="zoom">
    <button type="button" aria-label="zoom in" onclick={() => zoom(1.4)}>+</button>
    <button type="button" aria-label="zoom out" onclick={() => zoom(1 / 1.4)}>−</button>
    <button type="button" aria-label="fit to view" onclick={fit}>⤢</button>
  </div>
</div>

<style>
  .tip { position: absolute; z-index: 1; width: min(24rem, calc(100% - 8px)); pointer-events: none; padding: 6px 10px; background: var(--tooltip-bg); color: var(--fg); border: 1px solid var(--muted); border-radius: var(--radius); font: var(--text-sm)/1.5 system-ui, sans-serif; }
  .tip span { display: -webkit-box; -webkit-box-orient: vertical; -webkit-line-clamp: 5; line-clamp: 5; overflow: hidden; overflow-wrap: anywhere; }
  .graph { position: relative; height: calc(100vh - 220px); min-height: 360px; background: var(--card); border: 1px solid var(--line); border-radius: var(--radius); overflow: hidden; }
  canvas { font-size: var(--text-sm); display: block; cursor: grab; touch-action: none; user-select: none; }
  canvas:active { cursor: grabbing; }
  .legend { position: absolute; left: 10px; bottom: 6px; right: 120px; margin: 0; font-size: var(--text-sm); pointer-events: none; }
  .zoom { position: absolute; right: 10px; bottom: 8px; display: flex; gap: 4px; }
  .zoom button { width: 28px; height: 28px; padding: 0; font-size: 15px; }
</style>

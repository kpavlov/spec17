<script lang="ts">
  // Module dependencies as a force-directed graph: d3-force lays it out, a plain 2D canvas draws it.
  // A node is a module whose area is proportional to its symbols; an arrow is "uses" from one module to another (width by edge count).
  import { untrack } from "svelte";
  import { forceCollide, forceLink, forceManyBody, forceSimulation, forceX, forceY, type Simulation, type SimulationNodeDatum } from "d3-force";
  import { components, prefixForce, reduceCrossings, separate } from "$lib/requirements/layout";

  type Mod = { path: string; text?: string; tests: number; symbols: Record<string, number>; depends_on: { module: string; edges: number }[] };
  type N = SimulationNodeDatum & { id: string; text?: string; r: number; n: number; tests: number; deg: number; dependents: number; hue: number; x: number; y: number };
  type L = { source: string | N; target: string | N; edges: number };

  let { modules, open, q = "", requirements = false, cycleGroups = [] }: { modules: Mod[]; open: (path: string) => void; q?: string; requirements?: boolean; cycleGroups?: string[][] } = $props();

  let w = $state(0);
  let h = $state(0);
  let canvas: HTMLCanvasElement;
  let view = { k: 1, x: 0, y: 0 }; // zoom and pan; the origin is the canvas centre
  let nodes: N[] = [];
  let links: L[] = [];
  let hover = $state.raw<N | null>(null);
  let mouse = $state.raw({ x: 0, y: 0 });
  let sim: Simulation<N, undefined> | undefined;
  let raf = 0;
  let needsFit = $state(false);
  const pos = new Map<string, { x: number; y: number }>(); // survives a reload, so the layout does not jump
  let centers = new Map<string, { x: number; y: number }>();
  let cycleOf = $derived(new Map(cycleGroups.flatMap((group, i) => group.map((id) => [id, i] as const))));
  let colors = { fg: "", muted: "", ok: "", card: "", line: "", warn: "", mono: "monospace", labelSize: 12 };

  const symbols = (m: Mod) => Object.values(m.symbols).reduce((a, b) => a + b, 0);
  // a colour per folder, so modules that live together look alike
  // green (90–180°) is skipped: it means "tests detected"
  const hueOf = (path: string) => (180 + ([...(path.includes("/") ? path.slice(0, path.lastIndexOf("/")) : path)].reduce((a, c) => (a * 31 + c.charCodeAt(0)) % 360, 7) * 270) / 360) % 360;
  const ends = (l: L) => [l.source as N, l.target as N] as const; // d3 swaps the ids for the nodes
  const prefix = (n: N) => n.id.split(/[-_]/, 1)[0];

  $effect(() => {
    const sizes = modules.map(symbols);
    const max = Math.max(1, ...sizes);
    const rmax = Math.max(14, Math.min(64, 320 / Math.sqrt(modules.length))); // the biggest module; the rest by area
    const first = untrack(() => pos.size === 0);
    const ns: N[] = modules.map((m, i) => ({ id: m.path, text: m.text, n: sizes[i], r: Math.max(4, rmax * Math.sqrt(sizes[i] / max)), tests: m.tests, deg: 0, dependents: 0, hue: hueOf(m.path), x: NaN, y: NaN, ...pos.get(m.path) }));
    ns.sort((a, b) => b.r - a.r); // big first, so small nodes are drawn (and hit) on top
    const byId = new Map(ns.map((n) => [n.id, n]));
    const ls: L[] = modules.flatMap((m) => m.depends_on.filter((d) => byId.has(d.module)).map((d) => ({ source: m.path, target: d.module, edges: d.edges })));
    for (const l of ls) for (const e of [l.source, l.target]) byId.get(e as string)!.deg++;
    for (const l of ls) byId.get(l.target as string)!.dependents++; // (WEB-54)
    sim?.stop();
    // (WEB-46) Connected hubs need more room; seed new nodes with d3's spread instead of piling them at the origin.
    sim = forceSimulation(ns)
      .force("link", forceLink<N, L>(ls).id((n) => n.id).distance((l) => 50 + (l.source as N).r + (l.target as N).r + 8 * Math.sqrt((l.source as N).deg + (l.target as N).deg)).strength(0.5))
      // like charges repel (more for a bigger module), gravity pulls everything to the middle, collision is a hard core: atoms never overlap
      .force("charge", forceManyBody<N>().strength((n) => -(80 + 0.5 * n.r * n.r)).distanceMin(20))
      .force("gravity-x", forceX<N>((n) => centers.get(n.id)?.x ?? 0).strength(0.12))
      .force("gravity-y", forceY<N>((n) => centers.get(n.id)?.y ?? 0).strength(0.12))
      .force("collide", forceCollide<N>((n) => n.r + 12).strength(1).iterations(3))
      .stop();
    sim.tick(250); // settled before the first paint: no animation on load
    if (first) { // (WEB-50)
      const edges = ls as { source: N; target: N; edges: number }[];
      const groups = components(ns, edges);
      const groupOf = new Map(groups.flatMap((group, i) => group.map((node) => [node, i] as const)));
      const edgesByGroup = groups.map(() => [] as typeof edges);
      for (const edge of edges) edgesByGroup[groupOf.get(edge.source)!].push(edge);
      for (const [i, group] of groups.entries()) {
        const groupEdges = edgesByGroup[i];
        forceSimulation(group)
          .force("link", forceLink<N, L>(groupEdges).id((n) => n.id).distance((l) => 50 + (l.source as N).r + (l.target as N).r + 8 * Math.sqrt((l.source as N).deg + (l.target as N).deg)).strength(0.5))
          .force("charge", forceManyBody<N>().strength((n) => -(80 + 0.5 * n.r * n.r)).distanceMin(20))
          .force("x", forceX(0).strength(0.12)).force("y", forceY(0).strength(0.12))
          .force("prefix", requirements ? prefixForce<N>((n) => [prefix(n)]) : null) // (WEB-53)
          .force("collide", forceCollide<N>((n) => n.r + 12).strength(1).iterations(3)).stop().tick(200);
        if (requirements) reduceCrossings(group, groupEdges); // (WEB-48)
      }
      centers = new Map([...separate(groups, 60, requirements ? prefix : undefined)].map(([n, center]) => [n.id, center]));
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
    const [fg, muted, ok, card, line, warn] = [c("--fg"), c("--muted"), c("--ok"), c("--card"), c("--line"), c("--warn")];
    colors = { fg, muted, ok, card, line, warn, mono, labelSize };
  }

  function draw() {
    const ctx = canvas?.getContext("2d");
    if (!ctx) return;
    const dpr = devicePixelRatio || 1;
    if (canvas.width !== Math.round(w * dpr) || canvas.height !== Math.round(h * dpr)) { canvas.width = Math.round(w * dpr); canvas.height = Math.round(h * dpr); }
    const { fg, muted, ok, card, line, warn, mono, labelSize } = colors;
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
      const lw = Math.min(2.5, 0.75 + 0.25 * Math.log2(Math.max(1, l.edges))) / view.k;
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
      ctx.strokeStyle = cycleOf.has(n.id) ? warn : n.tests > 0 ? ok : `hsl(${n.hue} 45% 62% / 0.9)`;
      ctx.lineWidth = (cycleOf.has(n.id) ? 2.5 : n.tests > 0 ? 1.75 : 1) / Math.sqrt(view.k);
      ctx.stroke();
      if (n.r >= 14 || view.k > 1.3 || (q && lit(n))) {
        const label = { x: n.x, y: n.y + n.r + 20 / view.k, w: ctx.measureText(n.id).width, h: labelSize / view.k };
        if ((q && lit(n)) || !labels.some((p) => Math.abs(p.x - label.x) < (p.w + label.w) / 2 + 4 / view.k && Math.abs(p.y - label.y) < Math.max(p.h, label.h))) {
          labels.push(label);
          ctx.fillStyle = fg;
          ctx.globalAlpha *= n === hover ? 1 : 0.85; // light grey on dark, dark grey on light
          ctx.fillText(n.id, label.x, label.y);
        }
      }
    }

    if (hover && !requirements) { // tooltip, in screen pixels
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      ctx.globalAlpha = 1;
      ctx.font = `${labelSize}px ${mono}`;
      ctx.textAlign = "left";
      const lines = [hover.id, `${hover.n} symbols · ${hover.tests} tests detected · ${hover.deg} links`];
      const [bw, bh] = [Math.max(...lines.map((t) => ctx.measureText(t).width)) + 16, lines.length * (labelSize + 4) + 10];
      const [bx, by] = [Math.max(4, Math.min(mouse.x + 14, w - bw - 4)), Math.max(4, Math.min(mouse.y + 14, h - bh - 4))];
      ctx.fillStyle = card;
      ctx.strokeStyle = line;
      ctx.lineWidth = 1;
      ctx.beginPath();
      ctx.roundRect(bx, by, bw, bh, 6);
      ctx.fill();
      ctx.stroke();
      ctx.fillStyle = fg;
      lines.forEach((t, i) => ctx.fillText(t, bx + 8, by + labelSize + 4 + i * (labelSize + 4)));
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
  <canvas bind:this={canvas} style:width="{w}px" style:height="{h}px" aria-label={requirements ? "requirement reference graph" : "module dependency graph"} onpointerdown={down} onpointermove={over} onpointerleave={leave}></canvas>
  {#if requirements && hover}
    <!-- (WEB-54) -->
    <div class="requirement-tip" role="tooltip" style:left="{Math.max(4, Math.min(mouse.x + 14, w - Math.min(384, w - 8) - 4))}px" style:top="{Math.max(4, Math.min(mouse.y + 14, h - 130))}px"><b>{hover.id}</b>{#if hover.dependents} · {hover.dependents} {hover.dependents === 1 ? "dependent" : "dependents"}{/if}<span>{hover.text}</span></div>
  {/if}
  <p class="legend muted">{requirements ? "Arrow: the requirement references the one it points to. Highlighted arrows and rings mark circular references." : "Arrow: the module uses the one it points to (width: edge count). Area: symbols. Colour: folder. Green ring: tests detected (not run)."} Scroll to zoom, drag to move, click to open.</p>
  <div class="zoom">
    <button type="button" aria-label="zoom in" onclick={() => zoom(1.4)}>+</button>
    <button type="button" aria-label="zoom out" onclick={() => zoom(1 / 1.4)}>−</button>
    <button type="button" aria-label="fit to view" onclick={fit}>⤢</button>
  </div>
</div>

<style>
  .requirement-tip { position: absolute; z-index: 1; width: min(24rem, calc(100% - 8px)); pointer-events: none; padding: 6px 10px; background: var(--tooltip-bg); color: var(--fg); border: 1px solid var(--muted); border-radius: var(--radius); font: var(--text-sm)/1.5 system-ui, sans-serif; }
  .requirement-tip span { display: -webkit-box; -webkit-box-orient: vertical; -webkit-line-clamp: 3; overflow: hidden; overflow-wrap: anywhere; }
  .graph { position: relative; height: calc(100vh - 220px); min-height: 360px; background: var(--card); border: 1px solid var(--line); border-radius: var(--radius); overflow: hidden; }
  canvas { font-size: var(--text-sm); display: block; cursor: grab; touch-action: none; user-select: none; }
  canvas:active { cursor: grabbing; }
  .legend { position: absolute; left: 10px; bottom: 6px; right: 120px; margin: 0; font-size: var(--text-sm); pointer-events: none; }
  .zoom { position: absolute; right: 10px; bottom: 8px; display: flex; gap: 4px; }
  .zoom button { width: 28px; height: 28px; padding: 0; font-size: 15px; }
</style>

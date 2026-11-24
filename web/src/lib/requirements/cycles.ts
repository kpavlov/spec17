// (WEB-47) Each group is a maximal set of mutually reachable requirements.
export function cycles(rows: { id: string; references: string[] }[]): string[][] {
  const rowsById = new Set(rows.map((r) => r.id));
  const edges = new Map(rows.map((r) => [r.id, r.references.filter((id) => rowsById.has(id))]));
  const reverse = new Map(rows.map((r) => [r.id, [] as string[]]));
  for (const [id, refs] of edges) for (const ref of refs) reverse.get(ref)!.push(id);
  const seen = new Set<string>();
  const order: string[] = [];
  for (const { id } of rows) {
    if (seen.has(id)) continue;
    seen.add(id);
    const stack: { id: string; next: number }[] = [{ id, next: 0 }];
    while (stack.length) {
      const top = stack[stack.length - 1];
      const refs = edges.get(top.id)!;
      if (top.next === refs.length) { order.push(top.id); stack.pop(); continue; }
      const next = refs[top.next++];
      if (!seen.has(next)) { seen.add(next); stack.push({ id: next, next: 0 }); }
    }
  }
  seen.clear();
  const groupOf = new Map<string, number>();
  const groups: string[][] = [];
  for (let i = order.length - 1; i >= 0; i--) {
    const id = order[i];
    if (seen.has(id)) continue;
    const group: string[] = [];
    const stack = [id];
    seen.add(id);
    while (stack.length) {
      const current = stack.pop()!;
      group.push(current);
      for (const next of reverse.get(current)!) if (!seen.has(next)) { seen.add(next); stack.push(next); }
    }
    if (group.length > 1 || edges.get(id)!.includes(id)) {
      const index = groups.length;
      groups.push([]);
      for (const member of group) groupOf.set(member, index);
    }
  }
  // Keep both members and groups in the input order.
  const ordered: string[][] = [];
  for (const { id } of rows) {
    const group = groupOf.get(id);
    if (group !== undefined) {
      if (!groups[group].length) ordered.push(groups[group]);
      groups[group].push(id);
    }
  }
  return ordered;
}

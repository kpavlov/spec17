/** (WEB-13) The shut groups after the edits (group → open); groups the edits do not name stay as they are. */
export function applyOpen(shut: string[], edits: Map<string, boolean>): string[] {
  const out = new Set(shut);
  for (const [group, open] of edits) if (open) out.delete(group); else out.add(group);
  return [...out];
}

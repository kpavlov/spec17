// (WEB-69) what the command palette matches: every typed word must appear in an entry; entries whose label says it come first
export type Entry = { label: string; hint?: string };

export function search<T extends Entry>(entries: T[], query: string, limit = 50): T[] {
  const q = query.trim().toLowerCase(), words = q.split(/\s+/).filter(Boolean);
  const rank = (e: Entry) => {
    const label = e.label.toLowerCase(), all = `${label} ${(e.hint ?? "").toLowerCase()}`;
    return !words.every((w) => all.includes(w)) ? -1 : label.startsWith(q) ? 0 : words.every((w) => label.includes(w)) ? 1 : 2;
  };
  // the sort is stable: equal ranks keep the order the entries came in
  return entries.map((e) => [rank(e), e] as const).filter(([r]) => r >= 0).sort((a, b) => a[0] - b[0]).slice(0, limit).map(([, e]) => e);
}

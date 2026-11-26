// (WEB-67) the problem kinds `report` counts, grouped by severity. Built from its `problems` map, so the groups always add up to the total:
// a kind this build does not know is listed as a warning under its own name.
const KINDS: Record<string, { title: string; error?: true }> = {
  unimplemented: { title: "Unimplemented: due, no test and no code", error: true },
  untested: { title: "Untested: due, cited in code but no test" },
  unproven_done: { title: "Check-marked done without proof" },
  later_but_built: { title: "Marked later but already has tests or code" },
  invalid_tests: { title: "Invalid tests", error: true },
  spec_issues: { title: "Spec issues", error: true },
  dangling_citations: { title: "Dangling citations" },
  specs_to_compare: { title: "Specs to compare" },
};

export type Problem = { key: string; title: string; n: number };

export function problemGroups(problems: Record<string, number>) {
  const found = Object.entries(problems).filter(([, n]) => n > 0).map(([key, n]) => ({ key, n, title: KINDS[key]?.title ?? key.replaceAll("_", " ") }));
  const group = (id: string, title: string, tone: "bad" | "warn", open: boolean, error: boolean) => {
    const list: Problem[] = found.filter((p) => !!KINDS[p.key]?.error === error);
    return { id, title, tone, open, list, n: list.reduce((sum, p) => sum + p.n, 0) };
  };
  return [group("errors", "Errors", "bad", true, true), group("warnings", "Warnings", "warn", false, false)].filter((g) => g.n);
}

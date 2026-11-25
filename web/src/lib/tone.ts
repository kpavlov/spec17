// One vocabulary for "how it stands": a tone is set with data-tone="…" on any element (app.css turns it into --tone), so pills, dots, badges,
// row edges and sections all colour the same way.
export type Tone = "info" | "plum" | "blue" | "celery" | "cyan" | "teal" | "orange" | "red" | "indigo" | "brown" | "ok" | "warn" | "bad" | "accent" | "muted";

export const requirementLevelTone: Record<string, Tone> = { MUST: "plum", SHALL: "plum", SHOULD: "blue", MAY: "celery" };
// (WEB-57) metadata colors are hints, not evidence states
// P0 hottest, then by number; anything else (a custom pattern's value) stays muted
export const priorityTone = (priority: string): Tone => {
  const n = /^P(\d+)$/i.exec(priority)?.[1];
  return n === undefined ? "muted" : (["red", "orange", "blue"] as Tone[])[+n] ?? "indigo";
};
export const effortTone = (effort: string): Tone => ({ XXXS: "cyan", XXS: "cyan", XS: "cyan", S: "teal", M: "blue", L: "orange", XL: "red", XXL: "red", XXXL: "red" })[effort.toUpperCase()] as Tone ?? "muted";

// what the scanner saw per requirement state
export const stateTone: Record<string, Tone> = { tested: "cyan", "src-only": "orange", none: "red", done: "celery", manual: "teal", later: "indigo", ref: "brown" };
// the row edge shows only the states that carry a verdict
export const edgeTone = (state: string): Tone | undefined => (["orange", "red"].includes(stateTone[state]) ? stateTone[state] : undefined);
// setup and doctor statuses
export const levelTone = (level: string): Tone => (level === "ok" || level === "warn" || level === "info" ? level : "bad");
// what happened to a file: read like a diff (added green, deleted red); the requirement pages keep red and amber for the requirement state
export const changeTone = (change: string): Tone => (change === "added" ? "ok" : change === "deleted" ? "bad" : "accent");

// shown wherever a requirement state appears: filters, badges, tooltips (what the scanner saw, not that anything passes)
export const stateLabel = (state: string) => (state === "tested" ? "tagged test" : state);
export const STATE_HINT: Record<string, string> = {
  all: "Every requirement",
  none: "No test tag and no source citation",
  "src-only": "Cited in source, no test tag",
  tested: "A test carries its [ID] tag (run the suite to know it passes)",
  manual: "Checked by a person",
  later: "Marked later or deferred",
  ref: "Reference document, never due",
};

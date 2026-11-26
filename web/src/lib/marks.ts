export type Marks = Record<string, string>;
/** Requirement ID → the commit it was marked reviewed at; undefined clears the mark. */
export type Edits = Map<string, string | undefined>;

/** (WEB-35) Applies the edits to `marks` in place and returns it; other requirements stay as they are. */
export function applyMarks(marks: Marks, edits: Edits): Marks {
  for (const [id, commit] of edits) if (commit) marks[id] = commit; else delete marks[id];
  return marks;
}

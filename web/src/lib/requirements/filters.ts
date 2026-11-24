// Single-value URLs remain readable; arrays preserve arbitrary text and an unset value.
export type Value = string | null;
export function readValues(value?: string): Value[] {
  if (!value) return [];
  if (value.startsWith("~")) {
    try {
      const parsed: unknown = JSON.parse(value.slice(1));
      if (Array.isArray(parsed) && parsed.every((v) => v === null || typeof v === "string")) return parsed;
    } catch { /* an old literal value */ }
  }
  return [value];
}
export function writeValues(values: Value[]): string | undefined {
  if (!values.length) return undefined;
  if (values.length === 1 && values[0] !== null && values[0] !== "all" && !values[0].startsWith("~")) return values[0];
  return `~${JSON.stringify(values)}`;
}
export function toggleValue(values: Value[], value: Value): Value[] {
  return values.includes(value) ? values.filter((v) => v !== value) : [...values, value];
}
// A click on a choice: it alone (or none, when it already is the only one), or added to / removed from the rest with a modifier key.
// Every choice selected filters nothing, so it reads as "All" (no values). A field with one choice can still be pinned to it.
export function collapseAll(values: Value[], choices: Value[]): Value[] {
  return values.length > 1 && choices.every((c) => values.includes(c)) ? [] : values;
}
export function pickValue(values: Value[], value: Value, additive: boolean): Value[] {
  if (additive) return toggleValue(values, value);
  return values.length === 1 && values[0] === value ? [] : [value];
}

export function matches(values: Value[], value: Value): boolean {
  return !values.length || values.includes(value);
}

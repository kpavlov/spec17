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

// (WEB-70) `state:none level:must text`: the words that name a facet and one of its `choices` (facet → values), and the text left over.
function wordValue(key: string, word: string, choices: Record<string, Value[]>): Value | undefined {
  if (!Object.hasOwn(choices, key)) return undefined;
  const flag = key === "changed" || key === "reviewed";
  if (flag) return word === "yes" ? "1" : word === "no" ? "0" : undefined;
  if (word === "none" && (key === "priority" || key === "effort")) return null;
  return choices[key].find((v): v is string => v !== null && v.toLowerCase() === word);
}
export function parseQuery(text: string, choices: Record<string, Value[]>): { picked: [string, Value][]; rest: string } {
  const picked: [string, Value][] = [], left: string[] = [];
  for (const word of text.split(/\s+/).filter(Boolean)) {
    const [, key, raw] = /^([^:]+):(.+)$/.exec(word) ?? [];
    const value = key ? wordValue(key.toLowerCase(), raw.toLowerCase(), choices) : undefined;
    if (value === undefined) left.push(word); else picked.push([key.toLowerCase(), value]);
  }
  // text with no filter word is left exactly as typed
  return { picked, rest: picked.length ? left.join(" ") : text };
}
// the picked words added to what their facets have selected; `choices` is all a facet offers, so all of them selected is All
export function mergePicked(picked: [string, Value][], selected: (key: string) => Value[], choices: (key: string) => Value[]): Record<string, string | undefined> {
  return Object.fromEntries([...new Set(picked.map(([key]) => key))].map((key) => {
    const values = [...new Set([...selected(key), ...picked.filter(([k]) => k === key).map(([, v]) => v)])];
    return [key, writeValues(collapseAll(values, choices(key)))];
  }));
}

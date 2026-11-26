// (WEB-65) a path cut in the middle to `max` characters: the start of the path and the file name (with its line) survive
export function middle(path: string, max = 28): string {
  if (path.length <= max) return path;
  const slash = path.lastIndexOf("/");
  const name = path.slice(slash + 1);
  const head = max - name.length - 2; // what is left beside "…/" and the name
  if (slash >= 0 && head > 0) return `${path.slice(0, head)}…/${name}`;
  // the name alone is too long: keep both of its ends
  const lead = slash < 0 ? "" : "…/";
  const keep = max - lead.length - 1;
  return `${lead}${name.slice(0, Math.ceil(keep / 2))}…${name.slice(name.length - Math.floor(keep / 2))}`;
}

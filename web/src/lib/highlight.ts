import hljs from "highlight.js/lib/common";
import xml from "highlight.js/lib/languages/xml";

// (WEB-08) Svelte combines HTML with template expressions and TypeScript or JavaScript blocks.
hljs.registerLanguage("svelte", (h) => {
  const language = xml(h);
  const script = language.contains?.find((mode) => typeof mode !== "string" && String(mode.begin) === String(/<script(?=\s|>)/));
  if (script && typeof script !== "string" && script.starts) script.starts.subLanguage = "typescript";
  language.contains?.unshift({ begin: /\{(?=[#/:@A-Za-z_$])/, end: /\}/, subLanguage: "typescript" });
  return language;
});

const esc = (s: string) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");

/** One HTML string per source line, highlighted by file extension (plain escaped text for an unknown language).
 *  Multi-line tokens (block comments, raw strings) are closed at each line end and reopened on the next, so rows stay independent. */
export function highlightLines(path: string, text: string): string[] {
  const ext = path.split(".").pop() ?? "";
  const html = hljs.getLanguage(ext) ? hljs.highlight(text, { language: ext, ignoreIllegals: true }).value : esc(text);
  const open: string[] = [];
  return html.split("\n").map((line) => {
    const prefix = open.join("");
    for (const m of line.matchAll(/<span[^>]*>|<\/span>/g)) m[0] === "</span>" ? open.pop() : open.push(m[0]);
    return prefix + line + "</span>".repeat(open.length);
  });
}

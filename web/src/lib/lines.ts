// highlighted line HTML with known requirement IDs turned into links (only text between tags is touched)
import { data } from "$lib/data.svelte";
import { u, req } from "$lib/nav.svelte";
import { highlightLines } from "$lib/highlight";

const linkIds = (html: string) => html.replace(/(<[^>]+>)|\b[A-Z]{2,5}-\d{1,4}\b/g, (m, tag) => (tag || !data.known.has(m) ? m : `<a class="id" href="${u(req(m))}">${m}</a>`));
export const codeLines = (code: { path: string; text: string } | null): string[] => (code ? highlightLines(code.path, code.text).map(linkIds) : []);

# spec17 tools and output formats

Read this when you need a tool's arguments, its CLI form, or how to read `status` and `trace` output. The MCP tool schemas carry the same arguments.

## Tools

| Tool | Arguments | Same as CLI |
|---|---|---|
| `report` | `limit` | `spec17 report [--limit N]` |
| `status` | `spec`, `prefix`, `all`, `missing`, `changed` (git ref), `staged`, `include_untracked`, `ignore_whitespace`, `built`, `done`, `unproven`, `issues`, `effort`, `limit` | `spec17 status [flags]` |
| `changes` | `ref`, `commit`, `staged`, `include_untracked`, `ignore_whitespace`, `show_ignored`, `show_all` | `spec17 changes [--ref R] [--commit C] [--staged] [--no-untracked]`: the changed files (added, modified, deleted, renamed with `old_path`; staged, unstaged, both, untracked or committed) and the requirements the changed lines touch, with a note per file (bullet edited or removed, test tag added or changed, citation added or removed, or a changed source file a tagged test reaches in the CodeGraph index, which is weaker: nothing there was edited). Edits that only re-wrap or re-indent are not listed. `ref=main` shows everything on a branch since main; `commit=<sha>` explains one commit; `show_all` adds files spec17 does not scan |
| `diff` | `path` (a file `changes` lists) and the arguments of `changes` | `spec17 diff PATH`, like `git diff REF -- PATH`, with a line under each hunk naming the requirements it touches |
| `history` | `base`, `head`, `limit` | `spec17 history`, like `git log BASE..HEAD` (base: main or master), newest first, plus how many files are uncommitted; explain one with `changes commit=<sha>` |
| `commits` | none | MCP only: per requirement, the newest commit that touched its bullet (git blame of the spec file): who last changed it and when, not that it was reviewed |
| `blame` | `path` (a scanned spec, test or source file), `lines` | MCP only: who last changed those lines |
| `trace` | `id` | `spec17 trace ID [--codegraph]` |
| `search_tests` | `module`, `state`, `query`, `requirement_id`, `changed` | MCP only: filter detected tests, including untagged ones. `requirement_id` matches a full tag ID case-insensitively |
| `search_requirements` | `test_file`, `line`, `end_line` | MCP only: requirements tagged in a test file; with `line`, the nearest detected test starting at or before it; with `end_line`, also tests starting up to it |
| `search_modules` | `query`, `changed` | MCP only: filter modules, with counts and dependency data when available |
| `overlap` | `path`, `all` | `spec17 overlap [PATH]` |
| `uncovered` | `path`, `fanin`, `min` | `spec17 uncovered [PATH]` |
| `file` | `name` (omit to list) | read `specs/NAME` |
| `source` | `path` | read a scanned test or source file |
| `dashboard` | `view` (e.g. `modules`, `changes`, `requirements/DOC-25`) | URL of the web dashboard |
| `doctor` | none | `spec17 doctor`: repo setup, CodeGraph binary and index |
| `codegraph` | `action` (init\|sync\|refresh) | `spec17 doctor --fix`: builds or refreshes the CodeGraph index. Writes files and is slow: only when the user asks |
| `module` | `path`, `hidden` (true/false) | hides a module (adds its folder to `.spec17ignore`) or shows it again. Writes a file: only when the user asks |
| `install_status` | none | whether the skill and MCP entry are installed here or globally |
| `install` | `global`, `port`, `server` (`spec17` or `codegraph`) | `spec17 install [--global]`. Writes files: only when the user asks |
| `uninstall` | `global`, `only` (`skill` or `mcp`), `server` | `spec17 uninstall [--global] [--only skill\|mcp]`. Writes files: only when the user asks |

CLI without a server: `spec17 [--root DIR] [--json] report|status|trace|changes|diff|history|overlap|uncovered`. Output is plain text, identical on every run for the same tree.

Resources: `spec17://modules` gives the module structure (nearest build manifest; with a CodeGraph index also symbols per kind and module dependencies). `spec17://tests` gives the detected test cases per module and file, each with its `[ID]` tags. Tests are detected by marker (`#[test]`, `@Test`, `def test_`, `it(` …), not run: a listed test does not say it passes.

## Report sections

| Section | Meaning |
|---|---|
| unimplemented | due requirements with no detected test tag or code citation |
| untested | due requirements cited in code but with no `[ID]` test |
| check-marked done without proof | ✅-style marks with `NO-TEST` or `NO-LINK` |
| marked later but already has tests or code | a test tag or source citation was found |
| invalid tests | tags for an ID no spec defines (`unknown`) or a struck-out one (`retired`); an ID tagged only in comments (`comment-only`), only on disabled tests (`disabled`), or only on tests with no direct assertion (`no-assert`, may assert through a helper) |
| comment-tagged | IDs in comments attached to detected tests; evidence to review, not a problem count or a passing result |
| spec issues | duplicate IDs, bullets without MUST/SHOULD/MAY (unless the ID is defined elsewhere), requirement headings without an ID, misplaced markers, citations of undefined IDs, a check mark with `(later)` |
| dangling citations | IDs cited by tests or source that no spec defines |
| specs to compare | declarations covered by two or more requirements whose specs do not cite each other: possible conflict (needs the CodeGraph index) |
| code with no requirement | the top of `uncovered` (needs the CodeGraph index) |

## status output

**Summary (no list flag):** one line per spec file, `tested/due`, then `src-only` (cited in source, no test), `none` (neither), and counts of `manual` and `later`, the latter with how many already have tests or code, e.g. `later: 15 (3 with tests/code)`. Consecutive IDs print as ranges (`DOC-01..05`). A `TOTAL` line closes it, followed by spec issues and a hint.

**Requirement lists** print one line per requirement: `ID  state  spec-file:line  [flags]  first 100 characters`. Several flags together narrow the list (all must hold); `spec`/`prefix` narrow it further.

| Flag | Lists |
|---|---|
| `all` | every requirement, in ID order |
| `missing` | due requirements with no test: state `src-only` or `none`. `later`, `deferred` and `manual` ones are not due |
| `changed` | requirements touched by the working tree against a git ref (default `HEAD`; staged, unstaged and untracked files count): `spec` = its bullet was edited, `test` = a test that tags it was edited, `src` = a source unit that cites it was edited (the whole file in a language without braces). `staged`, `include_untracked=false` and `ignore_whitespace` narrow it. "May be affected", not "changed for the better" |
| `built` | requirements marked `later`/`deferred` that already have a `[ID]` test or a source citation, flagged `[built?: N tests, M src]`. Evidence only |
| `done` | requirements carrying a check mark (✅ ✔ ☑ ✳) outside struck text, each with `[DONE: verdict]`. A hint, not proof |
| `unproven` | check-marked requirements whose verdict is `NO-TEST` or `NO-LINK` |
| `issues` | requirements with a spec issue, printed under the line |

Verdicts of a check-marked requirement:
- `NO-TEST`: no matching test tag detected.
- `NO-LINK`: tests exist, but the CodeGraph index cannot follow any of them, through helpers up to 3 hops, to app code. The test may prove nothing, so verify it.
- `ok`: a tagged test reaches app code. It shows the test calls the code, not that it checks the requirement.
- `unchecked`: no CodeGraph index.

States:
- `tested`: a `[ID]` tag exists. Run the suite to know whether it passes.
- `src-only`, `none`.
- `manual`: the bullet ends in `(manual)`, marking the requirement for manual verification.
- `later`: the bullet ends in `(later)` or `(deferred)`. A marker inside struck-through text is ignored.
- `ref`: a reference doc, either markdown linked as `[ADR-0003](decisions/0003-x.md)` or a file in `decisions/`/`adr/`. Never due, may be outdated, so read it before relying on it.

Spec issues:
- An ID defined twice.
- A bullet without MUST/SHOULD/MAY. It is not counted. A line naming an ID defined elsewhere is a reference, not an issue.
- A `(later)`/`(deferred)`/`(manual)` marker before the end of a requirement. It then counts as due.
- A citation of an undefined ID.
- A `[ID](path.md)` link to a missing file, or an ID linked to two files.
- A check mark together with a `(later)`/`(deferred)`/`(manual)` marker.

## What is scanned

- **Spec bullets.** Wrapped lines are joined. Struck-through bullets are retired.
- **Test tags.** `[ABC-12]`, `[ABC-12][ABC-14]` or `[ABC-12, ABC-14]`, anywhere in the test folders. A comment tag directly above or inside a detected test belongs to that test, even with annotations in between.
- **Source citations.** An ID following the configured comment marker on the same line. Svelte also accepts `<!-- (ABC-12) -->`. Multiline comment boundaries are not tracked.
- **Test bodies.** A matching CodeGraph function range is used when the index is at least as new as the test file. Otherwise boundaries come from language heuristics. Review attribution when boundaries matter.

## trace output

For one ID, `trace` gives:
- The full spec text and the IDs it cites.
- The tests tagged with it.
- Each source comment citing it, with the declaration it sits in.
- Related code: types named in the tagged tests' bodies (`*` = the file also cites the ID).
- A last line with a `codegraph explore` query.

Name matching misses code a test reaches only indirectly, so read the flow with `codegraph_explore` for behavioral questions. An ID unknown everywhere is an error.

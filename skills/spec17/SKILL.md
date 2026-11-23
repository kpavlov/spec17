---
name: spec17
description: >
  Report spec-driven-development (SDD) status deterministically with the spec17 MCP server: which requirement IDs (ABC-12 in specs/*.md) have tests tagged [ABC-12] and source citations, gaps in detected links, requirements linked to current changes, `later` requirements with links, check-marked ones missing links, code with no detected requirement, code covered by several requirements (conflicting or complementary specs), and one requirement traced to its tests and code. Use when asked to explore or report the SDD/spec status of a project, before changing behaviour (write the requirement first), after code changes (check the requirements still match and ask the user before editing them), when asked what is implemented or untested, or to find the code a requirement affects.
---

# Spec-driven with spec17

## Start with the question

| Question | First call | When to stop |
|---|---|---|
| What is the overall SDD status? | `report` | Summarize the scan and its gaps. |
| Which due requirements lack test tags? | `status missing=true` with the requested scope | Return the detected IDs or count. |
| What does one requirement say and which locations link to it? | `trace id=ABC-12` | Return the text and recorded links. |
| Which requirements does spec17 link to this change? | `changes` | Return the detected links. For actual behavioral impact, inspect the relevant diff. |
| Does this behavior exist or have a test? | `trace id=ABC-12` if the ID is known | For implementation, inspect the relevant code path. For coverage, inspect what the test exercises and asserts. Cite the location that answers the question. |

Reuse results already in context for the same tree. Keep scan counts intact. Tags and citations can be missing or wrong. If a trace leaves a behavioral question open, search by behavior or symbol in the likely module and its tests, including untagged tests. Follow helpers only as needed. Report any remaining uncertainty and reserve project-wide inspection for a requested audit. A passing result requires an observed test run. Verify the underlying evidence before changing a tag or marker.

## Exploring the project status

For a project-wide SDD summary, call `report` and relay it. It prints a headline, a one-line problem tally, and one section per problem kind (an empty one says `none`):

| Section | Meaning |
|---|---|
| unimplemented | due requirements with no detected test tag or code citation |
| untested | due requirements cited in code but with no `[ID]` test |
| check-marked done without proof | ✅-style marks with `NO-TEST` or `NO-LINK` |
| marked later but already has tests or code | a test tag or source citation was found |
| invalid tests | tags for an ID no spec defines (`unknown`) or a struck-out one (`retired`); an ID tagged only in comments (`comment-only`), only on disabled tests (`disabled`), or only on tests with no direct assertion (`no-assert`, may assert through a helper) |
| spec issues | duplicate IDs, bullets without MUST/SHOULD/MAY (unless the ID is defined elsewhere), requirement headings without an ID, misplaced markers, citations of undefined IDs, a check mark with `(later)` |
| dangling citations | IDs cited by tests or source that no spec defines |
| specs to compare | declarations covered by two or more requirements whose specs do not cite each other: possible conflict (needs the CodeGraph index) |
| code with no requirement | the top of `uncovered` (needs the CodeGraph index) |

Then:
1. Give the user the report (the headline, tally and every non-empty section), then your reading of it: the two or three problems that matter most and why. Attribute scan counts and ID lists to spec17.
2. For a follow-up, choose the narrowest relevant tool: `status`, `trace`, `overlap` or `uncovered`. Apply the verification guidance above to behavioral claims.
3. Use the requested depth. For a scan summary, stop at `low`. Never call two requirements conflicting from the report: see "Asked for conflicting requirements".

If something looks missing ("not checked", no tests found), run `doctor`: it reports the repo layout, whether the `codegraph` binary exists and its index is initialized and fresh, and what to run. The CodeGraph index (`.codegraph/codegraph.db`, opened read-only) adds test-to-code links, overlap and uncovered code. Without one those parts say "not checked": indexing is the user's decision, so do not create the index on your own.

## status — recorded requirement evidence

**Summary (no list flag):** one line per spec file, `tested/due`, then `src-only` (cited in source, no test), `none` (neither), and counts of `manual` and `later`, the latter with how many already have tests or code, e.g. `later: 15 (3 with tests/code)`. Consecutive IDs print as ranges (`DOC-01..05`). A `TOTAL` line closes it, followed by spec issues and a hint.

**Requirement lists** print one line per requirement: `ID  state  spec-file:line  [flags]  first 100 characters`. Several flags together narrow the list (all must hold), and `spec`/`prefix` narrow it further.

| Flag | Lists |
|---|---|
| `all` | every requirement, in ID order |
| `missing` | due requirements with no test: state `src-only` or `none`. `later`, `deferred` and `manual` ones are not due |
| `changed` | requirements touched by the working tree against a git ref (default `HEAD`; staged, unstaged and untracked files count): `spec` = its bullet was edited, `test` = a test file that tags it was edited, `src` = a source file that cites it was edited. `staged` narrows it to the index, `include_untracked=false` leaves untracked files out, `ignore_whitespace` drops whitespace-only edits. Matching is by line: the edited bullet, the tagged test, the code unit that carries the citation (the whole file in a language without braces). "May be affected", not "changed for the better"; use `changes` for the files and what happened to each |
| `built` | requirements marked `later`/`deferred` that already have a `[ID]` test or a source citation, flagged `[built?: N tests, M src]`. Evidence only: the state still needs assessing (below) |
| `done` | requirements carrying a check mark (✅ ✔ ☑ ✳) before the ID or anywhere in the bullet outside struck text. A hint, not proof. Each line gets `[DONE: verdict]` |
| `unproven` | check-marked requirements whose verdict is `NO-TEST` or `NO-LINK` |
| `issues` | requirements with a spec issue, printed under the line |

Verdicts of a check-marked requirement: `NO-TEST` (no matching test tag detected), `NO-LINK` (tests exist, but the CodeGraph index cannot follow any of them, through helpers up to 3 hops, to app code, so the test may prove nothing: verify), `ok` (a tagged test reaches app code), `unchecked` (no CodeGraph index). The graph link is a hint too: it shows a test calls the code, not that the test checks the requirement.

States: `tested` (a `[ID]` tag exists; run the suite to know it passes), `src-only`, `none`, `manual` (checked by a person), `later`, `ref` (a reference doc: markdown linked as `[ADR-0003](decisions/0003-x.md)`, or a file in `decisions/`/`adr/`; never due, may be outdated, so read it before relying on it). A requirement ending in `(later)` or `(deferred)` is `later`; a marker inside struck-through text is ignored.

**Spec issues** are: an ID defined twice, a bullet without MUST/SHOULD/MAY (not counted; a line that names an ID defined elsewhere, like a cross-reference table, is a reference and no issue), a `(later)`/`(deferred)`/`(manual)` marker before the end of a requirement (it then counts as due), a requirement citing an ID no spec defines, a `[ID](path.md)` link to a file that does not exist, an ID linked to two files, a check mark together with a `(later)`/`(deferred)`/`(manual)` marker.

Scanned: `specs/*.md` bullets (wrapped lines joined; struck-through bullets are retired); test tags `[ABC-12]`, `[ABC-12][ABC-14]` or `[ABC-12, ABC-14]` anywhere in the test folders. A comment tag directly above or inside a detected test belongs to that test, including when annotations intervene. Source IDs follow a line-comment marker of the file's language (block comments are not read). Folders and extensions come from `spec17.json` or detection.

In a profile, `case` is a regex matching a test marker, declaration or call. It replaces the built-in pattern, so include every form to retain, for example Swift's `^\\s*@Test\\b|^\\s*func\\s+(?:test\\w*|\u0060test[^\u0060]*\u0060)`. `call: true` takes the test name from the quoted string in a matched call such as `it("name", ...)`. `inline: true` includes source folders when finding tests, as for Rust's inline tests.

## trace — what a requirement touches

For one ID: the full spec text and the IDs it cites, the tests tagged with it, each source comment citing it with the declaration it sits in, and related code: types named in the tagged tests' bodies (`*` = the file also cites the ID). The last line is a `codegraph explore` query. For a behavioral question, use the output as a starting set, then read the relevant flow with `codegraph_explore`; name matching misses code a test reaches only indirectly. An ID unknown everywhere is an error.

## Assessing the real state — effort modes

Honor an explicit effort request. Otherwise use the depth the question calls for.

| Effort | Scope |
|---|---|
| `low` — scan results | Report detected IDs, counts or links directly. |
| `medium` — implementation or coverage | Verify the requested case yourself. Inspect every clause only for a whole-requirement verdict. |
| `high` — deep inspection | Within the requested scope, follow indirect calls where needed, check the assertions and run the relevant tests. Report results and any gaps. |

Handle focused reviews directly. For an explicitly broad review, independent requirements can be assessed in parallel. Reuse the scan and give each assessor only the relevant requirements and locations.

For a whole-requirement assessment, report `implemented` (every clause has working code and a test), `partial` (list the missing clauses), `stub` (cited or tagged, behaviour absent) or `not-built` when the relevant implementation confirms absence. Use `unknown` when the inspected evidence cannot settle a clause. Include the evidence in one or two lines. When updating markers, follow these rules:

- Marked later and `implemented`: remove the `(later)`/`(deferred)` marker, so it counts as due.
- Marked done and `implemented`: keep the check mark; write any missing test for a clause the code already satisfies (workflow step 4).
- Marked done but `partial`, `stub` or `not-built`: tell the user, then remove the check mark, or split the bullet into a done part and a `(later)` part. Do not leave a half-built requirement looking finished.
- Marked later and `partial`, `stub` or `not-built`: keep the marker; remove a misleading tag or citation.
- Edit specs on your own only for `implemented` marked later; confirm every other change with the user. Report one table: ID, verdict, one line of evidence.

## uncovered — code with no requirement

Reads the CodeGraph index and lists top-level types and functions in the source folders that no requirement covers, grouped by file and sorted by size, as `Name:first-last` line ranges. These are candidates for a **missing requirement**, not proof.

A unit is listed only if all of this holds, so shared plumbing does not drown the result:
- No spec ID is cited in a comment inside it or just above it.
- No test tagged `[ID]` names it in its body.
- Code that does cite an ID does not use it directly (one hop of calls, instantiates, references): a helper of a covered feature is covered.
- It is not shared infrastructure: used from fewer than `fanin` other files (default 5), and not under the `infra_paths` of `spec17.json`.
- It is `internal` or `public` and at least `min` lines (default 10).

The first line shows how many units each rule skipped. The index follows file edits within about a second; run `codegraph sync` first if code just changed.

**Triage each candidate before writing anything** (read it with `codegraph_explore "<Name>"`):
- *Business behaviour* — a rule, a computation, a saved or shown field, a user-visible flow or limit: write a requirement in the matching spec (the specs index, next free ID, never a retired one), test it with `[ID]`, and cite `(ID)` at the code.
- *Plumbing* — framework glue, layout, styling, wiring with no rule of its own, generic helpers, platform adapters: ignore it. If a whole kind keeps showing up, raise `fanin` or add it to `infra_paths` instead of writing requirements for it.
- *Belongs to an existing requirement* — add `(ID)` in a comment at the code; the spec already says it.

## Resources — modules and tests

Read the MCP resource `spec17://modules` for the repo's module structure (nearest build manifest; with a CodeGraph index also symbols per kind and which modules each depends on) and `spec17://tests` for the test cases found in the test files, grouped by module and file, each with the `[ID]` tags on it. Tests are detected by marker (`#[test]`, `@Test`, `def test_`, `it(` …), not run: a listed test does not say it passes. For an unusual marker, set `case` in that language's `profiles` entry of `spec17.json`.

## overlap — code several requirements cover

Lists declarations (method, function, type, property) that two or more requirements cover, from the CodeGraph index. A requirement covers a declaration when its ID is cited in a comment inside it or in the doc comment right above it. Each pair of IDs is marked:
- `linked`: one spec's text cites the other, or both cite a third ID. The specs already know each other, so they are probably complementary.
- `UNLINKED`: no declared relation. Read these first; the specs may conflict, repeat each other, or share code only by accident.

Pairs from the same spec file are hidden unless `all`. spec17 finds the overlap; deciding what it means is a judgement (`medium` for `complementary`, `duplicate` and `incidental`; any claim of `conflicting` needs the conflict review below). For each `UNLINKED` pair read both requirements (`trace`) and the declaration (`codegraph_explore "<Name>"`), then classify:

| Verdict | Test | Action |
|---|---|---|
| conflicting | they demand incompatible behaviour of this code: different limits, formats, defaults, ordering, or one forbids what the other requires | Only after the review below. Show the user both texts and the code. Never pick a side silently. After the user decides, fix the losing spec, its tests and the code |
| complementary | different aspects of one feature (one validates, one stores, one displays) or one refines the other | Add a reference in each bullet (`see ABC-12`), so the next run reports the pair as `linked` |
| duplicate | the same rule stated twice | Propose retiring one: strike it through and keep its ID reserved; the user approves; move its tests' tags to the survivor |
| incidental | the code is shared plumbing and one citation is only a loose pointer | Move that citation to the code that really implements it, or remove it |

### Asked for conflicting requirements: verify the concrete conflict

An `UNLINKED` pair is a candidate, not a finding: two IDs on one declaration only prove that someone cited both. When the user asks which requirements conflict, or when you are about to call a pair `conflicting`, do not answer from the tool output or from the spec texts alone. A conflict verdict requires review by the strongest available model. Perform that review directly when qualified. If that review is unavailable, report the pair as an unverified candidate. For each pair within scope:

1. **Understand the current code first.** Read the declaration and what it does (`codegraph_explore "<Name>"`), its callers and callees, the tests tagged with either ID (`trace`), and any other code that cites either ID. Say in a few lines what the code does today for each requirement: implements A only, B only, both, or neither.
2. **Read both requirements in full** and extract their conditions: which entity, state, input, version or phase each one applies to, and whether either is `(later)`, `(deferred)`, struck or "after 1.0".
3. **Try to reconcile them.** Look for a reading in which both hold: disjoint conditions, one a refinement or exception of the other, different layers (storage vs display), different moments (on save vs on read).
4. **Only if no reading reconciles them, write the contradiction as a concrete case**: one input or situation, and the two outputs the specs demand from it. No such case, no conflict.
5. **Report** per pair: verdict (`conflicting`, `complementary`, `duplicate`, `incidental`, or `unclear` with what is missing), the concrete case for a conflict, what the code does today, and the `file:line` evidence for each claim. Never infer behaviour from names or comments.

Then relay the verdicts to the user. A conflict is reported with both bullets quoted and the case; fixing it is the user's decision (specs first, then tests and code). A `linked` pair needs no action unless the texts disagree; check that when reading the code anyway. Report one table: declaration, IDs, verdict, one line of evidence.

## Specs first, specs in sync

The spec is what the user agreed to; the code is only evidence of what exists. Keep them in step, and never let the code quietly rewrite the spec.

- **Before changing behaviour**, `trace` the requirement (or `changes` to see what is already touched) and write or edit its bullet first, then the tests, then the code with `(ID)` citations. A new requirement: draft the bullet, show it to the user, add it. A change to an existing one: show the old and the new wording and wait for the user's OK.
- **After code or tests changed** (yours or the user's), `changes` lists detected links. Use the diff to check for missed or misleading links in the edited behavior. For each affected requirement, `trace` it and read the code. If the requirement no longer says what the code does, **do not edit it on your own**. Show the user the requirement text, what the code does now (`file:line`) and the wording you propose, and let them pick: update the requirement, or fix the code. Edit the spec only after they confirm. Never weaken MUST/SHOULD/MAY, add `(later)`, or delete a bullet to make a report green.
- **No specs yet** (`doctor` warns "no specs found", `report` says so in a note instead of reporting success, the dashboard opens on Settings): offer to draft `specs/01-….md` from what the user tells you, show it, and write it after they confirm. Do not invent requirements from the code and write them unasked.

## Workflow for changes

0. Specs marked `(later)` may already be built (`built`) and check-marked specs may not be (`unproven`): assess per effort (above).
1. Starting work: `changes` (or `status changed=HEAD`) shows which requirements the current changes already touch; `missing=true prefix=X` lists due requirements in that area without detected test tags.
2. Behaviour change: `trace`, read the spec and its tests, edit the spec first (see "Specs first, specs in sync": the user confirms an edit to an existing requirement).
3. New requirement: draft the bullet and show it, then add it, write the tests, and cite `(ID)` in a comment at the code that implements it.
4. Writing or changing a test: it must name the requirement it proves, in the form spec17 reads, square brackets with the ID:
   - Title: e.g. `@Test("[ABC-12] what holds")` (Swift Testing; use the test framework's own title or name). One test per behaviour; list several IDs as `[ABC-12][ABC-14]` or `[ABC-12, ABC-14]` only when one assertion set really proves both.
   - Comment: in the body, a `// [ABC-13] why this line proves it` beside each assertion or setup that proves a *different* requirement than the title, and above a test helper or fixture that exists for one. A bare `(ABC-12)` in a test comment is not read.
   - No ID fits: the behaviour is unspecified. Write the requirement first (step 3), then the test. Never tag a test with a nearby ID that does not say what it checks.
   - After editing tests, `status changed=HEAD` should list the requirement as `tested` with `[changed: test]`.
5. Several specs on one piece of code: `overlap`, classify per the table above.
6. Cleaning up: `uncovered path=Domain/` lists candidates with no detected requirement link; triage as above.
7. Before finishing: `status changed=HEAD missing=true` should be empty (no detected tag gaps among the reported due requirements) and `status issues=true` clean. Then `changes`: a requirement whose code you changed must still say what the code does; if not, ask the user before editing it.
8. For a visual overview open the dashboard (`dashboard` returns its URL).

## How to call it

**MCP (preferred).** Run `spec17` in the repo once (it starts the server for that repo and opens the dashboard; `spec17 serve --no-open` does not open it) and connect the client to `http://127.0.0.1:7411/mcp` (streamable HTTP):

```json
{"mcpServers": {"spec17": {"type": "http", "url": "http://127.0.0.1:7411/mcp"}}}
```

Add it to the client's MCP config if the spec17 tools are not available. `spec17 --root DIR serve` picks the repo; without it the server uses the directory it was started in.

| Tool | Arguments | Same as CLI |
|---|---|---|
| `report` | `limit` | `spec17 report [--limit N]` |
| `status` | `spec`, `prefix`, `all`, `missing`, `changed` (git ref), `staged`, `include_untracked`, `built`, `done`, `unproven`, `issues`, `effort`, `limit` | `spec17 status [flags]` |
| `changes` | `ref`, `commit`, `staged`, `include_untracked`, `ignore_whitespace`, `show_ignored`, `show_all` | `spec17 changes [--ref R] [--commit C] [--staged] [--no-untracked]` (or `spec17 status --changed`), plus the changed files (change: added, modified, deleted or renamed with `old_path`; stage: staged, unstaged, both, untracked or committed; kind) and the requirements the changed lines touch, with a note per file saying what happened (bullet edited or removed, test tag added or changed, citation added or removed, or a changed source file a tagged test reaches in the CodeGraph index, which is weaker: nothing there was edited). A bullet, test or cited unit that reads the same as in the base version once whitespace and wrapping are collapsed is not listed. `ref=main` shows everything on a branch since main (committed and not); `commit=<sha>` explains one commit; `show_all` adds files spec17 does not scan |
| `diff` | `path` (a file `changes` lists) and the arguments of `changes` | `spec17 diff PATH`, like `git diff REF -- PATH`: what exactly changed in one file, with a line under each hunk naming the requirements it touches |
| `history` | `base`, `head`, `limit` | `spec17 history`, like `git log BASE..HEAD`: the commits not on the base branch (main or master by default), newest first, and how many files are uncommitted; explain one with `changes commit=<sha>` |
| `commits` | none | MCP only: for each requirement the newest commit that touched it (sha, author, date, subject; git blame of its spec file): who last changed the specification and when, not that it was reviewed |
| `blame` | `path` (a scanned spec, test or source file), `lines` | MCP only: who last changed those lines (git blame), e.g. a test by the line `trace` lists for it |
| `trace` | `id` | `spec17 trace ID [--codegraph]` |
| `search_tests` | `module`, `state`, `query`, `requirement_id`, `changed` | MCP only: filter detected tests, including untagged tests. `requirement_id` matches a full tag ID case-insensitively |
| `search_requirements` | `test_file`, `line`, `end_line` | MCP only: find requirements tagged in the file. With a 1-based `line`, use the nearest detected test starting at or before it. With an inclusive `end_line`, also include tests starting after `line` through `end_line` |
| `search_modules` | `query`, `changed` | MCP only: filter modules, with counts and dependency data when available |
| `overlap` | `path`, `all` | `spec17 overlap [PATH]` |
| `uncovered` | `path`, `fanin`, `min` | `spec17 uncovered [PATH]` |
| `file` | `name` (omit to list) | read `specs/NAME` |
| `source` | `path` | read a scanned test or source file |
| `dashboard` | `view` (optional page, e.g. `modules`, `changes`, `requirements/DOC-25`) | URL of the web dashboard (at that page) |
| `doctor` | none | `spec17 doctor`: repo setup, CodeGraph binary and index |
| `codegraph` | `action` (init\|sync\|refresh) | builds or refreshes the CodeGraph index (`doctor --fix`); `refresh` syncs when an index exists, also beside a running codegraph; writes files, slow: only when the user asks |
| `module` | `path` (a module), `hidden` (true/false) | hides the module (adds its folder to `.spec17ignore` so scans leave it out) or shows it again (removes that line; the path is one of `hidden` in `spec17://modules`); writes a file: only when the user asks |
| `install_status` | none | whether the skill and MCP entry are installed here or globally |
| `install` | `global`, `port`, `server` (`spec17` default, or `codegraph` to add the CodeGraph entry to the repo's `.mcp.json`) | `spec17 install [--global]` (writes files: only when the user asks) |
| `uninstall` | `global`, `only` (`skill` or `mcp`), `server` (`spec17` default, or `codegraph` to remove the CodeGraph entry from the repo) | `spec17 uninstall [--global] [--only skill\|mcp]`. With `only=mcp`, remove the spec17 entry from the repo's `.mcp.json` or `~/.mcp.json` (writes files: only when the user asks) |

**CLI (no server needed).** `spec17 [--root DIR] [--json] report|status|trace|changes|diff|history|overlap|uncovered`. Output is plain text, identical on every run for the same tree.

**Configuration.** None is needed in the usual layouts: every specs folder that holds markdown (`specs/`, `spec/`, `docs/specs/`, `openspec/specs/` and each `openspec/changes/*/specs/` but not `archive/`, `.kiro/specs/`, `docs/superpowers/specs/`, a `docs/` holding requirement lines; requirements under an OpenSpec `## REMOVED` heading are retired, a change's version of a requirement replaces the base one, and a `### Requirement` without an ID is a spec issue) and the source and test folders and extensions of the dominant language are detected, from the CodeGraph index when there is one. Each file is read with the built-in profile of its own language (`swift`, `rust`, `typescript` (also JavaScript), `python`, `go`, `jvm` (Java, Kotlin, Scala), `csharp`, `ruby`, `php`, `c/c++`, `dart`, `lua`, `r`, `objective-c`, `solidity`, `erlang`, `vb.net`, `pascal`, `sql`): comment markers, declaration, assertion, switched-off, test-path and test-case regexes. A file of no known language is read like the dominant one. Symlinks count only inside the repo root or `ALLOWED_PATHS`. Anything else goes in an optional `spec17.json` at the repo root: `specs`, `tests`, `src`, `ext`, `profiles`, `retired_prefixes`, `codegraph`, `infra_paths`; a field you set replaces its detected value. For a second known language, list its extension in `ext` (and its folders in `src`/`tests`). `profiles` is a list of entries with the fields `name`, `ext`, `comment`, `decl`, `assert`, `off`, `test`, `inline`, `case`, `call`: one named like a built-in profile changes only the fields it sets, as in `{"name": "rust", "assert": "assert!|panic!"}`; any other name adds a language, and the regexes it leaves out match nothing. An unknown field is an error, and so are the old top-level `comment`, `decl`, `assert`, `off`, `case` and `test_files`.

## Requirement formats

`specs/` (or `SPEC.md`, or markdown with such lines) is the source of truth: each requirement is `- **ABC-12** MUST|SHOULD|MAY — text` (also `- ABC-12: text … MUST`, `*`/numbered lists, `| ABC-12 | MUST | text |` rows, `SHALL`, and `### ABC-12: title` headings or `**ABC-12** title` paragraphs with the level, or EARS `shall`, anywhere in the block; OpenSpec `### Requirement: ABC-12 name`; IDs like `FR-001`, `AC-1.2`, `REQ_001`, never `UTF-8`/`ISO-8601`), a test links to it by tagging its title with `[ABC-12]`, and source comments cite it as `(ABC-12)`. Change the spec first, then tests and code.

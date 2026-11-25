---
name: spec17
description: >
   Trace requirement IDs across specs, tests, code, and diffs. Use for coverage gaps,
   conflicting requirements, and interpreting spec17 reports or dashboard counts.
license: See LICENSE in the github.com/kpavlov/spec17 repository
metadata:
   version: 0.4.0
   author: Konstantin Pavlov
   tags: sdd, requirements, traceability, testing
---

# Spec-driven with spec17

spec17 scans a repo's specs (`- **ABC-12** MUST — text`), test tags (`[ABC-12]`) and source citations (`(ABC-12)`) and reports the links it detects. It is deterministic evidence, not truth.

## Start with the question

| Question | First call | When to stop |
|---|---|---|
| What is the overall SDD status? | `report` | Summarize the scan and its gaps. |
| Which due requirements lack test tags? | `status missing=true` with the requested scope | Return the detected IDs or count. |
| What does one requirement say and which locations link to it? | `trace id=ABC-12` | Return the text and recorded links. |
| Which requirements does spec17 link to this change? | `changes` | Return the detected links. For actual behavioral impact, inspect the relevant diff. |
| Which deferred requirements might already be built? | `status built=true` | Inspect the implementation and test assertions for each candidate before recommending a marker change. |
| Does this behavior exist or have a test? | `trace id=ABC-12` if the ID is known | Inspect the code path for implementation, or what the test exercises and asserts for coverage. Cite the location that answers it. |
| Do requirements conflict? Which code has no requirement? | `overlap` / `uncovered` | Read `references/conflicts.md` before classifying anything. |
| Are tests duplicated? | `trace` or `search_tests requirement_id=…` | See "Duplicate tests". |

Reviews are read-only. Report findings and proposed fixes. On an edit request, apply changes within its scope and ask when the intended requirement is unclear.

**Evidence rules.** They hold for every answer:
- **Keep scan counts and ID lists intact,** and attribute them to spec17.
- **Treat tags and citations as claims.** They can be missing or wrong. A tag says a test names the requirement, not that the test checks it. A listed test is not a passing test: a pass needs an observed test run.
- **Settle open questions by behavior.** If a trace leaves a behavioral question open, search by behavior or symbol in the likely module and its tests, including untagged tests. Follow helpers only as needed, and report what remains uncertain.
- **Reserve project-wide inspection for a requested audit.** Reuse results already in context for the same tree.
- **Do not edit to make a report green.** Verify the evidence before changing a tag or marker. Never weaken MUST/SHOULD/MAY, add `(later)`, or delete a bullet to clear a report.

## Exploring the project status

`report` prints a headline, a one-line problem tally, and one section per problem kind (an empty one says `none`):

See [Report sections](references/tools.md#report-sections) for the meaning of each finding.

1. Give the user the headline, the tally and every non-empty section. Then add your reading: the two or three problems that matter most, and why.
2. For a follow-up, use the narrowest tool: `status`, `trace`, `overlap` or `uncovered`.
   - The flags, states and verdicts are in `references/tools.md`.
   - For each kind of gap, separate requirements with no test tag (`untested`, `unimplemented`) from code with no requirement link (`uncovered`). Neither one proves a test is absent.
3. A scan summary stops at effort `low`. A pair under "specs to compare" is a candidate: never call it conflicting from the report.

## Assessing the real state: effort modes

Honor an explicit effort request. Otherwise use the depth the question calls for.

| Effort | Scope |
|---|---|
| `low` — scan results | Report detected IDs, counts or links directly. |
| `medium` — implementation or coverage | Verify the requested case yourself. Inspect every clause only for a whole-requirement verdict. |
| `high` — deep inspection | Within the requested scope, follow indirect calls where needed, check the assertions and run the relevant tests. Report results and any gaps. |

Handle focused reviews directly. For an explicitly broad review, independent requirements can be assessed in parallel: give each assessor only its requirements and locations.

For a whole-requirement review, report implementation and test coverage separately. Identify which clauses the code implements, which are missing, and which remain uncertain. Cite the code and the assertions that exercise each clause. Say whether you observed a test run.

For example, a session can expire at the specified timeout while the tagged test only checks login. Report the timeout implementation and the coverage gap separately. For a misleading done or deferred marker, explain which clauses justify changing it.

## Duplicate tests

spec17 does not detect duplicate tests; a shared tag only makes two tests candidates.
1. List the tests for an ID with `trace id=ABC-12` or `search_tests requirement_id=ABC-12`. Find untagged neighbours with `search_tests query=…`.
2. Read each candidate's setup, inputs and assertions (`source`, `codegraph_explore`).
3. Two tests are duplicates only when they exercise the same path with equivalent inputs and assert the same outcome. Report meaningful differences. If evidence is incomplete, identify what remains uncertain.
4. Propose which test to keep, and merge or remove the other only after the user agrees.

## Specs first, specs in sync

The spec is what the user agreed to; the code is only evidence of what exists. Never let the code quietly rewrite the spec. Resolve disagreements about intended behavior with the user before changing the spec.

1. **Starting work.** `changes` (or `status changed=HEAD`) shows which requirements the current changes touch. `status missing=true prefix=X` lists due requirements in that area without test tags.
2. **Behaviour change or addition.** Read the matching spec for the requirement that already owns the behaviour, and prefer updating it to creating one: when the bullet stays small and about one rule, keep its ID. `trace` it and read its tests. Update the bullet to describe the requested behavior. Then change the tests, then the code with `(ID)` citations.
3. **New requirement.** Only for a really new one: no requirement owns the behaviour, or adding it would make a bullet long or give it a second rule. Draft the bullet in the matching spec (next free ID, never a retired one) using the agreed behavior. Then write the tests and cite `(ID)` at the code.
4. **Writing a test.** Name the requirement it proves, in square brackets:
   - In the title: `@Test("[ABC-12] what holds")`, or the framework's own name. Write one test per behaviour. Use `[ABC-12][ABC-14]` only when one assertion set really proves both.
   - In the body: `// [ABC-13] why this proves it` beside an assertion that proves a *different* requirement, or above a helper that exists for one. A bare `(ABC-12)` in a test is not read.
   - If no ID fits, the behaviour is unspecified: specify it first (step 2 or 3). Never borrow a nearby ID.
   - Afterwards, `status changed=HEAD` should list the requirement as `tested` with `[changed: test]`.
5. **After code or tests changed** (yours or the user's):
   - Run `changes`, and use the diff to spot missed or misleading links.
   - `trace` each affected requirement and read the code.
   - If the requirement no longer says what the code does, show the user the text, what the code does now (`file:line`), and your proposed wording. Let them choose between updating the spec and fixing the code.
6. **Before finishing.** `status changed=HEAD missing=true` and `status issues=true` show gaps to report within the requested scope. A requirement whose code changed must still say what the code does.
7. **No specs yet.** `doctor` warns "no specs found". Offer to draft `specs/01-….md` from what the user tells you, and write it after they confirm. Never invent requirements from the code.

Several requirements on one piece of code (`overlap`) and code with none (`uncovered`) are judgement calls: follow `references/conflicts.md`. For a visual overview, `dashboard` returns the web dashboard's URL.

## Limitations

Test-to-code links, `overlap` and `uncovered` depend on a readable CodeGraph index. Report “not checked” as an unknown result. For a question about a specific behavior, inspect its source and tests directly.

## Troubleshooting

If `trace` returns an unknown-ID error, check the ID against the spec text. If the ID is present or expected tests are missing, run `doctor` to check the detected layout and consult `references/config.md` for supported formats.

For an unchecked graph result, use `doctor` to inspect index availability and freshness. Run indexing or installation only when requested.

## Prerequisites and calling

**MCP (preferred).** Run `spec17` in the repo. It serves that repo and opens the dashboard; `spec17 serve --no-open` skips the dashboard, and `--root DIR` picks another repo. Then connect the client over streamable HTTP:

```json
{"mcpServers": {"spec17": {"type": "http", "url": "http://127.0.0.1:7411/mcp"}}}
```

If MCP is unavailable, try the CLI. Change client configuration only for a setup request.

**CLI (no server).** `spec17 [--root DIR] [--json] report|status|trace|changes|diff|history|overlap|uncovered`. Output is identical on every run for the same tree.

**Reference files.** Read one when the task needs it:
- `references/tools.md`: every tool's arguments and CLI form, `status`/`trace` output, states, verdicts, spec issues, and what is scanned.
- `references/conflicts.md`: `overlap` classification, the conflict review, and `uncovered` triage.
- `references/config.md`: requirement formats, detected layouts, and the optional repo config with its language profiles.

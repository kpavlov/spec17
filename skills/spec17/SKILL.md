---
name: spec17
description: >
   Trace written requirements to code, tests and diffs by ID (ABC-12) or behavior.
   Use for requirement coverage gaps, duplicate tests for one rule, conflicting specs,
   branch impact on requirements, and spec17 setup or reports. Skip standalone test
   writing, Git merge conflicts and general requirements advice.
license: See LICENSE in the github.com/kpavlov/spec17 repository
metadata:
   version: 0.5.0
   author: Konstantin Pavlov
   tags: sdd, requirements, traceability, testing
---

# Spec-driven with spec17

spec17 scans a repo's specs (`- **ABC-12** MUST — text`), test tags (`[ABC-12]`) and source citations (`(ABC-12)`) and reports the links it detects. Use these links to locate the evidence for the user’s question.

## Choose the first call

Use the row that answers the request. Calls below use MCP arguments. For example, `status missing=true prefix=BILL` is `spec17 status --missing --prefix BILL` on the CLI. See [tools and CLI flags](references/tools.md) for other mappings.

| Question | First call | When to stop |
|---|---|---|
| What is the overall SDD status? | `report` | Summarize the scan and its gaps. |
| Which due requirements lack test tags? | `status missing=true` with the requested scope | For a count, use the reported total of matching rows. For every ID, check that the list is complete. |
| What does one requirement say and which locations link to it? | `trace id=ABC-12` | Return the text and recorded links. |
| Which requirements does spec17 link to this change? | `changes` (a branch: `changes ref=main`) | Return the detected links and the comparison base. For actual behavioral impact, read the diff (`diff path=…` with the same comparison base) against each touched requirement's text. Changed tests are not proof that new behavior is authorized. |
| Which deferred requirements might already be built? (dashboard "later but built") | `status built=true` | Inspect the implementation and test assertions for each candidate before recommending a marker change. |
| Does this behavior satisfy a written rule? | Known ID: `trace id=ABC-12`. Otherwise use `file` to list specs and read the one that covers the behavior. | Match the rule’s conditions to the code path and assertions. If ownership stays ambiguous, report the candidate rules and what would distinguish them. |
| Do requirements conflict (dashboard "specs to compare")? Which code has no requirement? | `overlap` / `uncovered` | Follow the [conflict review and uncovered-code triage](references/conflicts.md). |
| Are tests duplicated? | `trace` or `search_tests requirement_id=…` | See "Duplicate tests". |

Reviews are read-only. Report findings and proposed fixes. On an edit request, apply changes within its scope and ask when the intended requirement is unclear.

## Decide how much evidence to collect

| User asks for | Work to do | What to report |
|---|---|---|
| Scan or dashboard summary (low) | Use the scan for the requested scope. For a project report, include the headline, tally and each nonempty [report section](references/tools.md#report-sections). | Attribute the counts and IDs to spec17. Identify the most important gaps. |
| Implementation or coverage (medium) | Read the requirement, the implementation path and the assertions. Search by behavior or symbol for relevant untagged tests. | Cite the lines that answer the question. For a whole requirement, distinguish implemented, missing and uncertain clauses from test coverage. |
| Deep inspection (high) | Follow indirect calls and run relevant tests within scope. | Give the command, observed outcome and remaining uncertainty. |

Honor the requested depth. The `status --effort` flag controls printed detail. A tag records a claim. A passing result requires an observed run. Reserve repo-wide inspection for an audit request.

## Examples

**“Do idle sessions actually expire when the spec says they should?”** The user has no ID. List the specs with `file`. Read the authentication spec using the returned path as its `name`, or search the spec text locally for “idle”, “inactivity” and “expire”. The matching bullet in this example is AUTH-07. It specifies 900 seconds of inactivity and carries a deferred marker. If no bullet matches, report the behavior as unspecified in the specs searched. For this repo, `status built=true` includes:

```text
[built?: 1 tests, 1 src]  expire a session after 900 seconds of inactivity. (later)
```

Run `spec17 trace AUTH-07`, read the cited function and assertion, then search for other calls to `active`. A completed review for this repo reads:

> AUTH-07's predicate returns false at 900 seconds (line 3 of the fixture’s auth module). Its only caller here is a test that asserts the session is active at zero seconds (line 5 of its login test). I found no expiry-boundary assertion or production caller that expires a session. Keep the deferred marker until that path is verified. I inspected the tests without running them.

**Which billing requirements lack test tags?** Run `status missing=true prefix=BILL`. If the response starts “first 200 of 237 listed”, the count is 237. For an ID inventory, repeat with the same filters and `limit=237`, or run `spec17 status --prefix BILL --missing`. Check that all 237 IDs were returned. If retrieval remains incomplete, report the number retrieved and the total. A row marked `src-only` means spec17 found a source citation and no test tag. To answer whether tests exercise the rule, inspect the billing tests.

**Dashboard counts.** “Specs to compare 3, later but built 2. Which are real problems?” Preserve the count of three declarations to compare and the two deferred IDs. A declaration can contain several pairs. For each pair, trace both requirements and apply the [conflict review](references/conflicts.md) to the shared declaration. For each deferred ID, inspect the code and assertions before proposing a marker change. Return one row per pair or ID with its verdict, evidence location and any missing evidence.

**Branch impact.** “Review feature/x against main. REF-04 caps a refund at the remaining balance.” With feature/x checked out, run `changes ref=main`, then `diff path=… ref=main` for the changed source and tests. Follow affected callers. Report the base, detected IDs and any further IDs connected by behavior. For example: “With 200 remaining, REF-04 allows a request of 150. The new cap refunds 100 before fees. Decide whether to restore the balance-only limit or amend REF-04 and its tests.” Cite the changed lines for a real review.

## Duplicate tests

spec17 does not detect duplicate tests; a shared tag only makes two tests candidates.
1. List the tests for an ID with `trace id=ABC-12` or `search_tests requirement_id=ABC-12`. Find untagged neighbours with `search_tests state=untagged query=…` (query matches the path or test name).
2. Read each candidate's setup, inputs and assertions (`source`, `codegraph_explore`).
3. Two tests are duplicates only when they exercise the same path with equivalent inputs and assert the same outcome. Report meaningful differences. If evidence is incomplete, identify what remains uncertain.
4. Propose which test to keep if both cover the same case. Merge or remove it only for an approved edit.

For example, two inventory tests both tagged INV-04 can reject an oversized reservation. If one also checks that stock stayed unchanged, keep that assertion when proposing a merge.

## Specs first, specs in sync

1. **Find the owner.** Use `changes` and the matching spec. Trace the existing ID and inspect its tests. Keep its ID if the updated bullet still states one rule. Where the diff and spec disagree, show the user the text, observed behavior, and decision needed.
2. **Write the agreed behavior first.** Create an ID only for a distinct rule. Check the candidate against existing specs, struck bullets and OpenSpec REMOVED sections. Check deleted specs in Git history when relevant (`git log -S "ABC-12" -- <spec-path>`). Keep retired IDs reserved. In a new repo, start with a rule supplied by the user.
3. **Update the tests, then the code.** Tag the test title with `[ABC-12]` in the framework’s supported form. Use multiple tags only if the assertions exercise each rule. A comment tag beside an assertion can identify another requirement it exercises. Use `(ABC-12)` in a source comment at the implementation. Keep requirement levels and markers faithful to the agreed behavior.
4. **Verify.** Run the affected tests unless the user reserved that step. On failure, compare expected and actual behavior with the agreed rule, fix the mismatch within scope, and rerun. Check `changes` and `trace` on the final tree for missed or misleading links. Report the test command and outcome, any in-scope gaps from `status changed=HEAD missing=true` and `status issues=true`, and any unexecuted check.

## Limitations

Test-to-code links, `overlap` and `uncovered` depend on a readable CodeGraph index. Report “not checked” as an unknown result. For a question about a specific behavior, inspect its source and tests directly.

## Troubleshooting

If `doctor` reports “no specs found”, check the root and detected layout. For first-time setup, write the agreed rule in a specs folder when requested, then run `status` to confirm detection.

If `trace` returns an unknown-ID error, check the ID against the spec text. If the ID is present or expected tests are missing, run `doctor` to check the detected layout and consult `references/config.md` for supported formats.

For an unchecked graph result, use `doctor` to inspect index availability and freshness. Run indexing or installation only when requested.

**Configuration repair example.** A repo sets `"tests":"checks"` and startup reports that a sequence was expected. The field takes a list of folders. For a repair request, change just that value to `"tests":["checks"]`, run `spec17 --root DIR doctor` to verify detection, then repeat the original trace. For other fields or a parse location, consult [formats and configuration](references/config.md).

If a command rejects an argument, check `spec17 --version` and `spec17 <command> --help`, or the connected MCP tool's input schema. Use the supported spelling and keep the requested comparison base and filters. If the installed version cannot express them, inspect that diff or scope directly and say which check you performed.

If the spec17 MCP tools are missing or the server does not answer, try the CLI (`spec17 --version`). If the binary is missing too, say so and point the user to the setup steps below or https://spec17.dev/quickstart/; meanwhile answer from the spec, code and tests directly and mark the result as not from spec17.

## Prerequisites and calling

The command and tool reference follows spec17 0.5.0. Check the installed version with `spec17 --version`.

**First-time setup.** Check `spec17 --version`. On macOS with Homebrew, the install command is `brew install kpavlov/spec17/spec17`. Then run `spec17 --root DIR doctor` and confirm the repo path and detected spec, source and test locations. CLI queries work at this point.

**Connect MCP.** Run `spec17` in the repo. It serves that repo and opens the dashboard; `spec17 serve --no-open` serves the dashboard without launching a browser, and `--root DIR` picks another repo. Then connect the client over streamable HTTP:

```json
{"mcpServers": {"spec17": {"type": "http", "url": "http://127.0.0.1:7411/mcp"}}}
```

For a setup request, reconnect the client and call its spec17 `doctor` tool. Check that it reports the intended repository. If the port is occupied, start `spec17 serve --no-open --port 7412` on an available port and use that port in the client URL. If binding is denied, use the CLI in the same repo and report the connection error.

**CLI (no server).** `spec17 [--root DIR] [--json] report|status|trace|changes|diff|history|overlap|uncovered`. The CLI reads the repo directly.

**References**

- [Tools and output](references/tools.md) lists arguments, CLI alternatives, report sections and scan states.
- [Conflict review](references/conflicts.md) covers shared declarations and code with no requirement link.
- [Formats and configuration](references/config.md) explains missing IDs, test discovery and layout overrides.

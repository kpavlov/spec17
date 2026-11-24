# Overlap, conflicts and uncovered code

Read this before you classify an `overlap` pair, call two requirements conflicting, or triage `uncovered` candidates.

## overlap: code several requirements cover

`overlap` lists declarations (method, function, type, property) that two or more requirements cover, from the CodeGraph index. A requirement covers a declaration when its ID is cited in a comment inside it or in the doc comment right above it. Each pair of IDs is marked:
- `linked`: one spec's text cites the other, or both cite a third ID. The specs know each other, so they are probably complementary.
- `UNLINKED`: no declared relation. Read these first; the specs may conflict, repeat each other, or share code only by accident.

Pairs from the same spec file are hidden unless `all` is set. spec17 finds the overlap; deciding what it means is a judgement.
- Effort `medium` is enough for `complementary`, `duplicate` and `incidental`.
- A claim of `conflicting` needs the conflict review below.

Treat the actions below as proposals during a review. For each `UNLINKED` pair, read both requirements (`trace`) and the declaration (`codegraph_explore "<Name>"`), then classify:

| Verdict | Test | Action |
|---|---|---|
| conflicting | they demand incompatible behaviour of this code: different limits, formats, defaults, ordering, or one forbids what the other requires | Only after the review below. Show the user both texts and the code. Never pick a side silently. After the user decides, fix the losing spec, its tests and the code |
| complementary | different aspects of one feature (one validates, one stores, one displays) or one refines the other | Add a reference in each bullet (`see ABC-12`), so the next run reports the pair as `linked` |
| duplicate | the same rule stated twice | Propose retiring one: strike it through and keep its ID reserved; the user approves; move its tests' tags to the survivor |
| incidental | the code is shared plumbing and one citation is only a loose pointer | Move that citation to the code that really implements it, or remove it |

## Conflict review

An `UNLINKED` pair is a candidate, not a finding: two IDs on one declaration only prove that someone cited both. When the user asks which requirements conflict, or before you call a pair `conflicting`, do not answer from the tool output or the spec texts alone.

The review needs strong enough model. If you are not that model and cannot hand the review to it, report the pair as an unverified candidate.
For each pair within scope:

1. **Understand the current code first.** Read:
   - the declaration, its callers and its callees. The CodeGraph MCP server provides `codegraph_explore` (spec17 setup option `server=codegraph`). If the tool is unavailable, read the declaration and follow the calls in source;
   - the tests tagged with either ID (`trace`);
   - any other code that cites either ID.

   Then say in a few lines what the code does today: it implements A only, B only, both, or neither.
2. **Read both requirements in full.** Extract their conditions: which entity, state, input, version or phase each applies to, and whether either is `(later)`, `(deferred)`, struck or "after 1.0".
3. **Try to reconcile them.** Look for a reading in which both hold: disjoint conditions, a refinement or exception, different layers (storage vs display), or different moments (on save vs on read).
4. **Only if no reading reconciles them, write the contradiction as a concrete case**: one input or situation, and the two outputs the specs demand from it. No such case, no conflict.
5. **Report per pair:**
   - the verdict (`conflicting`, `complementary`, `duplicate`, `incidental`, or `unclear` with what is missing);
   - the concrete case, for a conflict;
   - what the code does today;
   - `file:line` evidence for each claim.

   Never infer behaviour from names or comments.

Report one table: declaration, IDs, verdict, one line of evidence. Quote both bullets for a conflict. Fixing it is the user's decision: specs first, then tests and code. A `linked` pair needs no action unless the texts disagree.

## uncovered: code with no requirement

`uncovered` reads the CodeGraph index and lists top-level types and functions in the source folders that no requirement covers. They are grouped by file and sorted by size, as `Name:first-last` line ranges. These are candidates for a **missing requirement**, not proof.

A unit is listed only if all of this holds, so shared plumbing does not drown the result:
- No spec ID is cited in a comment inside it or just above it.
- No test tagged `[ID]` names it in its body.
- No code that cites an ID uses it directly (one hop: calls, instantiates, references). A helper of a covered feature counts as covered.
- It is not shared infrastructure: it is used from fewer than `fanin` other files (default 5) and is not under the `infra_paths` of `spec17.json`.
- It is `internal` or `public` and at least `min` lines long (default 10).

The first line shows how many units each rule skipped. If recent edits may be missing, check index freshness with `doctor`. Sync only on request.

During a review, recommend the appropriate action for each candidate (read it with `codegraph_explore "<Name>"`):
- **Business behaviour**: a rule, a computation, a saved or shown field, a user-visible flow or limit.
  - Draft a requirement for the matching spec (next free ID, never a retired one) and show it to the user.
  - After they agree, test it with `[ID]` and cite `(ID)` at the code.
- **Plumbing**: framework glue, layout, styling, wiring with no rule of its own, generic helpers, platform adapters. Ignore it. If a whole kind keeps showing up, raise `fanin` or add it to `infra_paths` instead of writing requirements for it.
- **Belongs to an existing requirement**: add `(ID)` in a comment at the code; the spec already says it.

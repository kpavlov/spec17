---
title: Specs and tests
weight: 20
---

spec17 recognises requirement IDs in markdown, test tags in test code, and citations in source comments, then reports where they meet.

## Requirement IDs

An ID is a prefix of 2 to 10 capital letters or digits (the first is a letter), a `-` and 1 to 5 digits, with optional dotted parts: `ABC-12`, `REQ-0001`, `FR-001`, `AC-1.2`. An underscore works too, dotted parts included: `REQ_001`, `REQ_001.2`. A non-breaking hyphen (U+2011) reads as `-`.

Well-known tokens that look like IDs are never requirements: `UTF-8`, `ISO-8601`, `TLS-1.3`, `SHA-256`, `RFC-2119`, `CVE-2024` and similar. Only these shapes are skipped (the list is `not_id` in `core/resources/spec_format.toml`), so `HTTP-01` and `RFC-1` are still IDs.

A requirement also has a level: `MUST`, `SHOULD`, `MAY` or `SHALL`. Heading and paragraph styles accept the EARS `shall` in lower case too; bullets and table rows do not, and `should` and `may` must always be capitals.

A priority hint is a `P` followed by digits in square brackets or parentheses, such as `[P1]` or `(P2)`, or a labelled `(Priority: P1)`. An effort hint is an upper-case T-shirt size in brackets such as `[M]` or `(XL)`, or a labelled estimate such as `[effort: 3 points]`, `(size: M)` or `[estimate: 2 days]`. A hint must follow whitespace, so `p95 latency`, `requirement(s)`, `P2P` and a code-quoted `` `[M]` `` are not hints. Listings and traces expose both as separate fields, and keep them in the text. Labelled hints take precedence over unlabelled hints; otherwise the order of patterns decides, then the first match of that pattern. Place annotations before an ending `(manual)` or `(later)` marker.

The patterns are `priority` and `effort` in `core/resources/spec_format.toml`. A repo replaces them with `priority` and `effort` in `spec17.json`: ordered regexes whose capture group 1 is the value, for example `"priority": ["\\*\\*(P[0-9]+)\\*\\*"]`. A pattern without a capture group, or one that does not compile, is an error.

## Requirement styles

spec17 reads several ways of writing a requirement, and one repo can mix them. The styles below are small fixtures in `core/test-data` that the tests load: bullets, tables, Spec Kit, headings and paragraphs are files of the `styles` repo; OpenSpec, Kiro, Superpowers and the fallback each have a repo of their own. Spec folders are detected, so there is nothing to configure.

### Bullets

An ID, usually bold, and a level on the first line. A bullet may be `-`, `*` or `+`, or a numbered item. The ID can be bold, in backticks or plain, with an optional `:`. The requirement runs on to the next blank line.

```md
- **STY-01** MUST — classic bold bullet.
* STY-02: The system SHALL accept a star bullet.
1. `STY-03` SHOULD: numbered, backtick ID.
- **STY-04:** MAY wrap
  onto a second line.
```

### Tables

A table row whose first cell is an ID, with the level in the row. Header rows hold no ID and are ignored.

```md
| ID | Level | Text |
|---|---|---|
| STY-05 | MUST | in a table row |
```

### GitHub Spec Kit

Functional requirements are plain bullets. A `[NEEDS CLARIFICATION: …]` stays in the requirement text.

```md
- **FR-001**: System MUST allow users to sign in.
- **FR-002**: System MUST [NEEDS CLARIFICATION: auth method not specified].
- **NFR-001**: Pages SHOULD load within 2 seconds.
```

### Headings

A heading that starts with an ID opens a requirement that runs to the next heading. The level can be on any line of the block, so EARS sentences work. Lines such as `**Covers:**` and a cross-reference table of IDs without levels are not read as requirements.

```md
### HDG-1: Input directory from a property
**Covers:** B-1

When the application starts with `importer.input-dir` set, the importer shall use that path.

### RUL-1
**Covers:** project-wide
**MUST NOT** add a batch framework.
```

### Paragraphs

A paragraph that opens with a bold ID is a block requirement too, with the level anywhere in it. Dotted IDs sort by number: `PAR-1.2` comes before `PAR-1.10`.

```md
**PAR-1.1** Happy path
WHEN the input directory has CSV files
THEN the batch job executes
SHALL read all `*.csv` files
```

### OpenSpec

Requirements are `### Requirement: ID name` headings, usually in `openspec/specs/*/spec.md`, with scenarios under them.

```md
### Requirement: UIX-01 Serve the page
The system SHALL serve an HTML page at `GET /`.

#### Scenario: Page loads
- **WHEN** a user requests `GET /`
- **THEN** the response status is `200 OK`
```

A change in progress, `openspec/changes/<name>/specs/…`, sorts its requirements under `## ADDED`, `## MODIFIED`, `## REMOVED` and `## RENAMED` headings. The change's version of an ID replaces the base one, and a requirement under `## REMOVED Requirements` is retired. `openspec/changes/archive/` and files such as `tasks.md` are not specs. A `### Requirement: Title` with no ID is reported as a spec issue and not counted: give it an ID.

### Kiro and other unnumbered specs

Kiro's `requirements.md` (`### Requirement 1`, user stories, numbered acceptance criteria) carries no IDs, so spec17 finds no requirements in it and reports the heading as a spec issue. Add IDs in any style above to have it counted.

### Superpowers

Design docs in `docs/superpowers/specs/` are scanned like any specs folder; plans in `docs/superpowers/plans/` are not.

### No specs folder

When no specs folder holds a requirement line, any markdown file that has one is used. In the `fallback` fixture that is a design note:

```md
### FBK-1: Startup
The service shall start in under a second.
```

A repo with no requirement lines at all still opens, with zero requirements.

### Reference documents

A link such as `[ADR-0003](decisions/0003-x.md)` to a decision record makes that record a reference document. It is listed with state `ref` and is never due.

### Lines that are not requirements

| Line | Why it is skipped |
| --- | --- |
| `- **UTF-8** encoding is used everywhere.` | `UTF-8` is a well-known token, not an ID |
| `- ISO-8601 MUST — a date format` | same; the level does not make it a requirement |
| `- **B-1** The system reads the input directory.` | `B-1` has one letter, so it is not an ID |
| `\| A1 \| single file? \| All files \|` | no ID |
| `Prose mentions STY-01 and HDG-1 but defines nothing.` | not a bullet, row, heading or paragraph that opens with an ID |
| `- **STY-01** is referenced here without a level.` | no level, so it does not define `STY-01` again; with no definition elsewhere it is a spec issue |

UTF-8 files are read with or without a BOM, and non-ASCII text is kept as written.

## Marks

An ending `(manual)`, `(later)` or `(deferred)` marks a bullet; `(deferred)` means the same as `(later)`. A mark before the end of the line does not count, and the requirement stays due. A struck-out bullet, `- ~~**ABC-12**`, is retired. A check mark (`✅`, `✔`, `☑` or `✳`) says the author considers it done; spec17 then reports whether the tree holds evidence for that (see the verdicts below).

## Tests and citations

- A test tag is the ID in square brackets: `[ABC-12]`, or `[ABC-12, ABC-14]` for several. Put it in the test's title, name or doc attribute, for example a Swift `@Test("[REQ_001] underscore tag")`, a Rust `#[doc = "[API-02]"]` above `#[test]`, or a Python docstring. Tags are read in files in test folders or named like tests (`test_*.py`, `*_test.go`, `*.test.ts`).
- A source citation is an ID after a configured comment marker on the same line, such as `// (ABC-12)`; the parentheses are not required. Svelte also recognizes `<!-- (ABC-12) -->`. The scanner does not track multiline comment boundaries.

The report also lists tags it distrusts, under invalid tests: an ID no spec defines (`unknown`) or a struck-out one (`retired`), an ID tagged only in comments outside detected tests (`comment-only`), only on a disabled test (`disabled`), or only on a test with no direct assertion (`no-assert`). A citation of an ID no spec defines is a dangling citation when a spec uses the same prefix, which catches typos and retired IDs.

Comments attached to a detected test are listed separately as `comment-tagged`. They are evidence to review and do not count as invalid tests.

## States

| State | Meaning |
| --- | --- |
| `tested` | A test is tagged with the ID |
| `src-only` | Code cites the ID, no test is tagged |
| `none` | No test and no code |
| `manual` | Marked `(manual)`: checked by hand, no test expected |
| `later` | Marked `(later)` or `(deferred)`: not due yet |
| `ref` | A reference document: never due |

A requirement with a check mark also gets a verdict: `NO-TEST` (no tagged test), `NO-LINK` (its tests reach no app code in the CodeGraph index), `ok` (they do), or `unchecked` (there is no index). Every state and verdict is evidence found in the tree, not a statement that the requirement is met.

## Configuration

Most repos need none. To override a detected field, put `spec17.json` at the repo root. `spec17.example.json` shows the common fields (`specs`, `tests`, `src`, `ext`, `profiles`, `retired_prefixes`, `codegraph`); `infra_paths` exists too. A field you set replaces its detected value:

```json spec17.example.json
{
  "specs": "docs/specs",
  "tests": ["Tests"],
  "src": ["Sources"],
  "ext": ["swift"],
  "profiles": [{
    "name": "swift",
    "case": "^\\s*@Test\\b|^\\s*func\\s+test",
    "test": "(^|/)[^/]*Tests?(/|\\.swift$)"
  }],
  "retired_prefixes": [],
  "codegraph": ".codegraph/codegraph.db"
}
```

Each file is read with the profile of its language: comment markers, `decl`, `assert`, `off`, `test` (which paths are tests) and `case` (the line that opens a test case). `profiles` holds entries shaped like a `[[profile]]` of `core/resources/profiles.toml`: `{"name": "rust", "assert": "assert!|panic!"}` changes only that field of the built-in Rust profile, and a new `name` with `ext` and `comment` adds a language. A field a profile does not have is an error, and so are the old top-level `comment`, `decl`, `assert`, `off`, `case` and `test_files`.

`annotation` is a regex for a line that attaches to the declaration below it, as a comment above does (`^\s*@` for Swift and Kotlin, `^\s*#\[` for Rust, `^\s*\[` for C#); a tag in a comment above such a line still belongs to the test. A test marker that sits inside a string literal or a trailing comment opens no case. A profile that sets no `annotation` has none.

A language without braces says how a test body ends in its profile: `block_indent` (Python: the first line no deeper than the declaration); `block_open` and `block_close` (nesting, where each match on a line adds or removes a level and the body ends back at 0, as for Ruby `do`/`end` or Lua `function`/`end`); `block_end` (the first line matching it, as for Visual Basic `End Sub`); `block_fence` (a SQL `$$` or `$tag$` region in which nothing counts). Without any of them a body is closed by its braces. `block_indent` cannot be combined with the others, and `block_open` and `block_close` come as a pair; anything else is an error when the repo is opened. A body is looked for only down to the next test's header, so a miscounted body cannot slow the scan down. These are patterns, not a parser: an unusual keyword position can miscount.

Each regex field accepts a string or an ordered array of strings, such as `"assert": ["assert!", "panic!"]`. For each input, the first matching pattern wins. Every `decl` pattern needs a capture group 1 for the name. Setting an array replaces that field's built-in patterns.

The requirement-line patterns (`core/resources/spec_format.toml`) and the language profiles (`core/resources/profiles.toml`) are compiled into the binary. To support another language, add a `[[profile]]` and rebuild, or put the same entry in `profiles`.

Paths in `.aiignore` or `.spec17ignore` (gitignore syntax) are left out of every scan. Symlinks are followed only inside the repo root or a path in the `ALLOWED_PATHS` environment variable, a `:`-separated list.

---
title: Specs and tests
weight: 20
---

Use the same ID in a Markdown requirement, its test tags, and its source comments. spec17 uses the ID to link them in the report and dashboard.

## Requirement IDs

Use an ID such as `ABC-12`, `REQ_001`, or `AC-1.2`. Start with a prefix of 2 to 10 uppercase letters or digits, beginning with a letter. Follow it with a hyphen or underscore and 1 to 5 digits, with optional dotted number parts. spec17 treats a nonbreaking hyphen (U+2011) as `-`.

spec17 excludes common tokens that resemble IDs, such as `UTF-8`, `ISO-8601`, and `RFC-2119`. It accepts IDs such as `HTTP-01` and `RFC-1`.

Use `MUST`, `SHOULD`, `MAY`, or `SHALL` as the requirement level. Headings and paragraphs also accept lowercase `shall`.

## Priority and effort

- **Priority** accepts a `P` followed by digits, as in `[P1]`, `(P2)`, or `(Priority: P1)`.
- **Effort** accepts an uppercase T-shirt size such as `[M]` or `(XL)`, or a labeled estimate such as `[effort: 3 points]`, `(size: M)`, or `[estimate: 2 days]`.

Put whitespace before each hint and place hints before any ending mark such as `(manual)`. Hints inside inline code are ignored. spec17 keeps hints in the requirement text and also reports them as separate fields. With the default patterns, labeled hints take precedence over unlabeled hints.

To use a different format, set `priority` or `effort` in `spec17.json` to an ordered list of regular expressions. Each expression must capture the value in group 1. spec17 uses the first matching pattern and its first match in the text.

## Requirement styles

You can mix these formats in the same project. spec17 detects spec folders automatically.

### Bullets

Start a bullet or numbered item with the ID and put the level on the same line. The ID can be bold, in backticks, or plain, with an optional colon. A blank line ends the requirement.

```md
- **STY-01** MUST — classic bold bullet.
* STY-02: The system SHALL accept a star bullet.
1. `STY-03` SHOULD: numbered, backtick ID.
- **STY-04:** MAY wrap
  onto a second line.
```

### Tables

Put the ID in the first cell and the level anywhere in the row.

```md
| ID | Level | Text |
|---|---|---|
| STY-05 | MUST | in a table row |
```

### GitHub Spec Kit

Use the standard bullet format for functional and nonfunctional requirements. spec17 retains `[NEEDS CLARIFICATION: …]` in the requirement text.

```md
- **FR-001**: System MUST allow users to sign in.
- **FR-002**: System MUST [NEEDS CLARIFICATION: auth method not specified].
- **NFR-001**: Pages SHOULD load within 2 seconds.
```

### Headings

Start a heading with the ID. The requirement can use a level anywhere in the block and ends at the next heading. Lines such as `**Covers:**` can reference other requirements without defining them.

```md
### HDG-1: Input directory from a property
**Covers:** B-1

When the application starts with `importer.input-dir` set, the importer shall use that path.

### RUL-1
**Covers:** project-wide
**MUST NOT** add a batch framework.
```

### Paragraphs

Start the paragraph with a bold ID and put the level anywhere in the block. Dotted IDs sort numerically, so `PAR-1.2` comes before `PAR-1.10`.

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

In `openspec/changes/<name>/specs/`, spec17 uses the change's version of an ID in place of the base version. It retires IDs under `## REMOVED Requirements` and skips archived changes and task lists. A requirement heading without an ID appears as a spec issue. Add an ID to include it in the scan.

### Kiro and other unnumbered specs

Headings such as Kiro's `### Requirement 1` appear as spec issues because they have no requirement ID. Add an ID using one of the formats above.

### Superpowers

Put requirements in `docs/superpowers/specs/`. spec17 skips `docs/superpowers/plans/`.

### No specs folder

If spec17 finds no requirement lines in its usual spec locations, it looks for them in other Markdown files. A requirement in a design note can use the same heading format.

```md
### FBK-1: Startup
The service shall start in under a second.
```

If the report shows zero requirements, use `spec17 doctor` to see where spec17 looked.

### Reference documents

A link such as `[ADR-0003](decisions/0003-x.md)` identifies a reference document. It appears as `ref` and is excluded from due requirements.

### Lines that are not requirements

| Line | Why it is skipped |
| --- | --- |
| `- ISO-8601 MUST — a date format` | spec17 excludes common tokens such as `ISO-8601`. |
| `- **B-1** MUST read the input directory.` | The prefix needs at least two characters. |
| `See STY-01 for details.` | The line refers to an ID without opening a requirement. |
| `- **STY-01** is referenced here without a level.` | The bullet lacks a level. If the ID has no definition elsewhere, spec17 reports a spec issue. |

## Marks

- End a bullet with `(manual)` to mark it for manual verification, or `(later)` or `(deferred)` to defer it.
- Strike through a bullet, as in `- ~~**ABC-12** MUST …~~`, to retire the ID.
- Add a check mark (`✅`, `✔`, `☑`, or `✳`) to mark a requirement done. spec17 reports what evidence it found for that mark. See [States](#states).

## Tests and citations

- **Test tag.** Put `[ABC-12]` or `[ABC-12, ABC-14]` in the test title, name, or documentation attribute. For example, use Swift's `@Test("[ABC-12] opens the file")`, Rust's `#[doc = "[ABC-12]"]` above `#[test]`, or a Python docstring. spec17 looks in test folders and files named like tests, such as `test_*.py`, `*_test.go`, and `*.test.ts`.
- **Source citation.** Put the ID on the same line as the language's comment marker, as in `// (ABC-12)` or Svelte's `<!-- (ABC-12) -->`. Parentheses are optional.

### Problems in the report

| Result | What spec17 found |
| --- | --- |
| `unknown` | A test tag refers to an ID with no definition. |
| `retired` | A test tag refers to a retired ID. |
| `comment-only` | All tags for the ID are in comments outside detected tests. |
| `disabled` | All detected references for the ID are on disabled tests. |
| `no-assert` | spec17 found no direct assertion in the tagged tests. Check whether a helper makes the assertion. |
| `comment-tagged` | A comment with the ID is attached to a detected test. The report lists these separately for review. |

A dangling citation is a test tag or source citation for an undefined ID with an active or configured retired prefix. Check it for a typo or a reference to a retired requirement.

## States

| State | Meaning |
| --- | --- |
| `tested` | A test carries the ID. |
| `src-only` | Source code cites the ID and no test carries it. |
| `none` | No test tag or source citation was found. |
| `manual` | The requirement is marked for manual verification. |
| `later` | The requirement is deferred. |
| `ref` | The item is a reference document and is never due. |

For a requirement marked done, the report adds a verdict based on tagged tests and CodeGraph links.

| Verdict | Meaning |
| --- | --- |
| `NO-TEST` | No tagged test was found. |
| `NO-LINK` | The index shows no link from its tests to application code. |
| `ok` | The index shows a link from its tests to application code. |
| `unchecked` | No usable CodeGraph index is available. |

These results describe the evidence found in the project. spec17 does not run tests or verify that a requirement is met.

## Configuration

spec17 detects the project layout by default. Put `spec17.json` at the project root to override the paths or file extensions it scans. Set only the fields you need to change.

```json
{
  "specs": "docs/specs",
  "tests": ["Tests"],
  "src": ["Sources"],
  "ext": ["swift"]
}
```

| Other field | Use it to |
| --- | --- |
| `retired_prefixes` | Keep reporting dangling references after retiring an entire ID prefix. |
| `codegraph` | Set the index path. The default is `.codegraph/codegraph.db`. |
| `infra_paths` | Set path prefixes under the first source folder that `uncovered` treats as infrastructure. |
| `priority`, `effort` | Override the patterns for [requirement hints](#priority-and-effort). |
| `profiles` | Override language detection rules or add a language. |

To exclude files from the scan, list them in `.aiignore` or `.spec17ignore` using Git ignore syntax. spec17 follows symlinks only within the project root or directories listed in `ALLOWED_PATHS`. Use the platform's path list separator, such as `:` on macOS and Linux.

### Language profiles

To change a built-in profile, add an entry to `profiles` with its name and the fields you want to replace. For example, `{"name": "rust", "assert": ["assert!", "panic!"]}` replaces Rust's assertion patterns. To add a language, use a new name and supply the file extensions and comment markers in `ext` and `comment`.

| Field | What it matches |
| --- | --- |
| `test` | Project-relative paths of test files. |
| `case` | The line that opens a test case. |
| `decl` | A declaration. Capture group 1 must contain its name. |
| `assert`, `off` | Assertions and markers for disabled tests, respectively. |
| `annotation` | Lines that attach to the declaration below, such as `@Test` or `#[test]`. A comment tag above the annotation can still belong to the test. |

Each regex field accepts one string or an ordered list of strings. spec17 uses the first pattern that matches. Unknown profile fields cause an error. If you have an older configuration, move top-level pattern fields into `profiles` and rename `test_files` to `test`.

By default, spec17 uses braces to find the end of a test body. For other languages, choose the rule that matches the syntax.

| Field | Where the body ends |
| --- | --- |
| `block_indent` | The first line with the same or less indentation than the declaration, as in Python. |
| `block_open` and `block_close` | Where nesting returns to zero, as with Ruby's `do` and `end`. |
| `block_end` | The first matching line, such as Visual Basic's `End Sub`. |
| `block_fence` | The closing occurrence of the opening delimiter, such as SQL's `$tag$`. |

Set `block_open` and `block_close` together. Use `block_indent` on its own. See examples in the [built-in language profiles](https://github.com/kpavlov/spec17/blob/main/core/resources/profiles.toml) and [sample configuration](https://github.com/kpavlov/spec17/blob/main/spec17.example.json).

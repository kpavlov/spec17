# Requirement formats and configuration

Read this when a requirement or test tag is not detected, when the repo uses an unusual layout or language, or before you edit `spec17.json`.

## Requirement formats

`specs/`, `SPEC.md`, or any markdown with requirement lines is the source of truth. The canonical form is `- **ABC-12** MUST|SHOULD|MAY — text`.

Also accepted:
- **Bullets:** `- ABC-12: text … MUST`, `*` or numbered lists, and `SHALL`.
- **Tables:** `| ABC-12 | MUST | text |` rows.
- **Headings and paragraphs:** `### ABC-12: title` headings or `**ABC-12** title` paragraphs, with the level (or EARS `shall`) anywhere in the block.
- **OpenSpec:** `### Requirement: ABC-12 name`.
- **IDs:** forms like `FR-001`, `AC-1.2` or `REQ_001`. `UTF-8` and `ISO-8601` are never IDs.

A test links to a requirement by tagging its title with `[ABC-12]`. Source comments cite it as `(ABC-12)`.

## Detected layout

None of this needs configuration in the usual layouts.

**Specs folders.** Recognized layouts include:
- `specs/`, `spec/`, `docs/specs/`, `.kiro/specs/`, `docs/superpowers/specs/`, and dedicated folders such as `requirements/` or `.specify/specs/`. Generic `docs/` or `doc/` is a fallback when dedicated folders contain no requirement lines.
- `openspec/specs/` and each `openspec/changes/*/specs/` (but not `archive/`). In OpenSpec:
  - requirements under a `## REMOVED` heading are retired;
  - a change's version of a requirement replaces the base one;
  - a `### Requirement` without an ID is a spec issue.

**Source and test folders and extensions** are those of the dominant language. Detection uses the index file list when readable and falls back to a repository walk.

**Profiles.** Each file is read with the built-in profile of its own language: `swift`, `rust`, `typescript` (also JavaScript), `svelte`, `python`, `go`, `jvm` (Java, Kotlin, Scala), `csharp`, `ruby`, `php`, `c/c++`, `dart`, `lua`, `r`, `objective-c`, `solidity`, `erlang`, `vb.net`, `pascal`, `sql`. A profile holds the comment markers and the declaration, assertion, switched-off, test-path and test-case regexes. For scanned extensions without a profile, the scanner falls back to the dominant profile when available.

**Symlinks** count only inside the repo root or `ALLOWED_PATHS`.

## spec17.json

If parsing fails, inspect the reported line and column for JSON syntax errors such as a trailing comma. For a type error such as `"tests":"tests"`, keep the path and use the required array, `"tests":["tests"]`. A legacy top-level `test_files` regex belongs under the matching language entry as `profiles[].test`.

At the repo root, set only the fields that need an override. For example, if Rust tests live exclusively under `tests/`, use `{"tests":["tests"]}`, then rerun `spec17 doctor` and trace a known ID.

| Field | Value and effect |
|---|---|
| `specs` | String naming a spec folder or one Markdown file, relative to the repo root. A nonempty value replaces spec discovery. |
| `src`, `tests`, `ext` | Arrays of source folders, test folders and extensions without dots. A nonempty list replaces detection for that field. Include both languages' paths and extensions in a mixed repo. |
| `profiles` | Language overrides or additions, described below. Unknown profile fields are errors. At the top level, the legacy keys `comment`, `decl`, `assert`, `off`, `case` and `test_files` are rejected; other unrecognized keys are ignored. |
| `priority`, `effort` | Ordered arrays of regexes for requirement text. A nonempty list replaces the defaults. The first nonempty capture group 1, in pattern order, supplies the value. |
| `retired_prefixes` | Array of strings naming extra prefixes to check for dangling citations even after their requirements disappear. |
| `codegraph` | Index path, default `.codegraph/codegraph.db`. |
| `infra_paths` | Array of path prefixes under the first source folder to exclude from uncovered-code candidates. |

Each entry in `profiles` requires a string `name`. A built-in name overrides only the supplied fields. A new name adds a profile ahead of the built-ins, so it can claim a shared extension. For example, `{"name":"rust","assert":"assert!|panic!"}` replaces the Rust assertion pattern.

| Profile field | Type and behavior |
|---|---|
| `ext`, `comment` | Arrays of strings for extensions without dots and literal comment markers. |
| `decl`, `assert`, `off`, `test`, `case`, `annotation` | Regexes for declarations, assertions, disabled tests, test-file paths, test-case starts, and annotations attached to the following declaration. Each declaration pattern must capture the name in group 1. |
| `inline`, `call`, `lifetimes` | Booleans. Include source folders in detected test folders, take the test name from a quoted call, or recognize Rust-style lifetimes, respectively. |
| `block_indent` | Boolean. End the test body by indentation. This mode excludes all four block regex fields below. |
| `block_open`, `block_close`, `block_end`, `block_fence` | Regexes for nested delimiters (supply open and close together), a terminating line, or a quoted fence such as a SQL dollar tag. Without a block rule, the scanner uses braces. |

A profile regex field accepts a string or an array of strings. Supplying either replaces that field’s inherited patterns. Swift’s built-in `case` recognizes `@Test` and `func test…`, including annotation prefixes. Preserve those forms when overriding it. When both `src` and `tests` are explicitly set, built-in test-path matching is disabled unless the profile override explicitly supplies `test`.

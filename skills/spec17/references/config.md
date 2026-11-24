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

**Specs folders.** Every folder that holds markdown is found:
- `specs/`, `spec/`, `docs/specs/`, `.kiro/specs/`, `docs/superpowers/specs/`, and a `docs/` holding requirement lines.
- `openspec/specs/` and each `openspec/changes/*/specs/` (but not `archive/`). In OpenSpec:
  - requirements under a `## REMOVED` heading are retired;
  - a change's version of a requirement replaces the base one;
  - a `### Requirement` without an ID is a spec issue.

**Source and test folders and extensions** are those of the dominant language. They are read from the CodeGraph index when there is one.

**Profiles.** Each file is read with the built-in profile of its own language: `swift`, `rust`, `typescript` (also JavaScript), `svelte`, `python`, `go`, `jvm` (Java, Kotlin, Scala), `csharp`, `ruby`, `php`, `c/c++`, `dart`, `lua`, `r`, `objective-c`, `solidity`, `erlang`, `vb.net`, `pascal`, `sql`. A profile holds the comment markers and the declaration, assertion, switched-off, test-path and test-case regexes. A file of no known language is read like the dominant one.

**Symlinks** count only inside the repo root or `ALLOWED_PATHS`.

## spec17.json

Put it, optionally, at the repo root. Fields:
- `specs`, `tests`, `src`, `ext`
- `profiles`
- `retired_prefixes`
- `codegraph`
- `infra_paths`

Rules:
- A field you set replaces its detected value.
- For a second known language, list its extension in `ext`, and its folders in `src`/`tests`.
- An unknown field is an error. So are the old top-level `comment`, `decl`, `assert`, `off`, `case` and `test_files`.

`profiles` is a list of entries with the fields `name`, `ext`, `comment`, `decl`, `assert`, `off`, `test`, `inline`, `case` and `call`.
- An entry named like a built-in profile changes only the fields it sets, as in `{"name": "rust", "assert": "assert!|panic!"}`.
- Any other name adds a language; the regexes it leaves out match nothing.
- `case` is a regex matching a test marker, declaration or call. It replaces the built-in pattern, so include every form to keep. For example, Swift's is `^\\s*@Test\\b|^\\s*func\\s+(?:test\\w*|`test[^`]*`)`.
- `call: true` takes the test name from the quoted string in a matched call such as `it("name", ...)`.
- `inline: true` includes source folders when finding tests, as for Rust's inline tests.
- `decl`, `assert`, `off`, `test` and `case` each accept a regex string or an ordered array of regex strings. The first pattern that matches wins. Each `decl` pattern needs capture group 1 for the name. A configured array replaces that field's built-in patterns.

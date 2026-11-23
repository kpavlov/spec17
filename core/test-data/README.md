# test-data

Small repos, one per spec layout, that `core/tests/core.rs` loads with `Analysis::load`. Left out of spec17's own scan by `.spec17ignore`.

- `styles/`: every requirement-line style (bullets, tables, kotlinconf-sdd headings, sdd-workshop paragraphs, Spec Kit, underscore IDs, UTF-8) and lines that must not parse.
  Run its Swift tests with `cd core/test-data/styles && swift test` (Swift 6 or later).
- `openspec/`: OpenSpec base specs, an in-flight change (ADDED, MODIFIED, REMOVED, RENAMED), an archived change, and a `spec/` folder.
- `superpowers/`: Superpowers design docs (scanned) and plans (not).
- `kiro/`: Kiro requirements, which carry no IDs.
- `fallback/`: no spec folder, a block requirement in other markdown.
- `nospecs/`: sources and a test, no markdown at all: spec17 must still open it (SCAN-28).

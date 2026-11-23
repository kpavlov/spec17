# Changelog

## 0.3.0 — Unreleased

- Detect priority and effort hints in requirements; filter by them in the dashboard.
- Add a requirements reference graph with circular reference groups.
- Recognize tags in comments attached to tests and improve test detection, including Swift tests.
- Scan requirement citations in Svelte files without custom configuration.
- Expose the embedded agent skill through the MCP Skills extension.
- Filter requirements with a click that selects one value, Shift, Ctrl or ⌘ + click to add or remove values, and All when every value is selected; fields with nothing to choose are hidden, and the Filters button becomes an icon that is disabled when there is nothing to filter by; selecting a segment no longer changes its width.
- Show up to five lines of requirement text in graph and link previews.
- Describe in language profiles which lines attach to a test (`annotation`), how a test body ends without braces (`block_*`) and which quotes are lifetimes (`lifetimes`); `spec17.json` can replace the priority and effort hint patterns. **Breaking:** a profile in `spec17.json` that relied on `@` lines attaching to a test must now set `annotation`, and conflicting `block_*` fields are an error.
- Use CodeGraph's function ranges and names only while the index holds the file's current content (by hash, not modification time), so the same tree gives the same output.

## 0.2.0

- Redesign dashboard navigation, with a Documents view and health-focused Settings.
- Remember reviewed requirements and collapsed groups per project.
- Add agent-friendly documentation in Markdown and `llms.txt` formats.

## 0.1.0

- Initial release: deterministic requirements, test and source citation analysis through a CLI, MCP server and web dashboard.
- Detect project layouts and common specification formats; configure language profiles for mixed projects.
- Trace changes to requirements and inspect related tests, code and module dependencies with optional CodeGraph integration.
- Install the agent skill and connect over HTTP or stdio MCP; ship the dashboard in the release binary.

Requirement links and test tags are evidence for review; they do not prove implementation or passing tests.

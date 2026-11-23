# Changelog

## 0.3.0 — Unreleased

- Detect priority and effort hints in requirements; filter by them in the dashboard.
- Add a requirements reference graph with circular reference groups.
- Recognize tags in comments attached to tests and improve test detection, including Swift tests.
- Scan requirement citations in Svelte files without custom configuration.
- Expose the embedded agent skill through the MCP Skills extension.

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

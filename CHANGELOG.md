# Changelog

## 0.4.0

- Skill: Keep agent reviews read-only and report separately what the code implements, what the tests assert, and which tests ran.
- Skill: Require a concrete example of incompatible behavior before the agent classifies two requirements as conflicting.
- Skill: Explain how to investigate unknown requirement IDs, missing tests, and unavailable CodeGraph results.

## 0.3.0

- Detect priority and effort hints in requirements, show them as pills and filter by them.
- Add a graph of references between requirements, with circular references highlighted.
- Recognize requirement tags in comments attached to tests, reported apart from invalid tests.
- Improve test detection across languages, including Swift.
- Expose the embedded agent skill through the MCP Skills extension
- Rewrite SKILL.md around the question asked.
- Rework the requirement filters: one value per click or several with a modifier key, and no fields with nothing to choose.
- Use the same filter toolbar on the Tests page, and hide its preview until a test is selected.
- Link requirement notes on Changes to the file's diff, and show longer requirement previews.
- Configure test detection in language profiles instead of code. **Breaking:** a `spec17.json` profile that relied on `@` lines attaching to a test must now set `annotation`.
- Read test ranges and names from CodeGraph only while the index holds the file's current content.
- Scan large test suites faster.

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

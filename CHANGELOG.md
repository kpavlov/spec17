# Changelog

## Unreleased

### Changed

- Dashboard and Changes sections resize by dragging the gap below them (up-down cursor; ↑/↓ and Home from the keyboard, double-click to reset). This replaces the small corner grip, and Dashboard sections now remember their height and open state too.
- Dashboard and docs: calmer, higher-contrast light and dark palette with an automated WCAG and APCA check; one type scale with a 13px floor.
- Show each evidence state as a colour-coded icon with its own shape; its name and meaning are on hover, focus and tap, and the state filter doubles as the key. A check mark in a spec with no tagged test, or none linked to code, gets its own orange icon.
- Keep requirement rows to one line, with status in its own column, long locations cut in the middle (full path on hover or focus, with a copy button), and several change reasons folded into one pill.
- Replace the always-open filter rows with a "+ Filter" menu and a chip per selected value, on Requirements, Changes and Tests. The menu starts closed.
- Head every page with the repository name, git branch (each with its own icon, apart from the product name) and scan age (`/api/info` now includes `branch`); cap running text outside requirement lists at 70 characters a line.
- Rebuild the Dashboard around a summary strip and one problem list grouped into errors and collapsed warnings. Specs to compare are now listed; before, they counted in the total but were not shown.
- Add display settings: row density, text size and theme. Make the dark theme a lighter blue-grey, like Zed's One Dark, and the light theme's violet accent more saturated.
- Add a vivid brand violet for the product name in the dashboard and docs headers and for the browser tab icon.
- Docs site: fonts, product name, tab icon, theme button, keyboard hints and menu highlight now match the dashboard. The table of contents sits close under its page, without a second scroll area, and marks the current section on a neutral line. Text uses its type scale one step larger, and the stripe along the top is the brand violet.
- Add a command palette (⌘K or Ctrl+K) over views, actions, saved views and requirement IDs.
- Turn `state:none level:must` words in the requirement search into filters on Enter.
- Save search-and-filter views per repository. This upgrades the browser database: an older spec17 served on the same port can then no longer load its reviewed marks (releases up to 0.5.0 offer no reset; clear the site's data).
- Offer to reset the browser database when it cannot be used (made by a newer spec17, incomplete or unreadable). Nothing is deleted before you confirm.
- Never make a Dashboard or Changes block taller than its content: a dragged height is now a limit.
- Say in Settings what CodeGraph is, with a link to its project.

### Fixed

- Dashboard: a late or failed answer no longer replaces a newer one (open requirement, source preview, blame, changes, commits and modules; saved views changed while storage was still answering).
- "Open in" links of the VS Code family and Zed now work for paths containing `#`, `?` or `%`.
- Test filter chips no longer break for a module named `tagged`, `untagged` or `changed`.
- Selected and hovered navigation items and buttons reach 4.5:1 text contrast in both themes; their tints are fainter.
- Marking or clearing a requirement as reviewed no longer overwrites marks saved from another tab.
- Collapsing or expanding a table group no longer overwrites groups another tab saved.

## 0.5.0

- Report a skill that is in no skills folder as information, not a warning: clients with MCP Skills support, such as Claude Code (`/spec17:spec17`), load it from the spec17 server. It no longer shows a banner or a badge, or opens Settings on a first run.
- Add a `.gitignore` to the installed skill folder so the on-disk copy stays out of git; it is not served as an MCP skill resource.
- Start the requirement graph with requirements of the same ID prefix together in one block; linked requirements stay together, in the block of their most common prefix.
- Colour the requirement graph by ID prefix, put graph labels closer to their own node, and open graphs sooner when most nodes are unlinked.
- Fix the requirement graph spreading off the canvas after dragging a node, and keep requirements added by a rescan with their prefix.
- Have the agent skill prefer updating an existing requirement, while it stays small, over adding one for behaviour that is not really new.
- Add worked examples to the agent skill. Its description now also covers duplicate tests, a branch's effect on requirements and spec17 setup, and excludes standalone test writing, Git merge conflicts and general requirements advice.
- Explain in the skill how to complete results cut off by row or text limits, work with the CLI alone, and recover from a missing binary or MCP server, a rejected argument or an invalid `spec17.json`; document more `spec17.json` and language profile fields.
- Drop the stronger-model requirement from the skill's conflict review, and have it report a pair as unclear when source, callers or tests are unavailable.

## 0.4.0

- Keep agent reviews read-only and report separately what the code implements, what the tests assert, and which tests ran.
- Require a concrete example of incompatible behavior before the agent classifies two requirements as conflicting.
- Explain how to investigate unknown requirement IDs, missing tests, and unavailable CodeGraph results.
- Include separate skill references for tools, configuration, and conflict review in local and global installs and through MCP.
- Apply the repository license to the bundled skill in place of the separate MIT license.
- Revise the quickstart, CLI and dashboard guide, requirement and test-tag examples, and MCP setup instructions.
- Refresh dashboard screenshots in light and dark themes and let readers open them at a larger size.

## 0.3.0

- Detect priority and effort hints in requirements, show them as pills and filter by them.
- Add a graph of references between requirements, with circular references highlighted.
- Recognize requirement tags in comments attached to tests, reported apart from invalid tests.
- Improve test detection across languages, including Swift.
- Expose the embedded agent skill through the MCP Skills extension.
- Organize `SKILL.md` around the question the agent needs to answer.
- Rework the requirement filters: one value per click or several with a modifier key, and no fields with nothing to choose.
- Use the same filter toolbar on the Tests page, and hide its preview until a test is selected.
- Link requirement notes on Changes to the file's diff, and show longer requirement previews.
- Configure test detection in language profiles instead of code. **Breaking:** a `spec17.json` profile that relied on `@` lines attaching to a test must now set `annotation`.
- Read test ranges and names from CodeGraph only while the index holds the file's current content.
- Scan large test suites faster.
- Replace the developer evaluation agreement with the Spec17 Source-Available License.

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

---
title: spec17
description: Find tagged tests and code references for project requirements.
---

## The problem

Requirements, tests and code change separately. A requirement can lack a test, a comment can cite an ID that no longer exists, and it is hard to tell which requirements a Git change touches.

## How it works

![The CLI, dashboard, and MCP server share a scan of the project files and optional CodeGraph index](img/overview.svg)

spec17 links requirements in Markdown to test tags and source comments, and shows what it finds without running the tests. Use it from the CLI, the dashboard, or an assistant connected through MCP. An optional [CodeGraph index](usage/#codegraph) adds links from tests to the code they call.

## Get started

Install with [Homebrew](https://brew.sh/) on an Apple Silicon Mac or on Linux (arm64 or x86_64).

```sh
brew tap kpavlov/spec17 https://github.com/kpavlov/spec17
brew install kpavlov/spec17/spec17
```

Scan your first project, try the sample, or build from source in the [quickstart](quickstart/).

![spec17 dashboard showing tagged test coverage and problems found during the scan](img/dashboard.png)

- [CLI and dashboard reference](usage/)
- [Requirement formats and test tags](specs/)
- [MCP setup for coding assistants](agents/)

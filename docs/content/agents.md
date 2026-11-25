---
title: Agents
weight: 30
---

Connect an AI coding assistant to spec17 using the Model Context Protocol (MCP). The bundled skill shows the assistant how to use spec17 and how to distinguish links in the project from test results.

## Install the skill and an MCP entry

If your client reads `.mcp.json`, run `spec17 install` from the project root. It installs the skill in `.agents/skills/spec17/`, with a `.gitignore` that keeps this copy out of Git, and adds an HTTP entry to `.mcp.json`. It preserves other server entries and leaves files with invalid JSON untouched.

| Command | Effect |
| --- | --- |
| `spec17 install --global` | Install the skill and HTTP entry in your home directory. |
| `spec17 install --codegraph` | Add only the CodeGraph server entry to the project. |
| `spec17 uninstall` | Remove the project skill and HTTP entry. |
| `spec17 uninstall --global` | Remove the global skill. To remove the global HTTP entry, run `spec17 uninstall --global --only mcp`. |

Use **Settings** in the dashboard to manage the skill and HTTP or stdio entries separately for the project or your home directory.

## stdio or HTTP

- **stdio.** The client starts a spec17 process for the project. The example below disables both the browser dashboard and the HTTP endpoint.
- **HTTP.** Run `spec17` and leave it running while the client is connected. The default MCP endpoint is `http://127.0.0.1:7411/mcp`.

> [!WARNING]
> **Use stdio on shared computers.** The HTTP server accepts local connections without a login. Other local users and processes can read project data and modify files through MCP using the permissions of the account that started spec17.

Each HTTP server serves one project. Check the dashboard header to see which project it opened. To serve multiple projects, start a server on a different port for each project. Use the same port for `install` and `serve`.

```sh
spec17 --root ~/code/app-a install --port 7411
spec17 --root ~/code/app-a serve --port 7411
```

## Connect a client

For HTTP, configure your client to use the port of the running spec17 server.

```json
{"mcpServers": {"spec17": {"type": "http", "url": "http://127.0.0.1:7411/mcp"}}}
```

For stdio, configure your client to launch spec17 without a listening port.

```json
{"mcpServers": {"spec17-stdio": {"type": "stdio", "command": "spec17", "args": ["--stdio-mcp", "--no-http-mcp", "--no-ui"]}}}
```

If the client starts spec17 outside the project, add `"--root", "/path/to/repo"` to `args`. If it can't find `spec17` on its `PATH`, set `command` to the absolute path of the executable.

## Open the dashboard in an MCP client

If your client supports MCP Apps, ask it to call the `dashboard` tool. Pass a `view` argument to open a specific page, such as `{"view": "requirements/API-02"}`. The release binary includes the app for both HTTP and stdio.

## Load the skill over MCP

Clients that support the [MCP Skills extension](https://modelcontextprotocol.io/extensions/skills/overview) can load the skill over either transport. The server exposes `skill://spec17/SKILL.md` and its references under `skill://spec17/references/`. These clients, such as Claude Code (`/spec17:spec17`), don't need a copy on disk, so `spec17 doctor` reports a missing one as information. For other clients, `spec17 install` writes the copy.

## Tools

| Task | Tools |
| --- | --- |
| Inspect requirements and tests | `report`, `status`, `trace`, `search_requirements`, `search_tests` |
| Inspect Git changes | `changes`, `diff`, `history`, `commits`, `blame` |
| Inspect code | `search_modules`, `file`, `source`, `overlap`, `uncovered` |
| Check setup or open the dashboard | `doctor`, `install_status`, `dashboard` |
| Change setup | `codegraph`, `install`, `uninstall`, `module` |

Only the tools under **Change setup** can write files. For terminal commands, see the [CLI reference](../usage/).

## Resources

- `spec17://report`, `spec17://status`, `spec17://changes`, `spec17://uncovered`, and `spec17://overlap` return text. Append `.json` for structured output. Schemas are available at `spec17://schemas/<name>`.
- `spec17://modules` returns modules, CodeGraph symbols, and dependencies as JSON.
- `spec17://tests` returns detected test cases and their tags as JSON.

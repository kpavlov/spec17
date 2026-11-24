---
title: Agents
weight: 30
---

An AI coding assistant can query spec17 through the Model Context Protocol (MCP). The bundled skill tells the assistant which tools to use and how to fall back to the command line.

The MCP server also exposes the bundled skill through the [MCP Skills extension](https://modelcontextprotocol.io/extensions/skills/overview), at `skill://spec17/SKILL.md`. Clients that support the extension can discover and load it from either transport. Client support varies; the install commands below also make the skill available on disk.

## Install the skill and an MCP entry

```sh
spec17 install              # repo skill + HTTP entry in <repo>/.mcp.json
spec17 install --global     # ~/.agents/skills/spec17/SKILL.md + HTTP entry in ~/.mcp.json
spec17 install --codegraph  # only adds the codegraph MCP server
spec17 uninstall            # remove the repo skill and HTTP entry
spec17 uninstall --global   # only the global skill; add --only mcp for the entry in ~/.mcp.json
```

`.mcp.json` is merged, never rewritten: other servers stay and invalid JSON is left alone. The Settings tab of the dashboard installs or removes each entry separately, for this repo or globally, and can also add the stdio entry below.

## stdio or HTTP

- **stdio** — The client starts its own spec17 process for one project. Use `--no-http-mcp --no-ui` to disable the HTTP server and dashboard.
- **HTTP** — Run `spec17` and leave it running. By default, the dashboard is at `http://127.0.0.1:7411/` and the MCP endpoint is at `http://127.0.0.1:7411/mcp`.

> [!WARNING]
> **Use stdio on shared computers.** The HTTP server accepts connections from the same computer without a login. Other local users and processes can read project data and call tools that modify files. Those tools run with the permissions of the user who started spec17.

An HTTP server serves the project it opened at startup. Check the project path in the dashboard header. To serve multiple projects, give each its own port and run each server in a separate terminal. Use the same port for `install` and `serve`:

```sh
spec17 --root ~/code/app-a install --port 7411
spec17 --root ~/code/app-a serve --port 7411
```

Repeat for the second project with its own folder and a different port, such as `7412`.

## Connect a client

HTTP, to a running `spec17` (default port 7411; `spec17 install --port N` to match):

```json
{"mcpServers": {"spec17": {"type": "http", "url": "http://127.0.0.1:7411/mcp"}}}
```

stdio, so each agent session launches its own process and binds no port:

```json
{"mcpServers": {"spec17-stdio": {"type": "stdio", "command": "spec17", "args": ["--stdio-mcp", "--no-http-mcp", "--no-ui"]}}}
```

Add `"--root", "/path/to/repo"` to `args` if the client starts spec17 outside the repo. The Settings tab and the `install` tool write this entry with the absolute path of the binary as `command`; a repo-scoped entry also sets `--root`, a global one does not.

## Tools

`report`, `status`, `changes`, `diff`, `history`, `commits`, `blame`, `trace`, `search_tests`, `search_requirements`, `search_modules`, `overlap`, `uncovered`, `file`, `source`, `doctor`, `codegraph`, `install_status`, `install`, `uninstall`, `module`, `dashboard`. The tools that write files are `codegraph`, `install`, `uninstall` and `module`; the rest only read. `commits`, `blame`, `file`, `source`, `module`, `codegraph`, `dashboard`, `install_status` and the `search_*` tools have no CLI command of that name.

Plain-text resources return the same output as the matching tools: `spec17://report`, `spec17://status`, `spec17://changes`, `spec17://uncovered`, `spec17://overlap`. JSON versions, typed by schemas at `spec17://schemas/<name>`, add `.json`. Two more resources are JSON only: `spec17://modules` (modules, with CodeGraph symbols and dependencies) and `spec17://tests` (test cases detected, with the IDs tagged on each; detected, not run).

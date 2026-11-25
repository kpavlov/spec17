---
title: Usage
weight: 10
---

Run spec17 from your project or pass `--root DIR`. By default, spec17 searches the current directory and its parents for specs, then falls back to the nearest Git work tree. Use `--json` for machine-readable results and `spec17 COMMAND --help` for command options.

| Command | Use it to |
| --- | --- |
| `spec17 report` | Summarize tagged test coverage and problems across the project. |
| `spec17 status` | Summarize each spec or filter requirements by state with flags such as `--missing` or `--issues`. |
| `spec17 status --changed [REF]` | List requirements touched by changes relative to a Git ref. The default is `HEAD`. |
| `spec17 trace ID` | Look up a requirement's text, tagged tests, code citations, and related code. |
| `spec17 changes` | See changed files and the requirements they touch. |
| `spec17 diff PATH` | See one file's diff with requirement IDs for each hunk. |
| `spec17 history` | List commits on your branch that are absent from the base branch. |
| `spec17 overlap` | Find declarations cited by multiple requirements. Requires [CodeGraph](#codegraph). |
| `spec17 uncovered` | Find declarations with no detected requirement evidence. Requires [CodeGraph](#codegraph). |
| `spec17 doctor` | Check project setup. Add `--fix` to create or refresh the CodeGraph index when needed. |
| `spec17 install`, `spec17 uninstall` | Manage the [agent skill and MCP entries](../agents/). |

```sh
spec17 status --missing               # due requirements with no tagged test
spec17 trace SCAN-01                  # evidence for one requirement
spec17 changes --ref main             # changes relative to main
spec17 report --json --root ../other  # JSON for another project
```

## Dashboard

Run `spec17` or `spec17 serve` to open the dashboard at `http://127.0.0.1:7411/`. The server accepts local connections only. For shared computers, see [stdio setup](../agents/#stdio-or-http).

The MCP server also serves the dashboard as an MCP App. See how to [open the dashboard in an MCP client](../agents/#open-the-dashboard-in-an-mcp-client).

| Option | Effect |
| --- | --- |
| `spec17 serve --port N` | Change the port for the dashboard and MCP server. |
| `spec17 serve --no-open` | Start the server without opening a browser. |
| `spec17 serve --ui-port N` | Put the dashboard on a separate port. The main port serves only `/mcp`. |

Open **Requirements** and select an ID to see its tests and code citations. Use the filters to narrow the list. Hold Shift, Ctrl, or Command while clicking to select multiple values in a filter. Copy the URL to link directly to a view or requirement, such as `/requirements/API-02`.

![Requirement API-02 with its tagged test, code citation, and edited spec text](../img/requirements.png)

Open **Tests** to browse detected test cases and their tags. In **Changes**, select a requirement note to open the related file diff.

![Detected tests and filters](../img/tests.png)

To follow references between requirements, switch **Requirements** to **Graph**. Each arrow points to the referenced requirement. Requirements are grouped and colored by ID prefix. Linked requirements stay together, in the group of their most common prefix. Highlighted rings and arrows mark circular references. Select a node to open its details.

![Requirements graph showing references and a highlighted cycle](../img/requirements-graph.png)

## CodeGraph

spec17 can read a [CodeGraph](https://github.com/colbymchenry/codegraph) index to link tests to the code they call and find declarations to review.

- **Overlap** lists declarations cited by multiple requirements. Review the requirements to determine whether they agree.
- **Uncovered** lists declarations with no detected requirement evidence. Review the code to determine whether it needs a requirement.

If an index is missing, the report and dashboard label these checks as unavailable. The `overlap` and `uncovered` commands and tools, including the corresponding `status` flags, return `no .codegraph index here`. If CodeGraph is installed, run `spec17 doctor --fix` or use **Settings** to create or refresh the index.

![Potential overlap between requirements](../img/overlap.png)

![Declarations with no detected requirement evidence](../img/uncovered.png)

## Build from source

See the [build prerequisites and installation steps](../quickstart/#build-from-source).

```sh
make build     # target/release/spec17, UI embedded
make install   # into ~/.cargo/bin
make dev       # Vite dev server (hot reload) + Rust server
make report    # text report for ROOT (default: this repo)
```

`SKIP_WEB_BUILD=1` reuses an existing build in `web/dist`. For packaging and publishing, see the [release instructions](https://github.com/kpavlov/spec17#releases).

---
title: Quickstart
weight: 5
description: Install spec17 and see which requirements in a project have tagged tests or links to code.
---

Scan a local project to see which requirements have tagged tests or code references. If you don't have a project to scan, install spec17 and [try the example](#try-an-example).

## 1. Install spec17

Install with [Homebrew](https://brew.sh/). Prebuilt releases are available for Apple Silicon Macs and for Linux on arm64 and x86_64.

```sh
brew tap kpavlov/spec17 https://github.com/kpavlov/spec17
brew install kpavlov/spec17/spec17
spec17 --version
```

The last command should print a version number. For Intel Macs and other systems without a prebuilt release, see [Build from source](#build-from-source).

## 2. Scan a project

From the project root, run the report.

```sh
spec17 report
```

The report summarizes tagged test coverage and lists items to review. For example, a requirement listed as `src-only` has a code citation but no tagged test. Coverage counts tags in the project without running the tests.

If the report shows zero requirements, run `spec17 doctor` to check where spec17 looked. Make sure the project uses a [supported requirement format](../specs/#requirement-styles).

## 3. Open the dashboard

On your own computer, run this command from the project root.

```sh
spec17
```

The dashboard opens at `http://127.0.0.1:7411/`. Select a requirement in **Requirements** to see its references. Stop the server with Ctrl+C.

On a shared computer, use `spec17 report` or configure an assistant with [stdio](../agents/#stdio-or-http). The browser dashboard accepts local connections without a login.

![Requirements view showing the tagged test and code citation for API-02](../img/requirements.png)

## 4. Connect an AI coding assistant

If your assistant supports MCP and reads `.mcp.json`, run `spec17 install` from the project root. Keep `spec17` running and start or restart the assistant in the same project.

Ask “Which requirements have no tagged test?” For manual configuration or stdio, see [MCP setup](../agents/).

## Try an example

This example creates a temporary project with requirements, source code, and a tagged test. You don't need Python to scan the files.

```sh
cd "$(mktemp -d)" && mkdir -p specs src tests && {
cat > specs/greeter.md <<'EXAMPLE'
# Greeter

- **GRT-01** MUST — greet a person by name.
- **GRT-02** SHOULD — greet the world when no name is given.
EXAMPLE
cat > src/greeter.py <<'EXAMPLE'
def greet(name=None):
    # (GRT-01) greet by name
    return f"Hello, {name or 'world'}!"
EXAMPLE
cat > tests/test_greeter.py <<'EXAMPLE'
from src.greeter import greet

def test_greets_by_name():
    """[GRT-01] greets by name"""
    assert greet("Ada") == "Hello, Ada!"
EXAMPLE
spec17 report
}
```

The report shows one of two requirements with a tagged test. Trace `GRT-01` to see the test and code citation, then list requirements that still need a tagged test.

```sh
spec17 trace GRT-01
spec17 status --missing
```

The second command lists `GRT-02`, which has neither a tagged test nor a code citation.

## Other installation options

Update an existing Homebrew installation with `brew update && brew upgrade spec17`.

For the rolling development build on Apple Silicon, install `kpavlov/spec17/spec17-dev`. Both formulas install a command named `spec17`, so uninstall one before installing the other.

## Build from source

You need Git, Make, Rust, Node.js, and pnpm to build spec17.

```sh
git clone https://github.com/kpavlov/spec17
cd spec17
make install
```

`make install` installs the command in `~/.cargo/bin`. Add that directory to your `PATH` if your shell can't find `spec17`. To build the default branch with Homebrew, run `brew install --HEAD kpavlov/spec17/spec17`.

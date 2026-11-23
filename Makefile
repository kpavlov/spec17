# spec17: Rust core + server + Svelte dashboard. Scans the current directory unless ROOT is given.
SHELL := /bin/bash
ROOT ?= $(CURDIR)
PORT ?= 7411
OPEN ?= 1
BIN  := target/release/spec17
# macOS and Linux
OS   := $(shell uname -s | tr A-Z a-z)
ARCH := $(shell uname -m | sed s/arm64/aarch64/)
OPENER := $(shell command -v open || command -v xdg-open)
VERSION := $(shell sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
# Archive label: the workspace version or a timestamped dev build. TAG selects the tagged release.
LABEL ?= $(VERSION)
TAG ?= v$(VERSION)
# macOS signing identity: "-" is ad hoc (enough for Homebrew); a "Developer ID Application: ..." identity for wider distribution
SIGN_IDENTITY ?= -
archive = dist/spec17-$(1)-$(OS)-$(ARCH).tar.gz

.PHONY: help dev serve web build release-build run install uninstall dist sign check-tag upload release-dev upload-edge notices report test parity bench docs screenshots site upgrade clean mcp-inspector

help:
	@echo "make dev      Vite (HMR, :5173) + Rust server on :$(PORT) proxying to it, scanning $(ROOT)"
	@echo "make serve    Rust server only (debug), scanning $(ROOT); run 'cd web && pnpm run dev' yourself"
	@echo "make build    release binary with the UI embedded: $(BIN)"
	@echo "make run      build, then serve the release binary on :$(PORT) for $(ROOT)"
	@echo "make install   build + install spec17 for your user into ~/.cargo/bin; make uninstall removes it"
	@echo "make dist      dist/spec17-$(VERSION)-$(OS)-$(ARCH).tar.gz (+ .sha256) for this machine (macOS or Linux); signed on macOS"
	@echo "make sign      macOS: codesign the release binary (SIGN_IDENTITY, default ad hoc)"
	@echo "make upload    add this machine's archive to the prerelease TAG (default v$(VERSION), must be pushed and checked out); creates it if missing"
	@echo "make notices   regenerate NOTICES; run after dependency changes"
	@echo "make release-dev  upload an Apple Silicon build from origin/main and write Formula/spec17-dev.rb"
	@echo "make report   print the text report for $(ROOT)"
	@echo "make test     cargo tests; make parity compares with the old Python scripts if vendored in $(ROOT)"
	@echo "make bench    time spec17 on a generated repo at REVS (default: main HEAD; . = working tree), SCALE=1 RUNS=10 JSON=1"
	@echo "make screenshots  retake docs/static/img from a deliberately broken tree (restored after; needs Chrome)"
	@echo "make docs    build the Hugo site in docs/ into docs/public (needs Hugo >= 0.146); make site serves it with live reload on :1313 and opens it (reusing a Chrome tab on the site; OPEN=0: don't open)"
	@echo "make upgrade   bump web (pnpm, to latest) and Rust (cargo update; cargo upgrade --incompatible if cargo-edit is installed) dependencies"
	@echo "override: make dev ROOT=/path/to/repo PORT=7500 OPEN=0   (OPEN=0: don't open the browser)"

web/node_modules: web/package.json
	cd web && pnpm install
	@touch $@

web: web/node_modules
	cd web && pnpm run build

dev: web/node_modules
	@trap 'kill 0' INT TERM EXIT; \
	(cd web && pnpm run dev) & \
	if [ "$(OPEN)" = 1 ]; then (until curl -sf -o /dev/null http://127.0.0.1:$(PORT)/api/info; do sleep 0.5; done; $(OPENER) http://127.0.0.1:$(PORT)/) & fi; \
	cargo run -p spec17 -- --root "$(ROOT)" serve --port $(PORT) --no-open

serve:
	cargo run -p spec17 -- --root "$(ROOT)" serve --port $(PORT) --no-open

# build.rs runs `pnpm run build` for release builds (SKIP_WEB_BUILD=1 to skip when web/dist is already built)
build: web/node_modules
	cargo build --release -p spec17

run: build
	$(BIN) --root "$(ROOT)" serve --port $(PORT)

# the release build embeds the web UI (build.rs runs pnpm); needs ~/.cargo/bin on PATH
install: web/node_modules
	cargo install --path server --locked --force
	@echo "installed: $$(command -v spec17 || echo 'add ~/.cargo/bin to PATH')   then: spec17 doctor"

uninstall:
	cargo uninstall spec17

# one tarball per OS/arch: build on that OS (rusqlite is bundled, so only a C compiler is needed); CI does the others.
# On macOS the binary is signed first, so an archive always holds a signed one.
# (NFR-07) Distribution builds use both lockfiles and embed a freshly built dashboard.
release-build:
	cd web && pnpm install --frozen-lockfile && pnpm run build
	SKIP_WEB_BUILD=1 cargo build --release --locked -p spec17
	@set -eo pipefail; \
	expected="spec17 $(VERSION)"; \
	if [ -n "$$SPEC17_DEV_HASH" ]; then expected="$$expected-dev+$$SPEC17_DEV_HASH ($$SPEC17_DEV_DATE)"; fi; \
	version=$$($(BIN) --version | sed -n '1p'); \
	[ "$$version" = "$$expected" ] || { echo "expected $$expected, got $$version"; exit 1; }

dist: $(if $(filter darwin,$(OS)),sign,release-build)
	@mkdir -p dist
	tar -czf $(call archive,$(LABEL)) -C target/release spec17 -C $(CURDIR) LICENSE NOTICES
	@cd dist && f=$(notdir $(call archive,$(LABEL))) && { sha256sum $$f 2>/dev/null || shasum -a 256 $$f; } > $$f.sha256
	@ls -la $(call archive,$(LABEL))*

sign: release-build
	@[ "$(OS)" = darwin ] || { echo "sign: macOS only (codesign)"; exit 1; }
	codesign --force --sign "$(SIGN_IDENTITY)" $(BIN)
	codesign --verify --verbose $(BIN)

# The uploads below do what .github/workflows/prerelease.yml does, so macOS and CI can add to one prerelease in any order. They
# never promote it (gh release edit $(TAG) --prerelease=false --latest). The build comes first: a failure changes nothing on GitHub.
check-tag:
	@case "$(TAG)" in "v$(VERSION)" | "v$(VERSION)"-?*) ;; *) echo "TAG $(TAG) must be v$(VERSION) or v$(VERSION)-<suffix>"; exit 1;; esac
	@[ "$$(git rev-parse HEAD)" = "$$(git rev-parse 'refs/tags/$(TAG)^{commit}')" ] || { echo "HEAD is not $(TAG): git checkout $(TAG)"; exit 1; }
	@[ -z "$$(git status --porcelain)" ] || { echo "uncommitted changes: the archive would not be $(TAG)"; exit 1; }

upload: check-tag
	@$(MAKE) --no-print-directory dist LABEL="$(VERSION)"
	gh release view "$(TAG)" >/dev/null 2>&1 || gh release create "$(TAG)" --prerelease --generate-notes --title "$(TAG)" --verify-tag || gh release view "$(TAG)" >/dev/null
	@[ "$$(gh release view "$(TAG)" --json isPrerelease,isDraft --jq '.isPrerelease and (.isDraft | not)')" = true ] \
	  || { echo "$(TAG) must be a published prerelease"; exit 1; }
	gh release upload "$(TAG)" $(call archive,$(VERSION)) $(call archive,$(VERSION)).sha256 --clobber

# (NFR-05) Keep the rolling edge release, with a distinct archive for each dev build.
upload-edge: release-dev

release-dev: DEV_STAMP := $(shell date -u '+%Y%m%d%H%M%S %Y-%m-%dT%H:%M:%SZ')
release-dev: DEV_VERSION = $(word 1,$(DEV_STAMP))
release-dev:
	@[ "$(OS)-$(ARCH)" = darwin-aarch64 ] || { echo "release-dev: Apple Silicon macOS only"; exit 1; }
	@git fetch -q origin main
	@[ "$$(git branch --show-current)" = main ] || { echo "release-dev runs on main"; exit 1; }
	@[ "$$(git rev-parse HEAD)" = "$$(git rev-parse origin/main)" ] || { echo "HEAD is not origin/main"; exit 1; }
	@[ -z "$$(git status --porcelain)" ] || { echo "uncommitted changes: the archive would not be main"; exit 1; }
	@SPEC17_DEV_HASH="$$(git rev-parse --short=12 HEAD)" SPEC17_DEV_DATE="$(word 2,$(DEV_STAMP))" \
	  $(MAKE) --no-print-directory dist LABEL="dev.$(DEV_VERSION)"
	@set -e; \
	file=$(call archive,dev.$(DEV_VERSION)); name=$${file##*/}; \
	read -r sha _ < "$$file.sha256"; \
	repo=$$(gh repo view --json nameWithOwner --jq .nameWithOwner); \
	private=$$(gh repo view --json isPrivate --jq .isPrivate); \
	n="Latest Apple Silicon dev build of main at $$(git rev-parse HEAD)."; \
	git push --force origin HEAD:refs/tags/edge; \
	gh release view edge >/dev/null 2>&1 || gh release create edge --prerelease --verify-tag --title edge --notes "$$n" || gh release view edge >/dev/null; \
	gh release upload edge "$$file" "$$file.sha256"; \
	gh release edit edge --prerelease --notes "$$n"; \
	if [ "$$private" = true ]; then \
	  id=$$(gh api "repos/$$repo/releases/tags/edge" --jq ".assets[] | select(.name == \"$$name\") | .id"); \
	  [ -n "$$id" ] || { echo "missing release asset ID for $$name"; exit 1; }; \
	  url=$$(printf 'url "https://api.github.com/repos/%s/releases/assets/%s", using: :curl, headers: ["Accept: application/octet-stream", "Authorization: token #{ENV["HOMEBREW_GITHUB_API_TOKEN"]}"]' "$$repo" "$$id"); \
	else \
	  url=$$(printf 'url "https://github.com/%s/releases/download/edge/%s"' "$$repo" "$$name"); \
	fi; \
	printf '%s\n' 'class Spec17Dev < Formula' \
	  '  desc "Prebuilt development version of spec17 for Apple Silicon"' \
	  '  homepage "https://spec17.dev"' '  license :cannot_represent' \
	  "  $$url" '  version "$(DEV_VERSION)"' "  sha256 \"$$sha\"" '' \
	  '  depends_on :macos' '  depends_on arch: :arm64' \
	  '  conflicts_with "spec17", because: "both install a spec17 binary"' '' \
	  '  def install' '    bin.install "spec17"' '  end' '' \
	  '  test do' '    system "#{bin}/spec17", "--version"' '  end' 'end' > Formula/spec17-dev.rb; \
	echo "Commit and push Formula/spec17-dev.rb to publish spec17-dev."

notices: web/node_modules ## regenerate NOTICES (licences of the crates and npm packages in the binary)
	node scripts/notices.mjs

report:
	cargo run -q -p spec17 -- --root "$(ROOT)" report

screenshots: ## retake docs/static/img from a deliberately broken tree (restored after); needs Chrome
	node scripts/screenshots.mjs

# (NFR-15) The same build as .github/workflows/pages.yml; --baseURL is only for the published subpath.
docs:
	hugo --source docs --gc --minify --panicOnWarning

# Opens the site once it answers. On macOS with Chrome running, an existing tab on the site is reused (focused; Hugo live reload refreshes it), else a new tab.
# The script returns "new" when no tab matched.
# ponytail: only Chrome tabs are reused; other browsers get a new tab. OPEN=0 skips the browser.
SITE_URL := http://localhost:1313/
define FOCUS_TAB
tell application "Google Chrome"
	repeat with w in windows
		set i to 0
		repeat with t in tabs of w
			set i to i + 1
			if URL of t starts with "$(SITE_URL)" then
				set active tab index of w to i
				set index of w to 1
				activate
				return
			end if
		end repeat
	end repeat
end tell
return "new"
endef
export FOCUS_TAB
site:
	if [ "$(OPEN)" = 1 ]; then (until curl -sf -o /dev/null $(SITE_URL); do sleep 0.5; done; \
		if [ "$(OS)" = darwin ] && pgrep -qx "Google Chrome"; then [ "$$(osascript -e "$$FOCUS_TAB")" != new ] || $(OPENER) $(SITE_URL); else $(OPENER) $(SITE_URL); fi) & fi; \
	hugo server --source docs --port 1313

test:
	cargo test --workspace

# optional: Rust output vs the old Python scripts, if still vendored in $(ROOT)/.agents/skills/spec-driven (no longer part of this repo)
parity:
	@cargo build -q -p spec17
	@set -e; S="$(ROOT)/.agents/skills/spec-driven/scripts"; R="$(CURDIR)/target/debug/spec17"; cd "$(ROOT)"; \
	norm() { sed -e 's/status\.py/spec17 status/g; s/trace\.py/spec17 trace/g'; }; \
	for a in "report.py|report" "status.py|status" "status.py --missing|status --missing" "status.py --issues|status --issues" \
	         "status.py --built|status --built" "status.py --overlap|status --overlap" "status.py --uncovered|status --uncovered"; do \
	  py=$${a%%|*}; rs=$${a##*|}; \
	  if diff <(python3 -B $$S/$$py 2>&1 | norm) <($$R $$rs 2>&1) >/dev/null; then echo "OK   $$rs"; else echo "DIFF $$rs"; exit 1; fi; \
	done

# pnpm goes to latest, past the ranges in package.json; plain cargo only updates Cargo.lock within Cargo.toml ranges, so
# major bumps need cargo-edit (cargo install cargo-edit). Run `make test` and `cd web && pnpm run check` after.
upgrade:
	cd web && pnpm update --latest
	cargo update
	@if cargo upgrade --help >/dev/null 2>&1; then cargo upgrade --incompatible && cargo update; \
	else echo "cargo-edit not installed: Cargo.toml ranges kept (cargo install cargo-edit for major bumps)"; fi

clean:
	cargo clean
	rm -rf web/dist web/node_modules docs/public docs/resources

mcp-inspector: ## Launch MCP Inspector UI
	@echo "🧐 MCP Inspector"
	@export MCP_INSPECTOR_SECRET_STORE=memory && \
	npx -y @modelcontextprotocol/inspector --config mcp-inspector.json

REVS ?= main HEAD
SCALE ?= 1
RUNS ?= 10
bench:
	@python3 bench/bench.py $(REVS) --scale $(SCALE) --runs $(RUNS) $(if $(JSON),--json)

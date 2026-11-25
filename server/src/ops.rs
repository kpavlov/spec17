//! The tools, defined once: the CLI and `/mcp` (the dashboard included) both run these over `spec17-core`.
//! Every tool returns plain text (what the Python scripts printed) and a JSON object (the same data, typed).
use serde_json::{Value, json};
use spec17_core::{Analysis, ChangeOpts, StatusOpts};
use std::path::{Path, PathBuf};

/// Why a tool call failed. `NoIndex` is "not checked" (no CodeGraph index: the user decides about indexing), which a resource shows as content.
#[derive(Debug)]
pub enum ToolError {
    NoIndex(String),
    Other(String),
}

impl std::fmt::Display for ToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let (Self::NoIndex(m) | Self::Other(m)) = self;
        f.write_str(m)
    }
}

impl From<String> for ToolError {
    fn from(m: String) -> Self {
        Self::Other(m)
    }
}

impl From<&str> for ToolError {
    fn from(m: &str) -> Self {
        Self::Other(m.into())
    }
}

pub struct Tool {
    pub name: &'static str,
    pub description: &'static str,
    pub schema: Value,
    /// Opens the dashboard in an MCP Apps host (only `dashboard` does: one app, the page comes from its `view` argument).
    pub ui: bool,
}

impl Tool {
    // (API-18) New tools default to not read-only until reviewed.
    pub fn read_only(&self) -> bool {
        matches!(self.name, "report" | "status" | "changes" | "diff" | "history" | "commits" | "blame" | "trace" | "search_tests" | "search_requirements" | "search_modules" | "overlap" | "uncovered" | "file" | "source" | "doctor" | "install_status" | "dashboard")
    }
}

pub const NOTE: &str = "Evidence, not truth: a citation or tag does not prove the requirement is implemented or that its test passes.";

pub fn tools() -> &'static [Tool] {
    static TOOLS: std::sync::LazyLock<Vec<Tool>> = std::sync::LazyLock::new(tool_list);
    &TOOLS
}

fn tool_list() -> Vec<Tool> {
    let status_props = json!({
        "spec": {"type": "string", "description": "only spec files whose name contains this, e.g. 04"},
        "prefix": {"type": "string", "description": "only IDs with this prefix, e.g. DOC"},
        "all": {"type": "boolean", "description": "list every requirement"},
        "missing": {"type": "boolean", "description": "due requirements with no test"},
        "changed": {"type": "string", "description": "requirements touched by the working tree vs this git ref (e.g. HEAD)"},
        "staged": {"type": "boolean", "description": "with changed (HEAD if not given): only what is staged"},
        "include_untracked": {"type": "boolean", "description": "with changed: count untracked files (default true)"},
        "ignore_whitespace": {"type": "boolean", "description": "with changed: leave out changes that only touch whitespace"},
        "built": {"type": "boolean", "description": "requirements marked later/deferred that already have tests or code"},
        "done": {"type": "boolean", "description": "check-marked requirements, each with its evidence verdict"},
        "unproven": {"type": "boolean", "description": "check-marked requirements with no test or no link to code"},
        "issues": {"type": "boolean", "description": "requirements with a spec issue"},
        "effort": {"type": "string", "enum": ["low", "medium", "high"], "description": "evidence per listed line"},
        "limit": {"type": "integer", "description": "max listed lines (default 200)"}
    });
    let change_props = json!({
        "ref": {"type": "string", "description": "git ref the working tree is compared with (default HEAD), e.g. main or origin/main"},
        "commit": {"type": "string", "description": "explain this one commit against its first parent (sha or ref; ignores ref, staged and include_untracked)"},
        "staged": {"type": "boolean", "description": "only what is staged"},
        "include_untracked": {"type": "boolean", "description": "count untracked files (default true)"},
        "ignore_whitespace": {"type": "boolean", "description": "leave out changes that only touch whitespace"},
        "show_ignored": {"type": "boolean", "description": "also files .aiignore, .spec17ignore and .agents hide (default false)"},
        "show_all": {"type": "boolean", "description": "also files spec17 does not scan: kind other (default false)"}
    });
    let path = json!({"path": {"type": "string", "description": "only files whose path contains this"}});
    vec![
        Tool { name: "report", description: "Whole-project spec-driven status: coverage headline, problem tally and one section per problem kind. Deterministic. Evidence, not truth.", schema: obj(json!({"limit": {"type": "integer", "description": "lines per problem section (default 12)"}})), ui: false },
        Tool { name: "status", description: "Per-spec-file summary, or one line per requirement when a list filter is given (missing, changed, built, done, unproven, issues, all). Deterministic. Evidence, not truth.", schema: obj(status_props), ui: false },
        Tool { name: "changes", description: "What changed against a git ref (HEAD by default) or in one commit: every changed file (added, modified, deleted, renamed; staged, unstaged, untracked or committed; spec, test, src, and other with show_all) and the requirements the changed lines touch: a removed bullet, an edited test or a cited function, with what happened to it. Evidence of edits, not that anything is done.", schema: obj(change_props.clone()), ui: false },
        Tool { name: "diff", description: "Unified diff of one changed file against a ref or in one commit (a rename shows both paths; an untracked file is all additions), and under each hunk the requirements it touches. Takes the same arguments as `changes`, and only files it lists. Shows what was edited, not whether it is right.", schema: required(obj(merge(change_props.clone(), json!({"path": {"type": "string", "description": "repo-relative path of a changed file, as in `changes`"}}))), "path"), ui: false },
        Tool { name: "history", description: "Commits on a branch (HEAD by default) that are not on the base branch (main or master by default), newest first, and how many files are uncommitted. Leaves out commits that change no file the filters show (ignore_whitespace, show_ignored, show_all as in `changes`). Explain one with `changes commit=<sha>`. Needs git.", schema: obj(json!({"base": {"type": "string", "description": "branch the commits are not on (default: main, master, origin/main or origin/master)"}, "head": {"type": "string", "description": "branch to list (default HEAD)"}, "limit": {"type": "integer", "description": "max commits (default 50)"}, "ignore_whitespace": change_props["ignore_whitespace"], "show_ignored": change_props["show_ignored"], "show_all": change_props["show_all"]})), ui: false },
        Tool { name: "commits", description: "For each requirement, the newest commit that touched its lines (sha, short, author, date, subject), by git blame of its spec file: who last changed the specification and when; a requirement with an uncommitted line has none. When the text was last changed, not whether anyone reviewed it. Needs git.", schema: obj(json!({})), ui: false },
        Tool { name: "blame", description: "Who last changed the given lines of a scanned spec, test or source file: for each line the commit (sha, short, author, date, subject), by git blame; a line not committed yet has none. For a test pass the line of its tag or name, as `trace` and `spec17://tests` list it. Needs git.", schema: merge(obj(json!({"path": {"type": "string", "description": "repo-relative path of a scanned file, e.g. core/tests/core.rs"}, "lines": {"type": "array", "items": {"type": "integer"}, "description": "1-based line numbers"}})), json!({"required": ["path", "lines"]})), ui: false },
        Tool { name: "trace", description: "One requirement in depth: spec text, tests tagged with it, source citations with their declaration, related code. Evidence, not truth.", schema: required(obj(json!({"id": {"type": "string", "description": "requirement ID, e.g. DOC-25"}})), "id"), ui: false },
        Tool { name: "search_tests", description: "Search detected tests, including untagged tests. Detected, not run; results do not say tests pass.", schema: obj(json!({"module": {"type": "string", "description": "exact module path"}, "state": {"type": "string", "enum": ["all", "tagged", "untagged"]}, "query": {"type": "string", "description": "case-insensitive text in file, test name or requirement IDs"}, "requirement_id": {"type": "string", "description": "exact requirement ID tagged on the test (case-insensitive)"}, "changed": {"type": "boolean", "description": "only files changed against HEAD"}})), ui: false },
        Tool { name: "search_requirements", description: "Find requirements tagged in a test file or on detected tests at or near the given lines. Tags are evidence, not proof of coverage or passing tests.", schema: required(obj(json!({
            "test_file": {"type": "string", "minLength": 1, "description": "exact repo-relative path of a scanned test file"},
            "line": {"type": "integer", "minimum": 1, "description": "1-based line; select the nearest detected test starting at or before this line. Omit to search the entire file"},
            "end_line": {"type": "integer", "minimum": 1, "description": "inclusive; also include tests starting after line through end_line. Requires line"},
        })), "test_file"), ui: false },
        Tool { name: "search_modules", description: "Search modules by path or changed files; includes module counts and dependencies. Index-derived data may be unavailable.", schema: obj(json!({"query": {"type": "string", "description": "case-insensitive text in module path"}, "changed": {"type": "boolean", "description": "only modules with files changed against HEAD"}})), ui: false },
        Tool { name: "overlap", description: "Code covered by 2+ requirements (CodeGraph index): where two specs may conflict or complement each other. A candidate list, not a verdict.", schema: obj(merge(path.clone(), json!({"all": {"type": "boolean", "description": "also pairs from the same spec file"}}))), ui: false },
        Tool { name: "uncovered", description: "Code no requirement covers (CodeGraph index): candidates for a missing requirement, not proof.", schema: obj(merge(path, json!({"fanin": {"type": "integer"}, "min": {"type": "integer"}}))), ui: false },
        Tool { name: "file", description: "A spec file's markdown (read-only; name is a file in the specs folder, e.g. 04-documents.md). Without name: the list of spec files.", schema: obj(json!({"name": {"type": "string", "description": "spec file name, e.g. 04-documents.md"}})), ui: false },
        Tool { name: "source", description: "A test or source file's text (read-only; path is one of the scanned test/source files, as in trace output).", schema: obj(json!({"path": {"type": "string", "description": "repo-relative path, e.g. src/main.rs"}})), ui: false },
        Tool { name: "doctor", description: "Repo setup checks: specs, tests and source folders, git, the codegraph binary and whether the CodeGraph index is initialized, readable and fresh, skill and MCP entry. Only looks; never indexes or installs.", schema: obj(json!({})), ui: false },
        Tool { name: "codegraph", description: "WRITES FILES, SLOW: build (action=init, when there is no .codegraph) or refresh (action=sync) the CodeGraph index by running the codegraph binary; action=refresh is the dashboard's rescan: sync when an index exists and the binary is on PATH, also beside a running codegraph (its watcher can be off); otherwise it says why it did nothing. Indexing is the user's decision: call only when the user asked for it. Same as `spec17 doctor --fix`.", schema: required(obj(json!({"action": {"type": "string", "enum": ["init", "sync", "refresh"]}})), "action"), ui: false },
        Tool { name: "install_status", description: "Whether the spec17 skill and HTTP and stdio MCP entries are installed in this repo or home directory. Also reads the HTTP entry in ~/.claude.json.", schema: obj(json!({})), ui: false },
        Tool { name: "install", description: "WRITES FILES: install the spec17 skill and HTTP MCP in this repo (global=true: home directory). Use only=skill or only=mcp to install one component. With only=mcp, transport=stdio installs spec17-stdio independently. With server=codegraph, add only the CodeGraph entry to the repo and keep any existing entry.", schema: obj(json!({"server": {"type": "string", "enum": ["spec17", "codegraph"]}, "global": {"type": "boolean"}, "only": {"type": "string", "enum": ["skill", "mcp"]}, "transport": {"type": "string", "enum": ["http", "stdio"], "description": "requires only=mcp (default: http)"}, "port": {"type": "integer", "minimum": 1, "maximum": 65535, "description": "HTTP MCP port (default: the port this server runs on)"}})), ui: false },
        Tool { name: "uninstall", description: "WRITES FILES: remove the spec17 skill and HTTP MCP in this repo. Use only=skill or only=mcp to remove one component. With only=mcp, transport=stdio removes spec17-stdio independently. With global=true, remove the global skill by default or the selected entry in ~/.mcp.json with only=mcp. Set server=codegraph to remove the CodeGraph entry in the repo.", schema: obj(json!({"server": {"type": "string", "enum": ["spec17", "codegraph"]}, "global": {"type": "boolean"}, "only": {"type": "string", "enum": ["skill", "mcp"]}, "transport": {"type": "string", "enum": ["http", "stdio"], "description": "requires only=mcp (default: http)"}})), ui: false },
        Tool { name: "module", description: "WRITES FILES: set a module hidden (true) or shown (false) by adding or removing its folder line in <repo>/.spec17ignore (gitignore syntax), keeping every other line. To hide, path is a module listed by spec17://modules; to show, one of its `hidden`. Only when the user asked for it.", schema: merge(obj(json!({"path": {"type": "string", "description": "module path, e.g. server"}, "hidden": {"type": "boolean", "description": "true: leave the module out of every scan; false: list it again"}})), json!({"required": ["path", "hidden"]})), ui: false },
        Tool { name: "dashboard", description: "URL of the spec17 web dashboard served by this process; in an MCP Apps host this tool opens the dashboard. view (optional) picks the page: a tab, or a tab with its selection and filters, e.g. modules, changes, requirements/DOC-25, requirements?changed=1, tests?module=core&state=untagged, requirements?view=docs&file=04-documents.md, settings.", schema: obj(json!({"view": {"type": "string", "description": "tab or route to open, e.g. requirements/DOC-25?state=none (default: the dashboard)"}})), ui: true },
    ]
}

fn change_opts(args: &Value) -> ChangeOpts {
    let on = |k: &str| args[k].as_bool().unwrap_or(false);
    ChangeOpts {
        reference: txt(args, "ref").into(), commit: txt(args, "commit").into(), staged: on("staged"), no_untracked: args["include_untracked"].as_bool() == Some(false),
        ignore_whitespace: on("ignore_whitespace"), show_ignored: on("show_ignored"), show_all: on("show_all"),
    }
}

/// The install tools: they write the skill and `.mcp.json`, so they never run an analysis.
fn setup(root: Option<&Path>, name: &str, args: &Value, mcp_port: u16) -> Option<Result<(String, Value), String>> {
    use crate::skill;
    if !matches!(name, "install_status" | "install" | "uninstall") { return None; }
    Some((|| {
        let changed = if name == "install_status" {
            vec![]
        } else {
            let global = match args.get("global") {
                None => false,
                Some(v) => v.as_bool().ok_or("global must be a boolean")?,
            };
            let codegraph = match args.get("server") {
                None => false,
                Some(v) if v == "spec17" => false,
                Some(v) if v == "codegraph" => true,
                Some(_) => return Err("server must be spec17 or codegraph".into()),
            };
            if codegraph {
                if global || args.get("only").is_some() || args.get("transport").is_some() {
                    return Err("global, only and transport apply to spec17".into());
                }
                skill::apply_codegraph(root, name == "install")?
            } else {
                let only = args.get("only").map(|v| serde_json::from_value(v.clone())).transpose()
                    .map_err(|_| "only must be skill or mcp")?;
                if args.get("transport").is_some() && only != Some(skill::InstallPart::Mcp) {
                    return Err("transport requires only=mcp".into());
                }
                let transport = args.get("transport").map(|v| serde_json::from_value(v.clone())).transpose()
                    .map_err(|_| "transport must be http or stdio")?.unwrap_or_default();
                let port = if name == "install" {
                    Some(match args.get("port") {
                        None => mcp_port,
                        Some(v) => v.as_u64().and_then(|n| u16::try_from(n).ok()).filter(|&n| n != 0).ok_or("port must be between 1 and 65535")?,
                    })
                } else { None };
                skill::apply(root, global, port, only, transport)?
            }
        };
        let st = skill::status(root, mcp_port)?;
        let mut text = changed.join("\n");
        let todo = skill::suggestions(&st);
        text += &(if todo.is_empty() { "\nready: the agent can use spec17 here".to_string() } else { format!("\n{}", todo.join("\n")) });
        // (WEB-42) Use the same version as the CLI for the Settings screen.
        Ok((text.trim_start().to_string() + "\n", json!({"status": st, "suggestions": todo, "version": crate::VERSION})))
    })())
}

fn obj(props: Value) -> Value {
    json!({"type": "object", "properties": props})
}

fn required(mut schema: Value, field: &str) -> Value {
    schema["required"] = json!([field]);
    schema
}

fn merge(mut a: Value, b: Value) -> Value {
    a.as_object_mut().unwrap().extend(b.as_object().unwrap().clone());
    a
}

fn num(args: &Value, k: &str) -> Option<i64> {
    args.get(k).and_then(Value::as_i64)
}

fn txt<'a>(args: &'a Value, k: &str) -> &'a str {
    args.get(k).and_then(Value::as_str).unwrap_or("")
}

// (API-01)
/// Run one tool against the repo at `root` (or the nearest ancestor of the cwd with a specs folder). Blocking.
pub fn call(root: Option<&Path>, name: &str, args: &Value, dashboard: &str, mcp_port: u16) -> Result<(String, Value), ToolError> {
    if name == "dashboard" {
        if dashboard.is_empty() {
            return Ok(("The web dashboard is disabled.\n".into(), json!({"url": ""})));
        }
        let url = format!("{dashboard}{}", args["view"].as_str().unwrap_or("").trim_start_matches('/'));
        return Ok((format!("{url}\n"), json!({"url": url})));
    }
    if name == "doctor" {
        let checks = crate::doctor::run(root, mcp_port);
        return Ok((crate::doctor::text(&checks), json!({"checks": checks, "index_action": crate::doctor::suggested(&checks)})));
    }
    if name == "codegraph" {
        let text = crate::doctor::index(root, txt(args, "action"))?;
        return Ok((text.clone(), json!({"output": text})));
    }
    if let Some(r) = setup(root, name, args, mcp_port) {
        return r.map_err(ToolError::Other);
    }
    // these read files by names the scanner lists: a Project is enough, no analysis
    if matches!(name, "file" | "source") {
        return Ok(read_listed(&spec17_core::scan::Project::open(root)?, name, args)?);
    }
    let a = Analysis::load(root)?;
    let note = |a: &Analysis| a.graph_note.clone().unwrap_or_else(|| "spec17: no .codegraph index here; the user decides about indexing".into());
    let out = match name {
        "report" => {
            let r = a.report();
            (r.text(&a, num(args, "limit").unwrap_or(12).max(0) as usize), json!(r))
        }
        "status" => {
            let o = StatusOpts {
                spec: args.get("spec").and_then(Value::as_str).map(str::to_owned),
                prefix: args.get("prefix").and_then(Value::as_str).map(str::to_owned),
                all: args["all"].as_bool().unwrap_or(false),
                missing: args["missing"].as_bool().unwrap_or(false),
                changed: args.get("changed").and_then(Value::as_str).filter(|s| !s.is_empty()).map(str::to_owned),
                staged: args["staged"].as_bool().unwrap_or(false),
                no_untracked: args["include_untracked"].as_bool() == Some(false),
                ignore_whitespace: args["ignore_whitespace"].as_bool().unwrap_or(false),
                built: args["built"].as_bool().unwrap_or(false),
                done: args["done"].as_bool().unwrap_or(false),
                unproven: args["unproven"].as_bool().unwrap_or(false),
                issues: args["issues"].as_bool().unwrap_or(false),
                effort: txt(args, "effort").to_string(),
            };
            let mut st = a.status(&o)?;
            let limit = num(args, "limit").unwrap_or(200).max(0) as usize;
            let more = st.rows.len().saturating_sub(limit);
            st.rows.truncate(limit);
            let mut text = st.text(&o);
            if more > 0 {
                text = format!("(first {limit} of {} listed; narrow with spec/prefix or raise limit)\n{text}", limit + more);
            }
            (text, json!(st))
        }
        "changes" => {
            let c = a.changes(&change_opts(args))?;
            (c.text(), json!(c))
        }
        "diff" => {
            let d = a.file_diff(txt(args, "path"), &change_opts(args))?;
            (d.text(), json!(d))
        }
        "history" => {
            let h = a.history(txt(args, "base"), txt(args, "head"), num(args, "limit").unwrap_or(50).clamp(1, 500) as usize, &change_opts(args))?;
            (h.text(), json!(h))
        }
        "commits" => {
            let c = a.blame_commits()?;
            (c.iter().map(|(i, c)| format!("{i} {} {} {} {}\n", c.short, c.date, c.author, c.subject)).collect(), json!({"commits": c}))
        }
        "blame" => {
            let lines: Vec<usize> = args["lines"].as_array().map(|l| l.iter().filter_map(|n| n.as_u64().map(|n| n as usize)).collect()).unwrap_or_default();
            let b = a.blame_at(txt(args, "path"), &lines)?;
            (b.iter().map(|(n, c)| format!("{}:{n} {} {} {} {}\n", txt(args, "path"), c.short, c.date, c.author, c.subject)).collect(), json!({"path": txt(args, "path"), "lines": b}))
        }
        "trace" => match a.trace(txt(args, "id")) {
            Some(t) => (t.text(), json!(t)),
            None => return Err(format!("{}: not in specs", txt(args, "id").to_uppercase()).into()),
        },
        // (API-07)
        "search_modules" => {
            let mut view = a.modules_view();
            let query = txt(args, "query").to_lowercase();
            let changed = if args["changed"].as_bool().unwrap_or(false) {
                let files = a.changes(&ChangeOpts::default())?.files;
                let mut seen = std::collections::HashMap::new();
                Some(files.iter().map(|f| spec17_core::detect::module_of(&a.p.root, &f.path, &mut seen).0).collect::<std::collections::HashSet<_>>())
            } else { None };
            view.modules.retain(|m| m.path.to_lowercase().contains(&query) && changed.as_ref().is_none_or(|c| c.contains(&m.path)));
            view.hidden.retain(|m| m.to_lowercase().contains(&query));
            (serde_json::to_string_pretty(&view).map_err(|e| e.to_string())?, json!(view))
        }
        // (API-07, API-19)
        "search_tests" => {
            let mut view = a.tests_view();
            let changed = if args["changed"].as_bool().unwrap_or(false) { Some(a.changes(&ChangeOpts::default())?.files.into_iter().map(|f| f.path).collect::<std::collections::HashSet<_>>()) } else { None };
            let query = txt(args, "query").to_lowercase();
            let requirement_id = txt(args, "requirement_id");
            let state = txt(args, "state");
            if !matches!(state, "" | "all" | "tagged" | "untagged") { return Err("state: expected all, tagged or untagged".into()); }
            view.modules.retain_mut(|m| {
                if !txt(args, "module").is_empty() && m.module != txt(args, "module") { return false; }
                m.files.retain_mut(|f| {
                    if changed.as_ref().is_some_and(|c| !c.contains(&f.file)) { return false; }
                    f.tests.retain(|t| {
                        (state != "tagged" || !t.ids.is_empty())
                            && (state != "untagged" || t.ids.is_empty())
                            && (requirement_id.is_empty() || t.ids.iter().any(|id| id.eq_ignore_ascii_case(requirement_id)))
                            && (query.is_empty() || format!("{} {} {}", f.file, t.name, t.ids.join(" ")).to_lowercase().contains(&query))
                    });
                    !f.tests.is_empty()
                });
                m.tests = m.files.iter().map(|f| f.tests.len()).sum();
                m.tests > 0
            });
            view.total = view.modules.iter().map(|m| m.tests).sum();
            (serde_json::to_string_pretty(&view).map_err(|e| e.to_string())?, json!(view))
        }
        // (API-20)
        "search_requirements" => {
            let file = txt(args, "test_file");
            if file.is_empty() { return Err("test_file: expected a repo-relative test file path".into()); }
            let line_number = |key: &str| -> Result<Option<usize>, ToolError> {
                args.get(key).map(|v| {
                    v.as_u64().and_then(|n| usize::try_from(n).ok()).filter(|n| *n > 0)
                        .ok_or_else(|| format!("{key}: expected a positive integer").into())
                }).transpose()
            };
            let lines = match (line_number("line")?, line_number("end_line")?) {
                (Some(line), Some(end)) if end < line => return Err("end_line: must be at least line".into()),
                (Some(line), end) => Some(line..=end.unwrap_or(line)),
                (None, Some(_)) => return Err("end_line: requires line".into()),
                (None, None) => None,
            };
            let result = RequirementSearch { rows: a.requirements_in_test_file(file, lines)? };
            (serde_json::to_string_pretty(&result).map_err(|e| e.to_string())?, json!(result))
        }
        "overlap" => {
            let rows = a.overlap(txt(args, "path"), args["all"].as_bool().unwrap_or(false)).map_err(|e| e.to_string())?.ok_or_else(|| ToolError::NoIndex(note(&a)))?;
            (a.overlap_text(&rows, 25), json!({"rows": rows}))
        }
        "uncovered" => {
            let u = a.uncovered(txt(args, "path"), num(args, "fanin").unwrap_or(5).max(0) as usize, num(args, "min").unwrap_or(10)).map_err(|e| e.to_string())?.ok_or_else(|| ToolError::NoIndex(note(&a)))?;
            (u.text(), json!(u))
        }
        "module" => {
            let hidden = args["hidden"].as_bool().ok_or("hidden: true or false")?;
            let want = txt(args, "path").trim_matches('/');
            let msg = spec17_core::detect::Ignore::set_module_hidden(&a.p.root, want, hidden, || a.modules_view().modules.iter().any(|m| m.path == want))?;
            (msg, json!({"path": want, "hidden": args["hidden"]}))
        }
        other => return Err(format!("unknown tool {other}").into()),
    };
    Ok(out)
}

// (API-03)
/// `file` and `source`: a spec file, or a test or source file, but only by a name the scanner itself lists.
fn read_listed(p: &spec17_core::scan::Project, name: &str, args: &Value) -> Result<(String, Value), String> {
    if name == "file" {
        let names = p.spec_files();
        let want = txt(args, "name");
        if want.is_empty() {
            return Ok((names.join("\n") + "\n", json!({"files": names})));
        }
        if !names.iter().any(|n| n == want) {
            return Err(format!("{want}: not a spec file"));
        }
        let md = std::fs::read_to_string(p.root.join(want)).map_err(|e| e.to_string())?;
        return Ok((md.clone(), json!({"name": want, "markdown": md})));
    }
    let want = txt(args, "path");
    if !p.test_files().iter().chain(p.src_files().iter()).any(|f| f == want) {
        return Err(format!("{want}: not a scanned test or source file"));
    }
    let text = std::fs::read_to_string(p.root.join(want)).map_err(|e| e.to_string())?;
    Ok((text.clone(), json!({"path": want, "text": text})))
}

/// Read-only JSON MCP resources: (uri, name, description, JSON text of the analysis). Listed and read through this table only.
pub const RESOURCES: [(&str, &str, &str, fn(&Analysis) -> serde_json::Result<String>); 2] = [
    ("spec17://modules", "spec17 modules", "Modules of the repo (nearest build manifest, else the folder above src): files, languages, tests, CodeGraph symbols and the modules each depends on. Evidence, not truth.", |a| serde_json::to_string_pretty(&a.modules_view())),
    ("spec17://tests", "spec17 tests", "Test cases detected in the test files, grouped by module and file, with the requirement IDs tagged on each. Detected, not run: it does not say a test passes.", |a| serde_json::to_string_pretty(&a.tests_view())),
];

/// The read-only tools that take no argument, also as MCP resources: `spec17://<tool>` as the text the CLI prints and
/// `spec17://<tool>.json` as the tool's `structuredContent`, typed by the schema at `spec17://schemas/<tool>`. Tools with arguments
/// (`trace`, status filters) and the writing ones stay tools only. Reading `uncovered` or `overlap` without a CodeGraph index gives the
/// "not checked" note as text, and an error as JSON (there is no typed data).
pub const VIEW_TOOLS: [&str; 5] = ["changes", "report", "status", "uncovered", "overlap"];

pub const SCHEMA_PREFIX: &str = "spec17://schemas/";
pub const JSON_MIME: &str = "application/json";
pub const SCHEMA_MIME: &str = "application/schema+json";
pub const TEXT_MIME: &str = "text/plain";

/// The description of a tool, for the resource that shows the same thing.
pub fn tool_description(name: &str) -> &'static str {
    tools().iter().find(|t| t.name == name).map_or("", |t| t.description)
}

/// Every resource that can change when the repo does (and so can be subscribed to). The schemas never change.
pub fn resource_uris() -> impl Iterator<Item = String> {
    RESOURCES.iter().map(|r| r.0.to_string()).chain(VIEW_TOOLS.iter().flat_map(|t| [format!("spec17://{t}"), format!("spec17://{t}.json")]))
}

/// The schema a JSON resource is typed by, as its `spec17://schemas/<name>` URI.
pub fn schema_uri(resource: &str) -> Option<String> {
    let name = resource.strip_prefix("spec17://")?;
    let name = name.strip_suffix(".json").unwrap_or(name);
    schema(name).map(|_| format!("{SCHEMA_PREFIX}{name}"))
}

/// Names that have a published schema (`spec17://schemas/<name>`): the JSON resources.
pub const SCHEMAS: [&str; 7] = ["report", "status", "changes", "uncovered", "overlap", "modules", "tests"];

/// The schema of a type. schemars' numeric `format` hints (`uint`, `int64`, ...) are not JSON Schema formats: validators warn on them.
fn of<T: schemars::JsonSchema>() -> Value {
    fn plain(v: &mut Value) {
        match v {
            Value::Object(m) => {
                if m.get("format").and_then(Value::as_str).is_some_and(|f| f.starts_with("uint") || f.starts_with("int") || f.starts_with("double") || f.starts_with("float")) {
                    m.remove("format");
                }
                // `"type": ["string", "null"]` is legal but less portable (Gemini/OpenAPI-style clients): use `anyOf` of single types.
                let nullable = match m.get("type").and_then(Value::as_array).map(Vec::as_slice) {
                    Some([t, n]) if t.is_string() && n == "null" => Some(t.clone()),
                    _ => None,
                };
                if let Some(t) = nullable {
                    let mut branch = std::mem::take(m);
                    for k in ["title", "description", "default"] {
                        if let Some(x) = branch.remove(k) {
                            m.insert(k.into(), x);
                        }
                    }
                    branch.insert("type".into(), t);
                    m.insert("anyOf".into(), json!([branch, {"type": "null"}]));
                }
                m.values_mut().for_each(plain);
            }
            Value::Array(a) => a.iter_mut().for_each(plain),
            _ => {}
        }
    }
    let mut v = serde_json::to_value(schemars::schema_for!(T)).unwrap_or_default();
    plain(&mut v);
    v
}

fn strs() -> Value {
    json!({"type": "array", "items": {"type": "string"}})
}

fn shape(props: Value, required: &[&str]) -> Value {
    json!({"$schema": "https://json-schema.org/draft/2020-12/schema", "type": "object", "properties": props, "required": required})
}

#[derive(serde::Serialize, schemars::JsonSchema)]
struct RequirementSearch {
    rows: Vec<spec17_core::report::Row>,
}

/// JSON Schema of a result's structured content: `report`, `status`, `trace`, `overlap`, `uncovered`, `modules`, `tests`,
/// `file`, `source`, `doctor`, `codegraph`, `install`, `dashboard`. Typed from the core structs where there are any.
pub fn schema(name: &str) -> Option<Value> {
    use spec17_core::{Changes, Commit, FileDiff, History, Report, Status, Trace, coverage::{OverlapRow, Uncovered}, modules::{ModulesView, TestsView}};
    #[derive(schemars::JsonSchema)]
    #[allow(dead_code)]
    struct Overlap {
        rows: Vec<OverlapRow>,
    }
    #[derive(schemars::JsonSchema)]
    #[allow(dead_code)]
    struct Commits {
        commits: std::collections::BTreeMap<String, Commit>,
    }
    #[derive(schemars::JsonSchema)]
    #[allow(dead_code)]
    struct Blames {
        path: String,
        lines: std::collections::BTreeMap<usize, Commit>,
    }
    #[derive(schemars::JsonSchema)]
    #[allow(dead_code)]
    struct Doctor {
        checks: Vec<crate::doctor::Check>,
        index_action: Option<String>,
    }
    #[derive(schemars::JsonSchema)]
    #[allow(dead_code)]
    struct Install {
        status: crate::skill::InstallStatus,
        suggestions: Vec<String>,
    }
    Some(match name {
        "report" => of::<Report>(),
        "status" => of::<Status>(),
        "changes" => of::<Changes>(),
        "trace" => of::<Trace>(),
        "uncovered" => of::<Uncovered>(),
        "overlap" => of::<Overlap>(),
        "modules" | "search_modules" => of::<ModulesView>(),
        "tests" | "search_tests" => of::<TestsView>(),
        "search_requirements" => of::<RequirementSearch>(),
        "doctor" => of::<Doctor>(),
        "file" => shape(json!({"files": strs(), "name": {"type": "string"}, "markdown": {"type": "string"}}), &[]),
        "diff" => of::<FileDiff>(),
        "history" => of::<History>(),
        "commits" => of::<Commits>(),
        "blame" => of::<Blames>(),
        "source" => shape(json!({"path": {"type": "string"}, "text": {"type": "string"}}), &["path", "text"]),
        "codegraph" => shape(json!({"output": {"type": "string"}}), &["output"]),
        "install" => of::<Install>(),
        "module" => shape(json!({"path": {"type": "string"}, "hidden": {"type": "boolean"}}), &["path", "hidden"]),
        "dashboard" => shape(json!({"url": {"type": "string"}}), &["url"]),
        _ => return None,
    })
}

/// The `outputSchema` of a tool: its result's `structuredContent`.
pub fn output_schema(tool: &str) -> Option<Value> {
    schema(if tool.starts_with("install") || tool == "uninstall" { "install" } else { tool })
}

// (API-07, API-08)
/// (mime type, text) of a resource, `None` for an unknown URI.
pub fn read_resource(root: Option<&Path>, uri: &str) -> Result<Option<(&'static str, String)>, String> {
    let tool_of = |suffix: &str| uri.strip_prefix("spec17://").and_then(|n| n.strip_suffix(suffix)).and_then(|n| VIEW_TOOLS.iter().find(|t| **t == n));
    if let Some(tool) = tool_of("") {
        return match call(root, tool, &json!({}), "", 0) {
            Ok((text, _)) => Ok(Some((TEXT_MIME, text))),
            // "not checked" (no usable CodeGraph index) is content here, not an error: indexing is the user's decision
            Err(ToolError::NoIndex(note)) => Ok(Some((TEXT_MIME, note))),
            Err(e) => Err(e.to_string()),
        };
    }
    if let Some(tool) = tool_of(".json") {
        let (_, data) = call(root, tool, &json!({}), "", 0).map_err(|e| e.to_string())?;
        return serde_json::to_string_pretty(&data).map(|t| Some((JSON_MIME, t))).map_err(|e| e.to_string());
    }
    if let Some(name) = uri.strip_prefix(SCHEMA_PREFIX).filter(|n| SCHEMAS.contains(n)) {
        return Ok(schema(name).map(|s| (SCHEMA_MIME, s.to_string())));
    }
    let Some((.., render)) = RESOURCES.iter().find(|r| r.0 == uri) else { return Ok(None) };
    render(&Analysis::load(root)?).map(|t| Some((JSON_MIME, t))).map_err(|e| e.to_string())
}

// (API-08)
/// A fingerprint (path, mtime, size) of what the analysis reads: the repo's files, the spec files, `spec17.json` and the
/// CodeGraph index with its WAL. Equal stamps mean the resources read the same. 0 when the repo cannot be opened. Blocking.
// ponytail: `Project::open_unread` (config, ignore files, file list and walks, no spec file read) per poll and per subscriber; share one poller or use a file watcher if that ever costs.
pub fn stamp(root: Option<&Path>) -> u64 {
    use std::hash::{Hash, Hasher};
    let Ok(p) = spec17_core::scan::Project::open_unread(root) else { return 0 };
    let db = p.cfg.codegraph.clone();
    let mut h = std::collections::hash_map::DefaultHasher::new();
    let extra = ["spec17.json".to_string(), ".git/index".to_string(), ".git/HEAD".to_string(), ".git/logs/HEAD".to_string(), format!("{db}-wal"), format!("{db}-shm"), db];
    for rel in p.files.iter().chain(p.spec_files()).chain(&extra) {
        if let Ok(m) = std::fs::metadata(p.root.join(rel)) {
            (rel, m.len(), m.modified().ok()).hash(&mut h);
        }
    }
    h.finish()
}

/// What the dashboard shows in its header.
pub fn info(root: Option<&PathBuf>) -> Result<Value, String> {
    let p = spec17_core::scan::Project::open(root.map(|r| r.as_path()))?;
    Ok(json!({"root": p.root, "note": NOTE}))
}

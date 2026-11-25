//! `spec17 doctor`: is this repo set up for spec17, and is CodeGraph usable here? `run` only looks and suggests. The one thing that writes is `index`, which runs only when the user asks (`doctor --fix`, the Settings tab buttons, the dashboard's rescan).
use serde::Serialize;
use spec17_core::{Analysis, codegraph::{Graph, GraphError}, scan::Project};
use crate::skill::{InstallStatus, McpState, SkillState};
use std::path::Path;
use std::time::SystemTime;

#[derive(Serialize, schemars::JsonSchema)]
pub struct Check {
    pub name: &'static str,
    /// `ok`, `info` (nothing to fix), `warn` (works, but something is missing) or `fail` (spec17 cannot work here).
    pub level: &'static str,
    pub detail: String,
    pub hint: Option<String>,
    /// The index action this check calls for (`init` or `sync`); not part of the output.
    #[serde(skip)]
    pub action: Option<&'static str>,
}

fn c(name: &'static str, level: &'static str, detail: impl Into<String>, hint: Option<&str>) -> Check {
    Check { name, level, detail: detail.into(), hint: hint.map(str::to_owned), action: None }
}

fn mtime(p: &Path) -> Option<SystemTime> {
    std::fs::metadata(p).and_then(|m| m.modified()).ok()
}

/// `codegraph --version`, or None when the binary is not on PATH.
fn codegraph_version() -> Option<String> {
    let out = std::process::Command::new("codegraph").arg("--version").output().ok()?;
    let text = String::from_utf8_lossy(if out.stdout.is_empty() { &out.stderr } else { &out.stdout }).trim().to_string();
    Some(text.lines().next().unwrap_or("").to_string())
}

/// How to install CodeGraph (its docs: getting-started/installation); spec17 never runs these. The Settings tab shows the same commands.
const INSTALL_HINT: &str = "install CodeGraph: `curl -fsSL https://raw.githubusercontent.com/colbymchenry/codegraph/main/install.sh | sh` or `npm i -g @colbymchenry/codegraph` (Windows: `irm https://raw.githubusercontent.com/colbymchenry/codegraph/main/install.ps1 | iex`), then `codegraph init` in the repo; https://colbymchenry.github.io/codegraph/getting-started/installation/; spec17 never installs it";

/// Build or refresh the CodeGraph index by running the `codegraph` binary: `init` when there is no index yet, `sync` when there is.
/// Never called by `run` or on its own: indexing is the user's decision.
/// `refresh` (SETUP-08) is the rescan button: `sync` whenever there is an index and the binary, running codegraph or not (its watcher may be off); it answers `Ok` with the reason when it does nothing.
// (SETUP-07)
pub fn index(root: Option<&Path>, action: &str) -> Result<String, String> {
    let p = Project::open(root)?;
    let exists = p.root.join(&p.cfg.codegraph).exists();
    let action = if action == "refresh" {
        if !exists { return Ok("no CodeGraph index: nothing to refresh\n".into()); }
        if codegraph_version().is_none() { return Ok("the codegraph binary is not on PATH: index not refreshed\n".into()); }
        "sync"
    } else {
        action
    };
    let args: [&str; 2] = match (action, exists) {
        ("init", false) => ["init", "-y"],
        ("sync", true) => ["sync", ""],
        ("init", true) => return Err("an index already exists; use sync to refresh it".into()),
        ("sync", false) => return Err("no index yet; use init first".into()),
        _ => return Err(format!("unknown action '{action}': use init, sync or refresh")),
    };
    codegraph_version().ok_or("the codegraph binary is not on PATH: install CodeGraph first")?;
    let out = std::process::Command::new("codegraph").args(args.iter().filter(|a| !a.is_empty())).arg(&p.root).current_dir(&p.root).output().map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&[out.stdout, out.stderr].concat()).trim().to_string();
    let tail = text.lines().rev().take(12).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n");
    if out.status.success() { Ok(format!("codegraph {action} finished\n{tail}\n")) } else { Err(format!("codegraph {action} failed\n{tail}{}", if tail.contains("lock") { "\n(a running codegraph holds the lock only briefly: try again; `codegraph unlock` removes a stale one)" } else { "" })) }
}

/// Which index action the checks call for, if any: `init` when missing, `sync` when stale.
pub fn suggested(checks: &[Check]) -> Option<&'static str> {
    let k = checks.iter().find(|k| k.name == "codegraph index")?;
    if checks.iter().any(|b| b.name == "codegraph binary" && b.level != "ok") { return None; }
    k.action
}

// (SETUP-06) a skill in no skills folder is only information: MCP Skills clients load it from the server
fn agent_setup(st: &InstallStatus) -> Vec<Check> {
    let mut out: Vec<Check> = crate::skill::suggestions(st).into_iter().map(|t| c("agent setup", "warn", t, Some("spec17 install"))).collect();
    if st.project.skill == SkillState::Missing && st.global.skill == SkillState::Missing {
        out.push(c("agent setup", "info", "no skill in a skills folder: clients with MCP Skills load it from the spec17 MCP server (Claude Code: `/spec17:spec17`)", Some("`spec17 install` copies it for other clients")));
    } else if out.is_empty() {
        out.push(c("agent setup", "ok", "skill and MCP entry installed", None));
    }
    out
}

// (SETUP-05, SETUP-06)
pub fn run(root: Option<&Path>, mcp_port: u16) -> Vec<Check> {
    let mut out = vec![];
    let p = match Project::open(root) {
        Ok(p) => p,
        Err(e) => return vec![c("repo", "fail", e, Some("run inside a git repo, or pass --root"))],
    };
    out.push(c("repo", "ok", p.root.display().to_string(), None));
    let cfg_file = p.root.join("spec17.json");
    out.push(c("config", "ok", if cfg_file.exists() { "spec17.json".to_string() } else { "no spec17.json: layout detected from the repo".to_string() }, None));

    match Analysis::load(Some(&p.root)) {
        Ok(a) => {
            let r = a.report();
            // (SETUP-06) no specs is a warning: everything else still works
            let (level, hint) = if r.requirements + r.refs == 0 { ("warn", Some("no requirements found: write lines like `- **ABC-12** MUST — text` or `- ABC-12: the system MUST …` in specs/01-name.md (folders specs/, spec/, docs/specs/, openspec/specs/) or SPEC.md, or link an ID to a doc (a markdown link whose text is the ID, e.g. to decisions/0003-x.md); set \"specs\" in spec17.json to point at them")) } else { ("ok", None) };
            let refs = if r.refs > 0 { format!(", {} reference docs", r.refs) } else { String::new() };
            let detail = if a.p.spec_files().is_empty() { "no specs found".to_string() } else { format!("{} requirements in {} spec files{refs}; scanned {}", r.requirements, r.spec_files, a.p.spec_paths().join(", ")) };
            out.push(c("specs", level, detail, hint));
            if !r.spec_issues.is_empty() {
                out.push(c("spec issues", "warn", format!("{} (first: {})", r.spec_issues.len(), r.spec_issues[0]), Some("spec17 status --issues")));
            }
        }
        Err(e) => out.push(c("specs", "fail", e, None)),
    }

    for (name, dirs, files) in [("tests", &p.cfg.tests, p.test_files()), ("sources", &p.cfg.src, p.src_files())] {
        let missing: Vec<&String> = dirs.iter().filter(|d| !p.root.join(d).is_dir()).collect();
        let n = files.len();
        let list = if dirs.is_empty() { "the repo".to_string() } else { dirs.join(", ") };
        if !missing.is_empty() {
            out.push(c(name, "warn", format!("not found: {}", missing.iter().map(|d| d.as_str()).collect::<Vec<_>>().join(", ")), Some("set the folders in spec17.json (\"tests\", \"src\", \"ext\")")));
        } else if n == 0 {
            out.push(c(name, "warn", if p.cfg.ext.is_empty() { format!("no source files of a known language in {list}") } else { format!("no .{} files in {list}", p.cfg.ext.join("/.")) }, Some("check \"ext\" in spec17.json")));
        } else {
            out.push(c(name, "ok", format!("{n} files in {list}"), None));
        }
    }

    let git = std::process::Command::new("git").args(["rev-parse", "--is-inside-work-tree"]).current_dir(&p.root).output().is_ok_and(|o| o.status.success());
    out.push(if git { c("git", "ok", "work tree", None) } else { c("git", "warn", "not a git work tree", Some("`status --changed` needs git")) });

    // CodeGraph: optional, adds test-to-code links, overlap and uncovered code
    let bin = codegraph_version();
    out.push(match &bin {
        Some(v) => c("codegraph binary", "ok", if v.is_empty() { "on PATH".to_string() } else { v.clone() }, None),
        None => c("codegraph binary", "warn", "not on PATH", Some(INSTALL_HINT)), // (SETUP-09) optional: spec17 works without it
    });
    let db = p.root.join(&p.cfg.codegraph);
    out.push(match Graph::open(&db) {
        Ok(None) => {
            let hint = if bin.is_some() { "run `codegraph init` in the repo if you want it (indexing is your call: spec17 does it only when you ask, with `doctor --fix` or the dashboard's Settings)" } else { "needs the codegraph binary first" };
            Check { action: Some("init"), ..c("codegraph index", "warn", format!("{} not found: not initialized", p.cfg.codegraph), Some(hint)) }
        }
        Ok(Some(_)) => {
            let newest = p.src_files().into_iter().chain(p.test_files()).filter_map(|f| mtime(&p.root.join(f))).max();
            // (SETUP-09) the index's last write against the newest file, whatever runs beside it: a codegraph MCP server can have its watcher off
            match (mtime(&db), newest) {
                (Some(d), Some(n)) if n > d => Check { action: Some("sync"), ..c("codegraph index", "warn", "initialized, but older than the newest source or test file", Some("run `codegraph sync` (the rescan button does)")) },
                _ => c("codegraph index", "ok", format!("{} readable, schema supported, no source or test file is newer", p.cfg.codegraph), None),
            }
        }
        Err(GraphError::SchemaDrift(m)) => c("codegraph index", "warn", format!("schema not supported by this spec17: {m}"), Some("update spec17 or CodeGraph; those checks show as \"not checked\"")),
        Err(e) => c("codegraph index", "warn", e.to_string(), Some("the index may be corrupt: `codegraph init` again")),
    });

    match crate::skill::status(Some(&p.root), mcp_port) {
        Ok(st) => {
            out.extend(agent_setup(&st));
            // (SETUP-10) CodeGraph is on this machine but the agent is not told about it
            if bin.is_some() && st.project.codegraph_mcp != McpState::Ok && st.global.codegraph_mcp != McpState::Ok {
                out.push(c("agent setup", "warn", "no codegraph server in .mcp.json: the agent cannot use CodeGraph", Some("spec17 install --codegraph")));
            }
        }
        Err(e) => out.push(c("agent setup", "warn", e, None)),
    }
    out
}

pub fn text(checks: &[Check]) -> String {
    let mut s = String::from("spec17 doctor\n");
    for k in checks {
        let tag = match k.level {
            "ok" => "ok  ",
            "info" => "info",
            "warn" => "warn",
            _ => "FAIL",
        };
        s += &format!("  {tag}  {:<17} {}\n", k.name, k.detail);
        if let Some(h) = &k.hint {
            s += &format!("        {:<17} -> {h}\n", "");
        }
    }
    let (w, f) = (checks.iter().filter(|k| k.level == "warn").count(), checks.iter().filter(|k| k.level == "fail").count());
    s + &format!("{}\n", if f > 0 { format!("{f} failed, {w} warnings") } else if w > 0 { format!("works, {w} warnings") } else { "all good".to_string() })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn level<'a>(cs: &'a [Check], name: &str) -> &'a str {
        cs.iter().find(|k| k.name == name).unwrap_or_else(|| panic!("no check {name}")).level
    }

    #[doc = "[SETUP-05]"]
    #[test]
    fn doctor_reports_each_part_of_the_setup() {
        let d = std::env::temp_dir().join(format!("spec17-doctor-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("specs")).unwrap();
        std::fs::create_dir_all(d.join("RunSolo/RunSolo")).unwrap(); // sources exist, tests do not
        std::fs::write(d.join("specs/01-a.md"), "- **AAA-01** MUST — one.\n").unwrap();
        std::fs::write(d.join("RunSolo/RunSolo/A.swift"), "struct A {}\n").unwrap();
        let cs = run(Some(&d), 7411);
        assert_eq!(level(&cs, "repo"), "ok");
        assert_eq!(level(&cs, "specs"), "ok");
        assert_eq!(level(&cs, "sources"), "ok");
        assert_eq!(level(&cs, "tests"), "warn");
        assert_eq!(level(&cs, "codegraph index"), "warn"); // no .codegraph here
        assert!(cs.iter().any(|k| k.name == "codegraph binary"));
        let t = text(&cs);
        assert!(t.contains("not initialized") && t.contains("warnings"), "{t}");
    }

    #[doc = "[SETUP-07]"]
    #[test]
    fn indexing_runs_only_on_request_and_checks_state_first() {
        let d = std::env::temp_dir().join(format!("spec17-doctor-idx-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("specs")).unwrap();
        assert!(index(Some(&d), "sync").unwrap_err().contains("use init first"));
        assert!(index(Some(&d), "bogus").unwrap_err().contains("unknown action"));
        std::fs::create_dir_all(d.join(".codegraph")).unwrap();
        std::fs::write(d.join(".codegraph/codegraph.db"), "").unwrap();
        assert!(index(Some(&d), "init").unwrap_err().contains("already exists"));
        assert!(!run(Some(&d), 7411).is_empty() && d.join(".codegraph/codegraph.db").metadata().unwrap().len() == 0, "run() must not touch the index");
    }

    #[doc = "[SETUP-06]"]
    #[test]
    fn no_specs_is_a_warning_and_every_other_check_still_runs() {
        let d = std::env::temp_dir().join(format!("spec17-doctor-bare-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("src")).unwrap();
        std::fs::write(d.join("src/a.py"), "def a():\n    pass\n").unwrap();
        let cs = run(Some(&d), 7411);
        assert_eq!((level(&cs, "repo"), level(&cs, "specs"), level(&cs, "sources")), ("ok", "warn", "ok"));
        assert!(cs.iter().all(|k| k.level != "fail"), "{}", text(&cs));
        assert!(cs.iter().any(|k| k.name == "codegraph index"));
        let t = text(&cs);
        assert!(t.contains("no specs found") && t.contains("works,"), "{t}");
    }

    #[doc = "[SETUP-06]"]
    #[test]
    fn a_skill_in_no_folder_is_only_info() {
        let mut st = crate::skill::status(Some(&std::env::temp_dir()), 7411).unwrap();
        (st.project.skill, st.global.skill, st.project.mcp) = (SkillState::Missing, SkillState::Missing, McpState::Ok); // whatever this machine has
        assert!(crate::skill::suggestions(&st).is_empty());
        let cs = agent_setup(&st);
        assert_eq!(cs.iter().map(|k| k.level).collect::<Vec<_>>(), ["info"]);
        let t = text(&cs);
        assert!(t.contains("  info  agent setup") && t.contains("/spec17:spec17") && t.ends_with("all good\n"), "{t}");
        st.global.skill = SkillState::Outdated; // a stale copy is still a warning
        assert_eq!(agent_setup(&st).iter().map(|k| k.level).collect::<Vec<_>>(), ["warn"]);
    }

    #[doc = "[SETUP-08, SETUP-09]"]
    #[test]
    fn refresh_syncs_an_index_that_is_behind_and_the_check_sees_it() {
        let d = std::env::temp_dir().join(format!("spec17-doctor-refresh-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("src")).unwrap();
        std::fs::write(d.join("src/a.py"), "def a():\n    pass\n").unwrap();
        assert!(index(Some(&d), "refresh").unwrap().contains("nothing to refresh")); // no index: no error, no sync
        if codegraph_version().is_none() { return } // the rest needs the real binary
        index(Some(&d), "init").unwrap();
        let state = || run(Some(&d), 7411).into_iter().find(|k| k.name == "codegraph index").unwrap();
        assert_eq!(state().level, "ok");
        std::thread::sleep(std::time::Duration::from_millis(100));
        std::fs::write(d.join("src/a.py"), "def a():\n    pass\n\n\ndef b():\n    pass\n").unwrap();
        let behind = state();
        assert_eq!((behind.level, behind.action), ("warn", Some("sync")), "{}", behind.detail);
        assert!(index(Some(&d), "refresh").unwrap().contains("sync finished"));
        assert_eq!(state().level, "ok");
    }

    #[doc = "[SETUP-09]"]
    #[test]
    fn missing_codegraph_comes_with_install_instructions() {
        assert!(INSTALL_HINT.contains("install.sh") && INSTALL_HINT.contains("npm i -g @colbymchenry/codegraph") && INSTALL_HINT.contains("never installs"));
        let d = std::env::temp_dir().join(format!("spec17-doctor-hint-{}", std::process::id()));
        std::fs::create_dir_all(d.join("specs")).unwrap();
        let k = run(Some(&d), 7411).into_iter().find(|k| k.name == "codegraph binary").unwrap();
        assert!(k.level == "ok" || k.hint.as_deref() == Some(INSTALL_HINT)); // the binary may or may not be on this machine
    }
}

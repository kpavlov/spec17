//! `spec17 install` / `uninstall`: the skill ships inside the binary and is written to `<base>/skills/spec17/SKILL.md`.
use serde::Serialize;
use serde_json::{Map, Value, json};
use std::path::{Path, PathBuf};

const NAME: &str = "spec17";
const STDIO_NAME: &str = "spec17-stdio";
const STDIO_FLAGS: [&str; 3] = ["--stdio-mcp", "--no-http-mcp", "--no-ui"];
const CODEGRAPH: &str = "codegraph";
/// Every file of the skill, by its path inside the skill folder; `SKILL.md` first (SETUP-12).
pub const FILES: [(&str, &str); 4] = [
    ("SKILL.md", include_str!("../../skills/spec17/SKILL.md")),
    ("references/tools.md", include_str!("../../skills/spec17/references/tools.md")),
    ("references/conflicts.md", include_str!("../../skills/spec17/references/conflicts.md")),
    ("references/config.md", include_str!("../../skills/spec17/references/config.md")),
];
pub const SKILL: &str = FILES[0].1;

/// `<repo>/.agents/skills` or `~/.agents/skills`.
pub fn skills_dir(root: Option<&Path>, global: bool) -> Result<PathBuf, String> {
    if global {
        return Ok(home()?.join(".agents/skills"));
    }
    let base = repo_root(root)?;
    Ok(base.join(".agents/skills"))
}

pub fn install(dir: &Path) -> Result<PathBuf, String> {
    let d = dir.join(NAME);
    for (path, text) in FILES {
        let f = d.join(path);
        let parent = f.parent().unwrap_or(&d);
        std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        std::fs::write(&f, text).map_err(|e| format!("{}: {e}", f.display()))?;
    }
    Ok(d.join("SKILL.md"))
}

/// Removes only the files we wrote, and the folders that leaves empty: anything else the user put there stays.
pub fn uninstall(dir: &Path) -> Result<Option<PathBuf>, String> {
    let d = dir.join(NAME);
    let f = d.join("SKILL.md");
    let mut removed = false;
    for (path, _) in FILES {
        let p = d.join(path);
        match std::fs::remove_file(&p) {
            Ok(()) => removed = true,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(format!("{}: {e}", p.display())),
        }
    }
    if !removed {
        return Ok(None);
    }
    let _ = std::fs::remove_dir(d.join("references")); // these fail (and are kept) when they still hold other files
    let _ = std::fs::remove_dir(&d);
    Ok(Some(f))
}

pub enum McpChange {
    Created,
    Added,
    Updated,
    Unchanged,
    Removed,
    Absent,
}

fn read_config(f: &Path) -> Result<Option<Map<String, Value>>, String> {
    match std::fs::read_to_string(f) {
        Ok(t) if t.trim().is_empty() => Ok(Some(Map::new())),
        Ok(t) => match serde_json::from_str::<Value>(&t) {
            Ok(Value::Object(m)) => Ok(Some(m)),
            Ok(_) => Err(format!("{}: not a JSON object, left untouched", f.display())),
            Err(e) => Err(format!("{}: {e}; left untouched", f.display())),
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("{}: {e}", f.display())),
    }
}

fn write_config(f: &Path, m: Map<String, Value>) -> Result<(), String> {
    let text = serde_json::to_string_pretty(&Value::Object(m)).map_err(|e| e.to_string())? + "\n";
    std::fs::write(f, text).map_err(|e| format!("{}: {e}", f.display()))
}

// (SETUP-01, SETUP-02)
/// Adds `mcpServers.spec17` to `<base>/.mcp.json`, creating the file if needed. Other servers and keys are kept as they are.
pub fn add_mcp(base: &Path, port: u16) -> Result<(PathBuf, McpChange), String> {
    add_server(base, NAME, json!({"type": "http", "url": format!("http://127.0.0.1:{port}/mcp")}), true)
}

// (SETUP-13)
fn add_stdio_mcp(base: &Path, global: bool) -> Result<(PathBuf, McpChange), String> {
    let command = std::env::current_exe().map_err(|e| format!("Locate the spec17 executable: {e}"))?;
    let mut args = STDIO_FLAGS.map(Value::from).to_vec();
    if !global {
        args.extend([json!("--root"), serde_json::to_value(base).map_err(|e| e.to_string())?]);
    }
    let command = serde_json::to_value(&command).map_err(|e| e.to_string())?;
    add_server(base, STDIO_NAME, json!({"type": "stdio", "command": command, "args": args}), true)
}

/// Adds the `codegraph` entry (SETUP-10), as `codegraph install --print-config claude` prints it. One that is already there is the user's: never replaced.
pub fn add_codegraph(base: &Path) -> Result<(PathBuf, McpChange), String> {
    add_server(base, CODEGRAPH, json!({"type": "stdio", "command": "codegraph", "args": ["serve", "--mcp"], "alwaysLoad": true}), false)
}

fn add_server(base: &Path, name: &str, entry: Value, replace: bool) -> Result<(PathBuf, McpChange), String> {
    let f = base.join(".mcp.json");
    let existing = read_config(&f)?;
    let created = existing.is_none();
    let mut m = existing.unwrap_or_default();
    let servers = m.entry("mcpServers").or_insert_with(|| json!({}));
    let servers = servers.as_object_mut().ok_or_else(|| format!("{}: mcpServers is not an object, left untouched", f.display()))?;
    let change = match servers.get(name) {
        Some(v) if *v == entry || !replace => McpChange::Unchanged,
        Some(_) => McpChange::Updated,
        None if created => McpChange::Created,
        None => McpChange::Added,
    };
    if !matches!(change, McpChange::Unchanged) {
        servers.insert(name.into(), entry);
        write_config(&f, m)?;
    }
    Ok((f, change))
}

// (SETUP-03)
/// Removes `mcpServers.spec17` and keeps the config file even if there are no servers left.
pub fn remove_mcp(base: &Path) -> Result<(PathBuf, McpChange), String> {
    remove_server(base, NAME)
}

/// Removes the `codegraph` entry (SETUP-10), whatever it holds: the user asked. The rest follows `remove_mcp`.
pub fn remove_codegraph(base: &Path) -> Result<(PathBuf, McpChange), String> {
    remove_server(base, CODEGRAPH)
}

fn remove_server(base: &Path, name: &str) -> Result<(PathBuf, McpChange), String> {
    let f = base.join(".mcp.json");
    let Some(mut m) = read_config(&f)? else { return Ok((f, McpChange::Absent)) };
    let Some(Value::Object(servers)) = m.get_mut("mcpServers") else { return Ok((f, McpChange::Absent)) };
    if servers.remove(name).is_none() {
        return Ok((f, McpChange::Absent));
    }
    write_config(&f, m)?;
    Ok((f, McpChange::Removed))
}

/// The repo root for project installs (`--root`, else the nearest ancestor with specs or a git work tree; else the cwd).
pub fn repo_root(root: Option<&Path>) -> Result<PathBuf, String> {
    spec17_core::scan::Project::open(root).map(|p| p.root).or_else(|_| root.map(Path::to_path_buf).map_or_else(|| std::env::current_dir().map_err(|e| e.to_string()), Ok))
}

fn home() -> Result<PathBuf, String> {
    std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from).ok_or_else(|| "HOME is not set".into())
}

#[derive(Clone, Copy, PartialEq, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum SkillState {
    Current,
    Outdated,
    Missing,
}

/// `Ok`: an http entry for this port; `Other`: a spec17 entry that points elsewhere. For the `codegraph` entry only `Ok` and `Missing` apply.
#[derive(Clone, Copy, PartialEq, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum McpState {
    Ok,
    Other,
    Missing,
}

#[derive(Serialize, schemars::JsonSchema)]
pub struct ProjectInstall {
    pub root: PathBuf,
    pub skill: SkillState,
    pub skill_path: PathBuf,
    pub mcp: McpState,
    pub mcp_stdio: McpState,
    pub mcp_details: McpDetails,
    pub mcp_path: PathBuf,
    /// Whether `.mcp.json` has a `codegraph` server (SETUP-10).
    pub codegraph_mcp: McpState,
}

#[derive(Serialize, schemars::JsonSchema)]
pub struct GlobalInstall {
    pub skill: SkillState,
    pub skill_path: PathBuf,
    pub mcp: McpState,
    pub mcp_file: McpState,
    pub mcp_stdio: McpState,
    pub mcp_details: McpDetails,
    pub mcp_path: PathBuf,
    pub codegraph_mcp: McpState,
}

#[derive(Serialize, schemars::JsonSchema)]
pub struct InstallStatus {
    pub project: ProjectInstall,
    pub global: GlobalInstall,
}

#[derive(Serialize, schemars::JsonSchema)]
pub struct McpDetails {
    http_url: Option<String>,
    stdio: Option<StdioLaunch>,
}

#[derive(Serialize, serde::Deserialize, schemars::JsonSchema)]
struct StdioLaunch {
    command: String,
    args: Vec<String>,
}

fn skill_state(dir: &Path) -> SkillState {
    let d = dir.join(NAME);
    if !d.join("SKILL.md").exists() {
        return SkillState::Missing;
    }
    match FILES.iter().all(|(path, text)| std::fs::read_to_string(d.join(path)).is_ok_and(|t| t == *text)) {
        true => SkillState::Current,
        false => SkillState::Outdated,
    }
}

fn mcp_state(entry: Option<&Value>, port: u16) -> McpState {
    server_state(entry, |e| e["url"] == format!("http://127.0.0.1:{port}/mcp"))
}

fn stdio_state(entry: Option<&Value>) -> McpState {
    server_state(entry, |e| e["type"] == "stdio" && e["command"].as_str().is_some_and(|s| !s.is_empty()))
}

fn server_state(entry: Option<&Value>, matches: impl FnOnce(&Value) -> bool) -> McpState {
    match entry {
        None => McpState::Missing,
        Some(e) if matches(e) => McpState::Ok,
        Some(_) => McpState::Other,
    }
}

fn configured_servers(config: &Path) -> Map<String, Value> {
    match read_config(config).ok().flatten().and_then(|mut m| m.remove("mcpServers")) {
        Some(Value::Object(servers)) => servers,
        _ => Map::new(),
    }
}

// (SETUP-14)
fn mcp_details(servers: &Map<String, Value>, root: Option<&Path>) -> McpDetails {
    let http_url = servers.get(NAME).and_then(|entry| entry["url"].as_str()).filter(|url| {
        url.strip_prefix("http://127.0.0.1:").and_then(|s| s.strip_suffix("/mcp")).is_some_and(|port| port.parse::<u16>().is_ok())
    }).map(str::to_owned);
    let stdio = servers.get(STDIO_NAME).filter(|entry| entry["type"] == "stdio")
        .and_then(|entry| serde::Deserialize::deserialize(entry).ok()).filter(|launch: &StdioLaunch| {
            let mut args = launch.args.iter().map(String::as_str);
            if launch.command.is_empty() || !args.by_ref().take(STDIO_FLAGS.len()).eq(STDIO_FLAGS) {
                return false;
            }
            match root {
                Some(root) => args.next() == Some("--root") && args.next().map(Path::new) == Some(root) && args.next().is_none(),
                None => args.next().is_none(),
            }
        });
    McpDetails { http_url, stdio }
}

// (SETUP-04, SETUP-12) Read the user-scope entry in ~/.claude.json separately from ~/.mcp.json.
pub fn status(root: Option<&Path>, port: u16) -> Result<InstallStatus, String> {
    let base = repo_root(root)?;
    let home = home()?;
    let pdir = base.join(".agents/skills");
    let gdir = home.join(".agents/skills");
    let (pmcp, gmcp, claude) = (base.join(".mcp.json"), home.join(".mcp.json"), home.join(".claude.json"));
    let (project, global, user) = (configured_servers(&pmcp), configured_servers(&gmcp), configured_servers(&claude));
    Ok(InstallStatus {
        project: ProjectInstall {
            skill: skill_state(&pdir), skill_path: pdir.join(NAME).join("SKILL.md"),
            mcp: mcp_state(project.get(NAME), port), mcp_stdio: stdio_state(project.get(STDIO_NAME)),
            mcp_details: mcp_details(&project, Some(&base)),
            codegraph_mcp: server_state(project.get(CODEGRAPH), |_| true), mcp_path: pmcp, root: base,
        },
        global: GlobalInstall {
            skill: skill_state(&gdir), skill_path: gdir.join(NAME).join("SKILL.md"),
            mcp: mcp_state(user.get(NAME), port), mcp_file: mcp_state(global.get(NAME), port),
            mcp_stdio: stdio_state(global.get(STDIO_NAME)), mcp_details: mcp_details(&global, None),
            mcp_path: gmcp, codegraph_mcp: server_state(user.get(CODEGRAPH), |_| true),
        },
    })
}

/// One line per thing a person should do after `status`; empty when the agent can already use spec17 here.
pub fn suggestions(st: &InstallStatus) -> Vec<String> {
    use {McpState as M, SkillState as S};
    let (p, g) = (&st.project, &st.global);
    let mut out = vec![];
    if p.skill == S::Missing && g.skill == S::Missing {
        out.push("skill not installed: `spec17 install` (this repo) or `spec17 install --global`".to_string());
    } else if p.skill == S::Outdated || (p.skill == S::Missing && g.skill == S::Outdated) {
        out.push("installed skill is out of date: run `spec17 install`".to_string());
    }
    if ![p.mcp, p.mcp_stdio, g.mcp, g.mcp_file, g.mcp_stdio].contains(&M::Ok) {
        out.push(if p.mcp == M::Other { "the spec17 entry in .mcp.json differs from this server: `spec17 install --port N` updates it" } else { "MCP entry missing: `spec17 install`" }.to_string());
    }
    out
}

/// `install` (`add`) or `uninstall` with `server=codegraph` (SETUP-10): only the CodeGraph entry of the repo's `.mcp.json`; the skill and the spec17 entry are left alone.
pub fn apply_codegraph(root: Option<&Path>, add: bool) -> Result<Vec<String>, String> {
    let base = repo_root(root)?;
    let (f, change) = if add { add_codegraph(&base)? } else { remove_codegraph(&base)? };
    let what = match change {
        McpChange::Created => "created with codegraph",
        McpChange::Added => "added codegraph",
        McpChange::Removed => "removed codegraph",
        McpChange::Absent => "no codegraph entry",
        _ => "codegraph already set, kept as it is",
    };
    Ok(vec![format!("mcp    {} ({what})", f.display())])
}

#[derive(Clone, Copy, PartialEq, clap::ValueEnum, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InstallPart {
    Skill,
    Mcp,
}

#[derive(Clone, Copy, Default, PartialEq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum McpTransport {
    #[default]
    Http,
    Stdio,
}

/// Install (`port` = Some) or uninstall; returns one line per change, as the CLI prints them.
// (SETUP-12, SETUP-13)
pub fn apply(root: Option<&Path>, global: bool, port: Option<u16>, only: Option<InstallPart>, transport: McpTransport) -> Result<Vec<String>, String> {
    use McpChange::*;
    let mut out = vec![];
    if only != Some(InstallPart::Mcp) {
        let dir = skills_dir(root, global)?;
        match port {
            Some(_) => out.push(format!("skill  {}", install(&dir)?.display())),
            None => out.push(match uninstall(&dir)? {
                Some(f) => format!("skill  removed {}", f.display()),
                None => format!("skill  not installed in {}", dir.display()),
            }),
        }
    }
    if only == Some(InstallPart::Skill) {
        return Ok(out);
    }
    if global && only.is_none() && port.is_none() {
        out.push("mcp    use `spec17 uninstall --global --only mcp` to remove the HTTP entry from ~/.mcp.json".into());
        return Ok(out);
    }
    let base = if global { home()? } else { repo_root(root)? };
    let (entry, (f, change)) = match (transport, port) {
        (McpTransport::Http, Some(p)) => (NAME, add_mcp(&base, p)?),
        (McpTransport::Http, None) => (NAME, remove_mcp(&base)?),
        (McpTransport::Stdio, Some(_)) => (STDIO_NAME, add_stdio_mcp(&base, global)?),
        (McpTransport::Stdio, None) => (STDIO_NAME, remove_server(&base, STDIO_NAME)?),
    };
    let what = match change {
        Created => "created with",
        Added => "added",
        Updated => "updated",
        Unchanged => "already set",
        Removed => "removed",
        Absent => "no entry for",
    };
    out.push(format!("mcp    {} ({what} {entry})", f.display()));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[doc = "[API-17]"]
    #[test]
    fn the_skill_lists_every_tool() {
        let (_, tools) = FILES.iter().find(|(p, _)| *p == "references/tools.md").unwrap();
        for t in crate::ops::tools().iter() {
            assert!(tools.contains(&format!("| `{}` |", t.name)), "skills/spec17/references/tools.md does not list the `{}` tool", t.name);
        }
        for (path, _) in &FILES[1..] {
            assert!(SKILL.contains(&format!("`{path}`")), "SKILL.md does not point to {path}");
        }
    }

    fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("spec17-skill-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[doc = "[SETUP-01]"]
    #[test]
    fn embedded_skill_is_the_spec17_skill() {
        assert!(SKILL.starts_with("---\nname: spec17\n"));
        assert!(!SKILL.contains("python3") && !SKILL.contains(".py"), "no Python left in the skill");
    }

    #[doc = "[SETUP-01, SETUP-03]"]
    #[test]
    fn install_is_idempotent_and_uninstall_keeps_foreign_files() {
        let dir = tmp("a").join(".agents/skills");
        let f = install(&dir).unwrap();
        assert_eq!(std::fs::read_to_string(&f).unwrap(), SKILL);
        install(&dir).unwrap();
        let tools = dir.join("spec17/references/tools.md");
        assert_eq!(std::fs::read_to_string(&tools).unwrap(), FILES[1].1);
        std::fs::remove_file(&f).unwrap(); // Retry after the entrypoint was already removed.
        std::fs::write(dir.join("spec17/notes.txt"), "mine").unwrap();
        assert_eq!(uninstall(&dir).unwrap(), Some(f.clone()));
        assert!(!f.exists() && !tools.exists() && !dir.join("spec17/references").exists() && dir.join("spec17/notes.txt").exists());
        assert_eq!(uninstall(&dir).unwrap(), None);
    }

    #[doc = "[SETUP-03]"]
    #[test]
    fn uninstall_removes_the_empty_folder() {
        let dir = tmp("b");
        install(&dir).unwrap();
        uninstall(&dir).unwrap();
        assert!(!dir.join("spec17").exists());
    }

    #[doc = "[SETUP-01]"]
    #[test]
    fn mcp_config_is_merged_never_clobbered() {
        let d = tmp("m");
        let (f, c) = add_mcp(&d, 7411).unwrap();
        assert!(matches!(c, McpChange::Created));
        // another server and an unrelated key, in this order
        std::fs::write(&f, r#"{"zeta": 1, "mcpServers": {"codegraph": {"type": "stdio", "command": "codegraph"}}}"#).unwrap();
        assert!(matches!(add_mcp(&d, 7411).unwrap().1, McpChange::Added));
        let v: Value = serde_json::from_str(&std::fs::read_to_string(&f).unwrap()).unwrap();
        assert_eq!(v["mcpServers"]["codegraph"]["command"], "codegraph");
        assert_eq!(v["mcpServers"]["spec17"]["url"], "http://127.0.0.1:7411/mcp");
        assert_eq!(v.as_object().unwrap().keys().next().unwrap(), "zeta", "key order kept");
        assert!(matches!(add_mcp(&d, 7411).unwrap().1, McpChange::Unchanged));
        assert!(matches!(add_mcp(&d, 7500).unwrap().1, McpChange::Updated));
        assert!(matches!(remove_mcp(&d).unwrap().1, McpChange::Removed));
        let v: Value = serde_json::from_str(&std::fs::read_to_string(&f).unwrap()).unwrap();
        assert!(v["mcpServers"].get("spec17").is_none() && v["mcpServers"]["codegraph"].is_object() && v["zeta"] == 1);
        assert!(matches!(remove_mcp(&d).unwrap().1, McpChange::Absent));
    }

    #[doc = "[SETUP-02, SETUP-03]"]
    #[test]
    fn uninstall_keeps_an_empty_config_and_leaves_bad_json_untouched() {
        let d = tmp("n");
        let f = d.join(".mcp.json");
        std::fs::write(&f, "{}").unwrap();
        let (f, _) = add_mcp(&d, 7411).unwrap();
        assert!(matches!(remove_mcp(&d).unwrap().1, McpChange::Removed));
        let v: Value = serde_json::from_str(&std::fs::read_to_string(&f).unwrap()).unwrap();
        assert_eq!(v, json!({"mcpServers": {}}));
        std::fs::write(&f, "{ not json").unwrap();
        assert!(add_mcp(&d, 7411).is_err() && remove_mcp(&d).is_err());
        assert_eq!(std::fs::read_to_string(&f).unwrap(), "{ not json");
    }

    #[doc = "[SETUP-01, SETUP-03]"]
    #[test]
    fn status_and_suggestions_follow_install() {
        let repo = tmp("s");
        std::fs::create_dir_all(repo.join("specs")).unwrap();
        let st = status(Some(&repo), 7411).unwrap();
        assert!(st.project.skill == SkillState::Missing && st.project.mcp == McpState::Missing);
        assert!(suggestions(&st).len() >= 1 || st.global.skill != SkillState::Missing); // a real ~/.agents may already hold it
        apply(Some(&repo), false, Some(7411), None, McpTransport::Http).unwrap();
        let st = status(Some(&repo), 7411).unwrap();
        assert!(st.project.skill == SkillState::Current && st.project.mcp == McpState::Ok);
        assert!(suggestions(&st).is_empty(), "{:?}", suggestions(&st));
        assert!(status(Some(&repo), 7500).unwrap().project.mcp == McpState::Other);
        std::fs::write(repo.join(".agents/skills/spec17/SKILL.md"), "old").unwrap();
        assert!(status(Some(&repo), 7411).unwrap().project.skill == SkillState::Outdated);
        apply(Some(&repo), false, None, None, McpTransport::Http).unwrap();
        assert!(status(Some(&repo), 7411).unwrap().project.skill == SkillState::Missing);
    }

    #[doc = "[SETUP-14]"]
    #[test]
    fn reports_connection_details_from_installed_entries() {
        let dir = tmp("connection details");
        for global in [false, true] {
            let root = (!global).then_some(dir.as_path());
            let (path, _) = add_mcp(&dir, 7423).unwrap();
            add_stdio_mcp(&dir, global).unwrap();
            let servers = configured_servers(&path);
            let details = mcp_details(&servers, root);
            assert_eq!(details.http_url.as_deref(), Some("http://127.0.0.1:7423/mcp"));
            let stdio = details.stdio.unwrap();
            assert_eq!(Path::new(&stdio.command), std::env::current_exe().unwrap());
            assert_eq!(stdio.args, servers[STDIO_NAME]["args"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect::<Vec<_>>());
        }
        let custom = json!({
            "spec17": {"url": "http://127.0.0.1:7423/mcp?token=secret"},
            "spec17-stdio": {"type":"stdio", "command":"spec17", "args":["--token", "secret"], "env":{"TOKEN":"secret"}},
        });
        let details = mcp_details(custom.as_object().unwrap(), None);
        assert!(details.http_url.is_none() && details.stdio.is_none());
        let missing = mcp_details(&Map::new(), None);
        assert!(missing.http_url.is_none() && missing.stdio.is_none());
    }

    #[doc = "[SETUP-01]"]
    #[test]
    fn project_dir_is_the_explicit_root() {
        let root = tmp("c");
        std::fs::create_dir_all(root.join("specs")).unwrap();
        assert_eq!(skills_dir(Some(&root), false).unwrap(), root.canonicalize().unwrap().join(".agents/skills"));
        // no specs folder yet (SCAN-28): still installs where asked
        let bare = tmp("d");
        assert_eq!(skills_dir(Some(&bare), false).unwrap(), bare.canonicalize().unwrap().join(".agents/skills"));
    }

    #[doc = "[SETUP-10]"]
    #[test]
    fn the_codegraph_entry_is_added_beside_the_others_and_never_replaced() {
        let d = tmp("g");
        let f = d.join(".mcp.json");
        let (_, c) = add_codegraph(&d).unwrap();
        assert!(matches!(c, McpChange::Created));
        let v: Value = serde_json::from_str(&std::fs::read_to_string(&f).unwrap()).unwrap();
        assert_eq!(v["mcpServers"]["codegraph"], json!({"type": "stdio", "command": "codegraph", "args": ["serve", "--mcp"], "alwaysLoad": true}));
        assert!(status(Some(&d), 7411).unwrap().project.codegraph_mcp == McpState::Ok);
        // a customised entry and other servers stay as they are
        std::fs::write(&f, r#"{"mcpServers": {"codegraph": {"command": "mine", "env": {"A": "1"}}, "other": {"command": "x"}}}"#).unwrap();
        assert!(matches!(add_codegraph(&d).unwrap().1, McpChange::Unchanged));
        let v: Value = serde_json::from_str(&std::fs::read_to_string(&f).unwrap()).unwrap();
        assert_eq!((v["mcpServers"]["codegraph"]["command"].as_str(), v["mcpServers"]["other"]["command"].as_str()), (Some("mine"), Some("x")));
        // the spec17 entry is added by its own install, and does not touch codegraph
        add_mcp(&d, 7411).unwrap();
        assert!(status(Some(&d), 7411).unwrap().project.mcp == McpState::Ok);
        let v: Value = serde_json::from_str(&std::fs::read_to_string(&f).unwrap()).unwrap();
        assert_eq!(v["mcpServers"]["codegraph"]["command"], "mine");
        // invalid JSON is left alone (SETUP-02)
        std::fs::write(&f, "{ nope").unwrap();
        assert!(add_codegraph(&d).is_err() && std::fs::read_to_string(&f).unwrap() == "{ nope");
        assert!(status(Some(&d), 7411).unwrap().project.codegraph_mcp == McpState::Missing);
        assert!(remove_codegraph(&d).is_err() && std::fs::read_to_string(&f).unwrap() == "{ nope");
    }

    #[doc = "[SETUP-10]"]
    #[test]
    fn the_codegraph_entry_is_removed_alone_customised_or_not() {
        let d = tmp("h");
        let f = d.join(".mcp.json");
        assert!(matches!(remove_codegraph(&d).unwrap().1, McpChange::Absent)); // no file, nothing to do
        std::fs::write(&f, r#"{"mcpServers": {"codegraph": {"command": "mine", "env": {"A": "1"}}, "spec17": {"type": "http", "url": "http://127.0.0.1:7411/mcp"}, "other": {"command": "x"}}, "keep": 1}"#).unwrap();
        assert!(matches!(remove_codegraph(&d).unwrap().1, McpChange::Removed));
        let v: Value = serde_json::from_str(&std::fs::read_to_string(&f).unwrap()).unwrap();
        assert!(v["mcpServers"].get("codegraph").is_none() && v["mcpServers"]["other"]["command"] == "x" && v["keep"] == 1);
        assert!(status(Some(&d), 7411).unwrap().project.mcp == McpState::Ok && status(Some(&d), 7411).unwrap().project.codegraph_mcp == McpState::Missing);
        assert!(matches!(remove_codegraph(&d).unwrap().1, McpChange::Absent)); // twice is fine
        // Keep the config file when its last server is removed (SETUP-03).
        std::fs::remove_file(&f).unwrap();
        add_codegraph(&d).unwrap();
        assert!(matches!(remove_codegraph(&d).unwrap().1, McpChange::Removed));
        let v: Value = serde_json::from_str(&std::fs::read_to_string(&f).unwrap()).unwrap();
        assert_eq!(v, json!({"mcpServers": {}}));
    }
}

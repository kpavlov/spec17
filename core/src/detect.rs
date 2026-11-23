//! Layout detection: where the specs, sources and tests of a repo are, when `spec17.json` does not say.
//! Specs come from the folder conventions of spec-driven tools; sources and tests from the file list of the CodeGraph
//! index (else a directory walk) classified by ecosystem. Symlinks are followed only into the repo root or `ALLOWED_PATHS`.
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use walkdir::{DirEntry, WalkDir};

/// Spec folders in priority order: dedicated names first, generic doc folders last (they need a requirement bullet to count).
/// Covers plain `specs/`, GitHub Spec Kit (`.specify/specs`), OpenSpec, Kiro and the usual docs layouts.
const SPEC_DIRS: [&str; 13] = [
    "specs", "spec", "specifications", "requirements", "docs/specs", "docs/spec", "docs/requirements", "doc/specs",
    "openspec/specs", ".specify/specs", ".kiro/specs", "docs", "doc",
];
const GENERIC: [&str; 2] = ["docs", "doc"];
/// File stems (upper case) of a single-file spec, at the root or in `docs/`.
const SPEC_STEMS: [&str; 6] = ["SPEC", "SPECS", "SPECIFICATION", "SPECIFICATIONS", "REQUIREMENTS", "PRD"];
/// Architecture decision records: reference docs, not bullets.
const DECISION_DIRS: [&str; 9] = ["decisions", "adr", "adrs", "docs/adr", "docs/adrs", "docs/decisions", "doc/adr", "doc/decisions", "architecture/decisions"];
/// Folders that never hold the project's own code.
const SKIP: [&str; 12] = ["node_modules", "target", "build", "dist", "vendor", "venv", "site-packages", "__pycache__", "Pods", "DerivedData", "obj", "coverage"];
/// Matches nothing: the regex of an ecosystem spec17 knows nothing about.
pub const NEVER: &str = r"[^\s\S]";

/// One ecosystem: how its files, comments, declarations and tests look. Loaded from `resources/profiles.toml`; a `profiles`
/// entry of `spec17.json` has the same shape (see `profiles`).
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub name: String,
    pub ext: Vec<String>,
    pub comment: Vec<String>,
    #[serde(deserialize_with = "patterns")]
    pub decl: Vec<String>,
    #[serde(deserialize_with = "patterns")]
    pub assert: Vec<String>,
    #[serde(deserialize_with = "patterns")]
    pub off: Vec<String>,
    /// Repo-relative path of a test file, also when it sits next to the code (`a_test.go`, `a.test.ts`).
    #[serde(deserialize_with = "patterns")]
    pub test: Vec<String>,
    /// Tests live in the source files too (Rust `mod tests`).
    #[serde(default)]
    pub inline: bool,
    /// A line that opens a test case (an attribute, annotation, call or declaration).
    #[serde(deserialize_with = "patterns")]
    pub case: Vec<String>,
    /// `case` matches a call such as `it("name", ...)` (the name is its quoted string) rather than a marker above a declaration.
    #[serde(default)]
    pub call: bool,
}

impl Profile {
    /// An ecosystem spec17 knows nothing about: no files, no comments, regexes that match nothing.
    pub fn unknown(name: &str) -> Profile {
        let n = || vec![NEVER.to_string()];
        Profile { name: name.into(), ext: vec![], comment: vec![], decl: n(), assert: n(), off: n(), test: n(), inline: false, case: n(), call: false }
    }
}

fn patterns<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<String>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum OneOrMany { One(String), Many(Vec<String>) }
    Ok(match OneOrMany::deserialize(d)? { OneOrMany::One(s) => vec![s], OneOrMany::Many(v) => v })
}

#[derive(Deserialize)]
struct Profiles {
    profile: Vec<Profile>,
}

/// To support a new language, add a `[[profile]]` to `resources/profiles.toml`.
pub static PROFILES: LazyLock<Vec<Profile>> = LazyLock::new(|| {
    toml::from_str::<Profiles>(include_str!("../resources/profiles.toml")).expect("resources/profiles.toml").profile
});

// (SCAN-34)
/// The built-in profiles with the `profiles` of `spec17.json`: an entry named like a built-in one replaces the fields it sets, any
/// other is a new ecosystem, ahead of the built-ins so it wins an extension they share; the regexes it leaves out match nothing.
pub fn profiles(extra: &[serde_json::Map<String, serde_json::Value>]) -> Result<Vec<Profile>, String> {
    let mut all = PROFILES.clone();
    for e in extra {
        let name = e.get("name").and_then(|n| n.as_str()).ok_or("spec17.json: a profile needs a `name`")?;
        let at = all.iter().position(|p| p.name == name);
        let base = at.map_or_else(|| Profile::unknown(name), |i| all[i].clone());
        let serde_json::Value::Object(mut v) = serde_json::to_value(base).map_err(|e| e.to_string())? else { unreachable!() };
        v.extend(e.clone());
        let p: Profile = serde_json::from_value(v.into()).map_err(|e| format!("spec17.json: profile {name}: {e}"))?;
        match at {
            Some(i) => all[i] = p,
            None => all.insert(0, p),
        }
    }
    Ok(all)
}

/// How test cases are found in one file: its ecosystem, and whether the line that opens one is a call.
pub struct Case<'a> {
    pub profile: &'a Profile,
    pub call: bool,
}

/// The test-case rule of a file's ecosystem, by extension.
pub fn case_of<'a>(profiles: &'a [Profile], file: &str) -> Option<Case<'a>> {
    let p = &profiles[profile_of(profiles, file)?];
    Some(Case { profile: p, call: p.call })
}

/// The index in `profiles` of a file's ecosystem, by extension.
pub fn profile_of(profiles: &[Profile], file: &str) -> Option<usize> {
    profiles.iter().position(|p| p.ext.iter().any(|e| e == ext_of(file)))
}

/// Build manifests that make a folder a module.
const MANIFESTS: [&str; 12] = ["Cargo.toml", "package.json", "build.gradle.kts", "build.gradle", "pom.xml", "go.mod", "pyproject.toml", "setup.py", "Package.swift", "composer.json", "CMakeLists.txt", "Gemfile"];

// (GRAPH-05)
/// The module of a repo-relative file and its build manifest: the nearest folder above it that has one, else (also when only
/// the repo root has one) the folder above its `src`, else its top-level folder, else `.`. `seen` caches per folder.
pub fn module_of(root: &Path, file: &str, seen: &mut std::collections::HashMap<String, (String, Option<&'static str>)>) -> (String, Option<&'static str>) {
    let dir = file.rsplit_once('/').map_or("", |(d, _)| d);
    if let Some(v) = seen.get(dir) {
        return v.clone();
    }
    let manifest = |d: &str| MANIFESTS.iter().find(|m| root.join(d).join(m).is_file()).copied();
    let mut d = dir;
    let nested = loop {
        if d.is_empty() {
            break None;
        }
        if let Some(m) = manifest(d) {
            break Some((d.to_string(), Some(m)));
        }
        d = d.rsplit_once('/').map_or("", |(up, _)| up);
    };
    let found = nested.unwrap_or_else(|| {
        let comps: Vec<&str> = dir.split('/').filter(|c| !c.is_empty()).collect();
        let by_src = comps.iter().position(|c| matches!(*c, "src" | "Sources" | "Source" | "lib" | "app")).filter(|i| *i > 0).map(|i| comps[..i].join("/"));
        let m = by_src.or_else(|| comps.first().map(|c| c.to_string())).unwrap_or_else(|| ".".into());
        let own = manifest(if m == "." { "" } else { &m });
        (m, own)
    });
    seen.insert(dir.to_string(), found.clone());
    found
}

/// Canonical roots a symlink may point into: the repo root, plus every entry of `ALLOWED_PATHS` (path-list syntax).
pub fn allow_list(root: &Path) -> Vec<PathBuf> {
    let extra = std::env::var_os("ALLOWED_PATHS").map(|v| std::env::split_paths(&v).collect::<Vec<_>>()).unwrap_or_default();
    std::iter::once(root.to_path_buf()).chain(extra).filter_map(|p| p.canonicalize().ok()).collect()
}

fn inside(p: &Path, allow: &[PathBuf]) -> bool {
    p.canonicalize().is_ok_and(|c| allow.iter().any(|a| c.starts_with(a)))
}

/// Whether a path from `spec17.json` stays in the repo (or `ALLOWED_PATHS`). One that does not exist yet only has to be a plain
/// relative path, so `doctor` can still say "not found".
pub fn confined(root: &Path, rel: &str, allow: &[PathBuf]) -> bool {
    let at = root.join(rel);
    if at.exists() {
        return inside(&at, allow);
    }
    Path::new(rel).components().all(|c| matches!(c, std::path::Component::Normal(_) | std::path::Component::CurDir))
}

/// Files with gitignore syntax that hide paths from spec17, read from the repo root: `.aiignore` (shared with other agent
/// tools) and `.spec17ignore` (this tool only). `.agents` (installed skills) is always hidden.
const IGNORE_FILES: [&str; 2] = [".aiignore", ".spec17ignore"];

pub struct Ignore(ignore::gitignore::Gitignore);

impl Ignore {
    pub fn load(root: &Path) -> Self {
        let mut b = ignore::gitignore::GitignoreBuilder::new(root);
        let _ = b.add_line(None, ".agents");
        for f in IGNORE_FILES {
            if root.join(f).is_file() {
                let _ = b.add(root.join(f)); // a bad line is skipped, the rest still applies
            }
        }
        Ignore(b.build().unwrap_or_else(|_| ignore::gitignore::Gitignore::empty()))
    }

    /// Modules hidden by an anchored folder line (`/server/`, what `spec_module` writes) in `.spec17ignore`.
    pub fn hidden_modules(root: &Path) -> Vec<String> {
        let text = std::fs::read_to_string(root.join(".spec17ignore")).unwrap_or_default();
        let set: std::collections::BTreeSet<&str> = text.lines().filter_map(|l| l.trim().strip_prefix('/')?.strip_suffix('/')).filter(|m| !m.is_empty() && !m.contains(['*', '?', '[', '\\', '!', '#'])).collect();
        set.into_iter().map(str::to_owned).collect()
    }

    // (API-11, API-12)
    /// Hide (`hidden`) or show a module folder: add or remove its anchored line in `.spec17ignore`, keeping every other line. To hide, `listed`
    /// says the scanner lists the module; the root and anything glob-like are never hidden, so the line is that folder and nothing else.
    /// Returns what the file says now.
    pub fn set_module_hidden(root: &Path, module: &str, hidden: bool, listed: impl FnOnce() -> bool) -> Result<String, String> {
        let want = module.trim_matches('/');
        let line = format!("/{want}/");
        let file = root.join(".spec17ignore");
        let mut text = std::fs::read_to_string(&file).unwrap_or_default();
        let is_hidden = Self::hidden_modules(root).iter().any(|m| m == want);
        if hidden {
            if want == "." || want.contains(['*', '?', '[', '\\', '\n']) || !listed() {
                return Err(format!("{want}: not a module that can be hidden"));
            }
            if !is_hidden {
                if !text.is_empty() && !text.ends_with('\n') {
                    text.push('\n');
                }
                text += &format!("{line}\n");
            }
        } else {
            if !is_hidden {
                return Err(format!("{want}: not a hidden module"));
            }
            text = text.lines().filter(|l| l.trim() != line).map(|l| format!("{l}\n")).collect();
        }
        std::fs::write(&file, text).map_err(|e| format!(".spec17ignore: {e}"))?;
        Ok(format!("{line} is {}in .spec17ignore\n", if hidden { "" } else { "not " }))
    }

    /// Whether a repo-relative path, or any folder above it, is hidden.
    pub fn hides(&self, rel: &str) -> bool {
        let rel = rel.trim_start_matches("./");
        !rel.is_empty() && rel != "." && self.0.matched_path_or_any_parents(rel, false).is_ignore()
    }
}

fn wanted(e: &DirEntry, allow: &[PathBuf]) -> bool {
    let skip = e.depth() > 0 && e.file_type().is_dir() && {
        let n = e.file_name().to_string_lossy();
        n.starts_with('.') || SKIP.contains(&n.as_ref())
    };
    !skip && (!e.path_is_symlink() || inside(e.path(), allow))
}

// (SCAN-10, SCAN-15)
/// Files under `dir`, sorted. Symlinks are followed only when they resolve inside `allow` (its first entry is the repo root);
/// junk folders and what `ignore` (the root's `.aiignore` / `.spec17ignore`) hides are skipped.
pub fn walk(dir: &Path, allow: &[PathBuf], ignore: &Ignore) -> Vec<PathBuf> {
    let base = allow.first().and_then(|root| dir.canonicalize().ok()?.strip_prefix(root).ok().map(Path::to_path_buf));
    let hides = |e: &DirEntry| {
        base.as_ref().is_some_and(|base| e.path().strip_prefix(dir).is_ok_and(|r| ignore.hides(&base.join(r).to_string_lossy().replace('\\', "/"))))
    };
    let mut v: Vec<PathBuf> = WalkDir::new(dir).follow_links(true).into_iter().filter_entry(|e| wanted(e, allow) && !hides(e)).filter_map(Result::ok).filter(|e| e.file_type().is_file()).map(DirEntry::into_path).collect();
    v.sort();
    v
}

fn is_md(p: &Path) -> bool {
    p.extension().is_some_and(|x| x == "md")
}

/// Does markdown file `f` hold a requirement line, a block one (`### ID` with the level further down) too?
fn has_requirement(f: &Path) -> bool {
    crate::scan::read_text(f).is_some_and(|t| crate::scan::holds_requirement(&t))
}

/// Decision-record folders of `root` that exist.
pub fn decision_dirs(root: &Path) -> Vec<String> {
    DECISION_DIRS.iter().filter(|d| root.join(d).is_dir()).map(|d| d.to_string()).collect()
}

/// Single-file specs (`SPEC.md`, `docs/REQUIREMENTS.md` …) of `root`, whatever the case of the name.
fn spec_file_names(root: &Path) -> Vec<String> {
    let mut v = vec![];
    for dir in ["", "docs"] {
        let mut names: Vec<String> = std::fs::read_dir(root.join(dir)).into_iter().flatten().filter_map(Result::ok).filter(|e| e.path().is_file() && is_md(&e.path())).filter_map(|e| e.file_name().into_string().ok()).filter(|n| SPEC_STEMS.contains(&n.trim_end_matches(".md").to_uppercase().as_str())).collect();
        names.sort();
        v.extend(names.into_iter().map(|n| if dir.is_empty() { n } else { format!("{dir}/{n}") }));
    }
    v
}

/// Cheap check for `find_root`: does `d` look like a repo with specs? Reads no file, except in a generic `docs/` when nothing else is there.
pub fn has_spec_marker(d: &Path, allow: &[PathBuf]) -> bool {
    if !spec_file_names(d).is_empty() || !decision_dirs(d).is_empty() || d.join("specs").is_dir() {
        return true;
    }
    let ignore = Ignore::load(d);
    !named_dirs(d, allow, &ignore).is_empty() || generic_dir(d, allow, &ignore).is_some()
}

/// Where the requirement lines of `root` are, repo-relative folders or files: the spec folders, single-file specs, and when
/// none of them holds a requirement line, every markdown file of the repo that does. Empty when the repo has no such place.
// (SCAN-09, SCAN-12, SCAN-26)
// ponytail: the fallback walks the whole repo's markdown once per load; add a cache if repos get huge.
pub fn spec_paths(root: &Path, allow: &[PathBuf], ignore: &Ignore) -> Vec<String> {
    let dirs = scan_dirs(root, allow, ignore);
    let files = spec_file_names(root);
    let holds = dirs.iter().any(|d| d.1) || files.iter().any(|f| has_requirement(&root.join(f)));
    let mut v: Vec<String> = dirs.into_iter().map(|d| d.0).chain(files).collect();
    if !holds {
        v.extend(walk(root, allow, ignore).iter().filter(|f| is_md(f) && has_requirement(f)).filter_map(|f| f.strip_prefix(root).ok()).map(|f| f.to_string_lossy().replace('\\', "/")));
    }
    v
}

/// Every place `spec_paths` may pick, found without reading a file: the dedicated spec folders, existing generic doc folders and
/// single-file specs, else every markdown file of the repo. For change polling, which must see an edit wherever it lands.
// ponytail: a repo whose spec folders hold no requirement line but whose other markdown does gets only the folders here, so an
// edit to that markdown is seen only when the file list has it (a walk, not CodeGraph); read once per stamp change if that matters.
pub fn spec_places(root: &Path, allow: &[PathBuf], ignore: &Ignore) -> Vec<String> {
    let mut v: Vec<String> = named_dirs(root, allow, ignore).into_iter().map(|d| d.0).collect();
    let files = spec_file_names(root);
    let fallback = v.is_empty() && files.is_empty();
    v.extend(GENERIC.iter().filter(|g| root.join(g).is_dir() && confined(root, g, allow)).map(|g| g.to_string()));
    v.extend(files);
    if fallback {
        v.extend(walk(root, allow, ignore).iter().filter(|f| is_md(f)).filter_map(|f| f.strip_prefix(root).ok()).map(|f| f.to_string_lossy().replace('\\', "/")));
    } else if v.is_empty() && root.join("specs").is_dir() {
        v.push("specs".to_string());
    }
    v
}

/// The main spec folder of `root` (see `scan_dirs`). `None` when the repo has none.
pub fn spec_dir(root: &Path, allow: &[PathBuf]) -> Option<String> {
    scan_dirs(root, allow, &Ignore::load(root)).into_iter().next().map(|d| d.0)
}

/// The spec folders of `root` that hold markdown, each with whether it holds a requirement line, in a fixed order: the dedicated
/// names of `SPEC_DIRS`, each in-flight OpenSpec change's `specs/` (sorted, `archive/` left out), Superpowers' design docs; a generic
/// `docs/` only when none of them holds a requirement line, and a bare `specs/` when nothing else is there. Folders that hold a
/// requirement line come first: the first one is the main one. A folder that resolves out of the repo (a symlink) is left out (SCAN-10).
fn scan_dirs(root: &Path, allow: &[PathBuf], ignore: &Ignore) -> Vec<(String, bool)> {
    let mut v: Vec<(String, bool)> = named_dirs(root, allow, ignore).into_iter().map(|(d, fs)| {
        let holds = fs.iter().any(|f| has_requirement(f));
        (d, holds)
    }).collect();
    if !v.iter().any(|d| d.1) && let Some(g) = generic_dir(root, allow, ignore) {
        v.push((g, true));
    }
    if v.is_empty() && root.join("specs").is_dir() {
        v.push(("specs".to_string(), false));
    }
    v.sort_by_key(|d| !d.1); // stable: the fixed order within each group
    v
}

fn mds(root: &Path, d: &str, allow: &[PathBuf], ignore: &Ignore) -> Vec<PathBuf> {
    walk(&root.join(d), allow, ignore).into_iter().filter(|p| is_md(p)).collect()
}

/// The dedicated spec folders of `root` (see `scan_dirs`) that hold markdown, with their markdown files. Reads no file.
fn named_dirs(root: &Path, allow: &[PathBuf], ignore: &Ignore) -> Vec<(String, Vec<PathBuf>)> {
    let mut changes: Vec<String> = std::fs::read_dir(root.join("openspec/changes")).into_iter().flatten().filter_map(Result::ok)
        .filter_map(|e| e.file_name().into_string().ok()).filter(|n| n != "archive").map(|n| format!("openspec/changes/{n}/specs")).collect();
    changes.sort();
    let named = SPEC_DIRS.iter().filter(|d| !GENERIC.contains(d)).map(|d| d.to_string()).chain(changes).chain(["docs/superpowers/specs".to_string()]);
    named.filter(|d| root.join(d).is_dir() && confined(root, d, allow)).filter_map(|d| {
        let fs = mds(root, &d, allow, ignore);
        (!fs.is_empty()).then_some((d, fs))
    }).collect()
}

/// The first generic doc folder (`docs/`, `doc/`) that holds a requirement line.
fn generic_dir(root: &Path, allow: &[PathBuf], ignore: &Ignore) -> Option<String> {
    GENERIC.iter().find(|g| root.join(g).is_dir() && confined(root, g, allow) && mds(root, g, allow, ignore).iter().any(|f| has_requirement(f))).map(|g| g.to_string())
}

/// What the file list says about the code: the dominant ecosystem and the folders of its sources and tests.
pub struct Layout<'a> {
    pub profile: &'a Profile,
    pub src: Vec<String>,
    pub tests: Vec<String>,
}

fn ext_of(f: &str) -> &str {
    f.rsplit_once('.').map_or("", |(_, x)| x)
}

fn is_test_dir(c: &str) -> bool {
    matches!(c, "test" | "tests" | "spec" | "__tests__" | "e2e" | "cypress" | "testing") || c.ends_with("Test") || c.ends_with("Tests")
}

/// The folder a source file belongs to: up to its `src` (or `src/main`), else its top-level folder, else `.`.
fn src_root(f: &str) -> String {
    let dirs: Vec<&str> = f.split('/').collect::<Vec<_>>().split_last().map_or(vec![], |(_, d)| d.to_vec());
    match dirs.iter().position(|c| matches!(*c, "src" | "Sources" | "Source" | "lib" | "app")) {
        Some(i) => dirs[..=i + usize::from(dirs.get(i + 1) == Some(&"main"))].join("/"),
        None => dirs.first().map_or(".".to_string(), |d| d.to_string()),
    }
}

fn test_root(f: &str) -> Option<String> {
    let dirs: Vec<&str> = f.split('/').collect();
    let i = dirs[..dirs.len() - 1].iter().position(|c| is_test_dir(c))?;
    Some(dirs[..=i].join("/"))
}

/// Sorted, deduplicated, without folders that sit under another listed folder; `.` only when nothing else is left.
fn minimal(mut v: Vec<String>) -> Vec<String> {
    v.sort();
    v.dedup();
    let others: Vec<String> = v.iter().filter(|d| *d != ".").cloned().collect();
    let v = if others.is_empty() { v } else { others };
    v.iter().filter(|d| !v.iter().any(|o| o != *d && d.starts_with(&format!("{o}/")))).cloned().collect()
}

/// `files`: repo-relative, `/`-separated. `None` when no known ecosystem has a file.
pub fn layout<'a>(profiles: &'a [Profile], files: &[String]) -> Option<Layout<'a>> {
    let mine: Vec<&String> = files.iter().filter(|f| !f.split('/').any(|c| c.starts_with('.') && c != "." || SKIP.contains(&c))).collect();
    let (profile, _) = profiles.iter().map(|p| (p, mine.iter().filter(|f| p.ext.iter().any(|e| e == ext_of(f))).count())).fold(None, |best: Option<(&Profile, usize)>, c| if c.1 > best.map_or(0, |b| b.1) { Some(c) } else { best })?;
    let test: Vec<Regex> = profile.test.iter().map(|p| Regex::new(p).unwrap()).collect();
    let (tests, src): (Vec<&String>, Vec<&String>) = mine.into_iter().filter(|f| profile.ext.iter().any(|e| e == ext_of(f))).partition(|f| test.iter().any(|r| r.is_match(f)));
    let src_dirs = minimal(src.iter().map(|f| src_root(f)).collect());
    let mut test_dirs: Vec<String> = tests.iter().filter_map(|f| test_root(f)).collect();
    if profile.inline {
        test_dirs.extend(src_dirs.iter().cloned());
    }
    Some(Layout { profile, src: src_dirs, tests: minimal(test_dirs) })
}

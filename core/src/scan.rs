//! File scanning: spec IDs, test tags `[ABC-12]`, source comment mentions. Port of scripts/common.py (minus SQL).
use crate::codegraph::{Graph, GraphError};
use crate::config::Config;
use crate::detect;
use indexmap::IndexMap;
use regex::Regex;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{cell::{OnceCell, RefCell}, collections::HashMap, path::{Path, PathBuf}, process::Command, rc::Rc};

/// An identifier of 4+ characters: what the tests of a requirement name.
pub static WORD: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| Regex::new(r"(?-u:\b)[A-Za-z_]\w{3,}(?-u:\b)").unwrap());

/// The patterns of a requirement line. Loaded from `resources/spec_format.toml`.
#[derive(serde::Deserialize)]
pub struct Format {
    pub id: Vec<String>,
    pub not_id: Vec<String>,
    pub level: String,
    pub block_level: String,
    pub done: String,
    pub priority: Vec<String>,
    pub effort: Vec<String>,
    pub line: Vec<String>,
    pub block: Vec<String>,
    pub removed: String,
    pub unnamed: String,
}

impl Format {
    /// The ID rules as one regex, without a capture group.
    pub fn id(&self) -> String {
        format!("(?:{})", self.id.iter().map(|r| format!("(?:{r})")).collect::<Vec<_>>().join("|"))
    }
}

/// Is `id` a requirement ID, not a well-known token such as `UTF-8`?
pub fn is_id(id: &str) -> bool {
    !LINE.not_id.is_match(id)
}

/// To accept a new spec format, edit `resources/spec_format.toml`.
pub static FORMAT: std::sync::LazyLock<Format> = std::sync::LazyLock::new(|| toml::from_str(include_str!("../resources/spec_format.toml")).expect("resources/spec_format.toml"));

/// The requirement-line regexes, built once from `FORMAT`: shared by the spec scan, `detect` and `changes`.
// (SCAN-23, SCAN-24)
pub struct Line {
    /// Start of a requirement line: a `line` or `block` opener, an optional check mark, then the ID, plain, bold or in backticks,
    /// with an optional `:`. Group 1 is the ID, group 2 the rest of the line.
    pub head: Regex,
    pub level: Regex,
    priority: Patterns,
    effort: Patterns,
    /// The level in a heading or paragraph block: EARS `shall` in lower case counts too.
    block_level: Regex,
    /// A `block` opener of `spec_format.toml`: a heading or paragraph whose level may sit on any of its lines.
    opens_block: Regex,
    /// A wrapped bullet continues until a blank line, bullet, heading or table row.
    wrap_end: Regex,
    removed: Regex,
    pub(crate) unnamed: Regex,
    /// The whole of a well-known token (`UTF-8`).
    not_id: Regex,
}

pub static LINE: std::sync::LazyLock<Line> = std::sync::LazyLock::new(|| {
    let f = &*FORMAT;
    let r = |s: String| Regex::new(&s).unwrap_or_else(|e| panic!("resources/spec_format.toml: {s}: {e}"));
    let open = f.line.iter().chain(&f.block).cloned().collect::<Vec<_>>().join("|");
    Line {
        head: r(format!(r"^\s*(?:{open})(?:{}\s*)?[*_`]*({})(?-u:\b)[*_`]*[:.]?[*_`]*(.*)", f.done, f.id())),
        level: r(f.level.clone()),
        priority: Patterns(f.priority.iter().map(|s| r(s.clone())).collect()),
        effort: Patterns(f.effort.iter().map(|s| r(s.clone())).collect()),
        block_level: r(f.block_level.clone()),
        opens_block: r(format!(r"^\s*(?:{})", f.block.join("|"))),
        wrap_end: r(r"^\s*([-*+] |\d+[.)] |#|\|)".into()),
        removed: r(f.removed.clone()),
        unnamed: r(f.unnamed.clone()),
        not_id: r(format!("^(?:{})$", f.not_id.join("|"))),
    }
});

// (SCAN-25)
/// A file's text, lossy UTF-8, cleaned as `clean` does.
pub fn read_text(p: &Path) -> Option<String> {
    std::fs::read(p).ok().map(|b| clean(&String::from_utf8_lossy(&b)))
}

/// Text as the scan reads it: no byte-order mark, and the Unicode hyphens that editors paste into IDs (U+2010, U+2011) as `-`.
pub fn clean(t: &str) -> String {
    t.strip_prefix('\u{FEFF}').unwrap_or(t).replace(['\u{2010}', '\u{2011}'], "-")
}

/// For each line, whether it sits in a section whose requirements are retired (`## REMOVED Requirements`); a `#`/`##` heading opens a section.
pub fn removed_lines(ls: &[String]) -> Vec<bool> {
    let mut gone = false;
    ls.iter().map(|l| {
        if l.starts_with("# ") || l.starts_with("## ") {
            gone = LINE.removed.is_match(l);
        }
        gone
    }).collect()
}

/// End of the block the requirement line `n` (1-based) opens, as a 0-based exclusive index: a table row is one line, a
/// heading runs to the next heading or requirement line, a bullet or `**ID**` paragraph wraps until a blank line, bullet, heading or table row.
pub fn block_end(ls: &[String], n: usize) -> usize {
    let l = ls[n - 1].trim_start();
    let mut j = n;
    if l.starts_with('|') {
        return j;
    }
    if l.starts_with('#') {
        while j < ls.len() && !ls[j].trim_start().starts_with('#') && !LINE.head.is_match(&ls[j]) {
            j += 1;
        }
        while j > n && ls[j - 1].trim().is_empty() {
            j -= 1;
        }
        return j;
    }
    while j < ls.len() && !ls[j].trim().is_empty() && !LINE.wrap_end.is_match(&ls[j]) {
        j += 1;
    }
    j
}

/// The level of the requirement on `line`, whose ID is followed by `rest`: on the line itself for a bullet or table row, anywhere
/// in `more` (the block's other lines) for a heading or paragraph. `None`: no level, no requirement.
pub fn level_of(line: &str, rest: &str, more: &[String]) -> Option<String> {
    if LINE.opens_block.is_match(line) {
        LINE.block_level.find(&format!("{rest} {}", more.join(" "))).map(|x| x.as_str().to_uppercase())
    } else {
        LINE.level.find(rest).map(|x| x.as_str().to_string())
    }
}

/// Does `text` hold a requirement line (an ID with a level)?
pub fn holds_requirement(text: &str) -> bool {
    if !text.lines().any(|l| LINE.head.is_match(l)) {
        return false;
    }
    let ls: Vec<String> = text.lines().map(str::to_owned).collect();
    ls.iter().enumerate().any(|(i, l)| LINE.head.captures(l).is_some_and(|m| is_id(&m[1]) && level_of(l, &m[2], &ls[i + 1..block_end(&ls, i + 1)]).is_some()))
}

#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
pub struct Spec {
    pub id: String,
    pub file: String,
    pub line: usize,
    pub end: usize,
    pub level: String,
    pub text: String,
    pub priority: Option<String>,
    pub effort: Option<String>,
    /// "" | "manual" | "later"
    pub mark: String,
    pub done: bool,
}

#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
pub struct Ref {
    pub file: String,
    pub line: usize,
    pub text: String,
}

/// A mistake found in the specs, about one requirement ID. Prints as `ID message`, which is how it reaches the reports and the JSON.
#[derive(Debug, Clone)]
pub struct Issue {
    pub id: String,
    pub msg: String,
    /// The bullet has no MUST/SHOULD/MAY, so it is no requirement and is not counted.
    pub uncounted: bool,
}

impl Issue {
    pub fn new(id: &str, msg: String) -> Issue {
        Issue { id: id.into(), msg, uncounted: false }
    }

    fn uncounted(id: &str, msg: String) -> Issue {
        Issue { uncounted: true, ..Issue::new(id, msg) }
    }
}

impl std::fmt::Display for Issue {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} {}", self.id, self.msg)
    }
}

pub type Refs = HashMap<String, Vec<Ref>>;

/// One detected test case, as 0-based line numbers (SCAN-35).
#[derive(Debug, Clone, Copy)]
pub(crate) struct Case {
    /// The line that opens the case (`@Test`, `it "..." do`).
    pub marker: usize,
    /// The declaration the marker attaches to: the marker itself, or the first declaration below it.
    pub decl: usize,
    /// The first line of the comments and annotations attached above the marker.
    pub head: usize,
    /// One past the last line of the body.
    pub end: usize,
}

/// How the files of one ecosystem read: line-comment markers, the declaration, assertion and switched-off regexes, which paths
/// are tests and which line opens a test case.
// (SCAN-33)
pub struct Lang {
    pub comment: Vec<String>,
    pub decl: Patterns,
    assert: Patterns,
    off: Patterns,
    /// None when `spec17.json` lists both `src` and `tests` and does not set this profile's `test`: the folders say it all.
    test: Option<Patterns>,
    pub(crate) case: Patterns,
    /// `case` matches a call: the name is its quoted string.
    pub(crate) call: bool,
    /// A `'` may open a lifetime, not a string (`Profile::lifetimes`).
    pub(crate) lifetimes: bool,
    /// (SCAN-35) Lines that attach to the declaration below (`Profile::annotation`).
    annotation: Patterns,
    /// (SCAN-35) How a body ends without braces (`Profile::block_*`), and the strings and comments left out when counting.
    block_indent: bool,
    block_open: Patterns,
    block_close: Patterns,
    block_end: Patterns,
    block_fence: Patterns,
    strip: Regex,
}

pub struct Patterns(Vec<Regex>);

impl Patterns {
    fn is_match(&self, text: &str) -> bool { self.0.iter().any(|r| r.is_match(text)) }
    fn count(&self, text: &str) -> usize { self.0.iter().map(|r| r.find_iter(text).count()).sum() }
    pub(crate) fn captures<'a>(&self, text: &'a str) -> Option<regex::Captures<'a>> { self.0.iter().find_map(|r| r.captures(text)) }
}

impl Lang {
    fn new(p: &detect::Profile, test: bool) -> Result<Lang, String> {
        let r = |patterns: &[String]| patterns.iter().map(|s| Regex::new(s).map_err(|e| format!("profile {}: bad regex {s}: {e}", p.name))).collect::<Result<Vec<_>, _>>().map(Patterns);
        // one way to end a body: indentation, or nesting (both patterns), possibly with an end line and a fence
        if p.block_indent && !(p.block_open.is_empty() && p.block_close.is_empty() && p.block_end.is_empty() && p.block_fence.is_empty()) {
            return Err(format!("profile {}: block_indent cannot be combined with block_open, block_close, block_end or block_fence", p.name));
        }
        if p.block_open.is_empty() != p.block_close.is_empty() {
            return Err(format!("profile {}: block_open and block_close go together: a level that opens must close", p.name));
        }
        let decl = r(&p.decl)?;
        for (s, regex) in p.decl.iter().zip(&decl.0) {
            if regex.captures_len() < 2 && s != detect::NEVER {
                return Err(format!("profile {}: decl {} needs a capture group 1 for the declared name", p.name, s));
            }
        }
        // a string or a line comment, tried left to right: a marker inside a string is no comment
        let strip = Regex::new(&["\"(?:\\\\.|[^\"\\\\])*\"".to_string(), "'(?:\\\\.|[^'\\\\])*'".to_string()].into_iter().chain(p.comment.iter().map(|c| format!("{}.*", regex::escape(c)))).collect::<Vec<_>>().join("|")).map_err(|e| format!("profile {}: {e}", p.name))?;
        Ok(Lang {
            comment: p.comment.clone(), decl, assert: r(&p.assert)?, off: r(&p.off)?, test: if test { Some(r(&p.test)?) } else { None }, case: r(&p.case)?, call: p.call, lifetimes: p.lifetimes, annotation: r(&p.annotation)?,
            block_indent: p.block_indent, block_open: r(&p.block_open)?, block_close: r(&p.block_close)?, block_end: r(&p.block_end)?, block_fence: r(&p.block_fence)?, strip,
        })
    }

    /// Whether byte `at` of `line` sits inside a string literal or a trailing comment.
    fn inside_literal(&self, line: &str, at: usize) -> bool {
        self.strip.find_iter(line).any(|t| t.start() < at && at < t.end())
    }

    /// A comment or an annotation: what attaches to the declaration below it.
    fn attaches(&self, line: &str) -> bool {
        self.is_comment(line.trim_start()) || self.annotation.is_match(line)
    }

    // (SCAN-35) The exclusive end of the body that starts on line `at`, looking no further than line `limit`, when the profile describes one; None: use braces.
    fn block_end_of(&self, ls: &[String], at: usize, limit: usize) -> Option<usize> {
        if self.block_indent {
            let decl = ls[at].len() - ls[at].trim_start().len();
            return Some((at + 1..limit).find(|&i| { let t = ls[i].trim_start(); !t.is_empty() && ls[i].len() - t.len() <= decl }).unwrap_or(limit));
        }
        if self.block_open.0.is_empty() && self.block_end.0.is_empty() {
            return None;
        }
        // shortcut: nesting by regex, so a keyword in odd syntax (a Ruby `end` method, a one-line `begin end`) miscounts; a parser is exact
        let (mut fence, mut depth, mut opened) = (None::<String>, 0i64, false);
        for (i, line) in ls.iter().enumerate().take(limit).skip(at) {
            let code = self.strip.replace_all(line, " ");
            for r in &self.block_fence.0 {
                for m in r.find_iter(&code) {
                    match &fence {
                        Some(tag) if tag == m.as_str() => return Some(i + 1),
                        None => fence = Some(m.as_str().to_string()),
                        _ => {}
                    }
                }
            }
            if fence.is_some() { continue; }
            let n = self.block_open.count(&code);
            opened |= n > 0;
            depth += n as i64 - self.block_close.count(&code) as i64;
            if self.block_end.is_match(line) || (opened && depth <= 0) { return Some(i + 1); }
        }
        Some(limit)
    }

    /// The text after the first line-comment marker of `line`.
    pub(crate) fn comment_of<'a>(&self, line: &'a str) -> Option<&'a str> {
        self.comment.iter().filter_map(|c| line.find(c.as_str()).map(|p| (p, c.len()))).min().map(|(p, l)| &line[p + l..])
    }

    pub fn is_comment(&self, text: &str) -> bool {
        self.comment.iter().any(|c| text.starts_with(c.as_str()))
    }
}

pub struct Rx {
    link: Regex,
    adr: Regex,
    code_span: Regex,
    tag: Regex,
    pub mention: Regex,
    mark: Regex,
    done: Regex,
    struck: Regex,
    later_inside: Regex,
    retired: Regex,
}

impl Rx {
    fn new() -> Result<Rx, String> {
        let r = |s: String| Regex::new(&s).map_err(|e| format!("bad regex {s}: {e}"));
        let (f, id) = (&*FORMAT, &FORMAT.id());
        Ok(Rx {
            link: r(format!(r"\[({id})\]\(([^)#\s]+)(?:#[^)]*)?\)"))?, // `[ADR-0003](../decisions/0003-x.md#why)`
            adr: r(r"^(\d{1,4})-".into())?,
            code_span: r(r"`[^`]*`".into())?,
            tag: r(format!(r"\[({id}(?:\s*[,;]\s*{id})*)\]"))?, // `[ABC-12]`, `[ABC-12][ABC-14]` and `[ABC-12, ABC-14]`
            mention: r(format!(r"(?-u:\b)({id})(?-u:\b)"))?, // ASCII `\b`: a Unicode one is slow on non-ASCII lines
            mark: r(r"\((later|deferred|manual)\)$".into())?, // `(deferred)` means the same as `(later)`
            done: r(f.done.clone())?,
            struck: r(r"~~.*?~~".into())?,
            later_inside: r(r"\((?:later|deferred|manual)\)".into())?,
            retired: r(format!(r"^\s*(?:[-*+]|\d+[.)])\s+~~[*_`]*({id})"))?,
        })
    }
}

/// The text of a requirement's first line: what follows the ID, minus separators and a level that opens it.
pub(crate) fn lead(rest: &str, level: &str) -> String {
    let sep = |c: char| c.is_whitespace() || "|:—–-".contains(c);
    let rest = rest.trim_start_matches(sep);
    rest.strip_prefix(level).unwrap_or(rest).trim_start_matches(sep).trim_end_matches(|c: char| c.is_whitespace() || c == '|').to_string()
}

pub struct Project {
    pub root: PathBuf,
    pub cfg: Config,
    pub rx: Rx,
    /// The built-in profiles and those of `spec17.json` (`detect::profiles`).
    pub profiles: Vec<detect::Profile>,
    /// (SCAN-40) `priority` and `effort` of `spec17.json`; None = the patterns of `spec_format.toml`.
    priority: Option<Patterns>,
    effort: Option<Patterns>,
    /// The dominant one: it reads the files no profile knows by extension.
    main: Option<usize>,
    /// One per profile, built on first use.
    langs: Vec<OnceCell<Lang>>,
    /// No comment markers, no declarations: a file of an ecosystem nothing describes.
    none: Lang,
    /// Per profile: whether its `test` regex makes a file a test (`Lang::test`).
    tests_by_path: Vec<bool>,
    /// Where a symlink may point: the root and `ALLOWED_PATHS`.
    allow: Vec<PathBuf>,
    /// Folders and files (repo-relative) that hold requirement lines, and decision-record folders.
    spec_paths: Vec<String>,
    doc_dirs: Vec<String>,
    /// Repo-relative files of the repo: the CodeGraph file list, else a walk.
    pub files: Vec<String>,
    ignore: detect::Ignore,
    cache: RefCell<HashMap<String, Rc<Vec<String>>>>,
    /// Repo-relative files under a folder, walked once per folder.
    walked: RefCell<HashMap<String, Rc<Vec<String>>>>,
    /// The index as `open` found it, for `Analysis::load` to take instead of opening it again.
    graph: Option<Result<Option<Graph>, GraphError>>,
    /// The file lists, walked once.
    tests: OnceCell<Vec<String>>,
    srcs: OnceCell<Vec<String>>,
    specs: OnceCell<Vec<String>>,
    /// Struck-out IDs and IDs under `## REMOVED Requirements`, read once.
    gone: OnceCell<(Vec<String>, Vec<String>)>,
}

/// The repo is the nearest ancestor of `from` that has the `specs` path, or, when none is configured, any known spec place;
/// failing that the nearest git work tree.
pub fn find_root(from: &Path, specs: &str) -> Option<PathBuf> {
    from.ancestors().find(|d| if specs.is_empty() { detect::has_spec_marker(d, &detect::allow_list(d)) } else { d.join(specs).exists() })
        .or_else(|| from.ancestors().find(|d| d.join(".git").exists()))
        .map(Path::to_path_buf)
}

/// `from`'s folder joined with a link target, `..` resolved; `None` when it leaves the repo.
fn resolve(from: &str, target: &str) -> Option<String> {
    let mut parts: Vec<&str> = from.split('/').collect();
    parts.pop();
    if target.starts_with('/') {
        parts.clear();
    }
    for c in target.split('/') {
        match c {
            "" | "." => {}
            ".." => { parts.pop()?; }
            c => parts.push(c),
        }
    }
    Some(parts.join("/"))
}

impl Project {
    /// `root`: explicit repo root, else walk up from the current directory.
    pub fn open(root: Option<&Path>) -> Result<Project, String> {
        Self::open_with(root, true)
    }

    /// `open` for change polling: no file is read to find the specs, so the spec files are every markdown file of every place
    /// that may hold them (see `detect::spec_places`).
    pub fn open_unread(root: Option<&Path>) -> Result<Project, String> {
        Self::open_with(root, false)
    }

    fn open_with(root: Option<&Path>, read: bool) -> Result<Project, String> {
        let start = match root {
            Some(r) => r.canonicalize().map_err(|e| format!("{}: {e}", r.display()))?,
            None => std::env::current_dir().map_err(|e| e.to_string())?,
        };
        // the config can move the specs folder, so look for spec17.json first
        let base = start.ancestors().find(|d| d.join("spec17.json").exists()).unwrap_or(&start);
        let first = Config::load(base)?;
        // (SCAN-28) a repo with no specs still opens: only "no repo at all" is an error, so spec17 never walks `~` or `/` unasked
        let root = if root.is_some() { start.clone() } else { find_root(&start, &first.specs).ok_or_else(|| format!("spec17: no repo here (no git work tree and no specs) in {} or above; run inside a repo or pass --root", start.display()))? };
        let mut cfg = if root == base { first } else { Config::load(&root)? };
        let allow = detect::allow_list(&root);
        let ignore = detect::Ignore::load(&root);
        let doc_dirs = detect::decision_dirs(&root);
        // (SCAN-21)
        let named = [("specs", vec![cfg.specs.clone()]), ("src", cfg.src.clone()), ("tests", cfg.tests.clone()), ("codegraph", vec![cfg.codegraph.clone()])];
        if let Some((field, p)) = named.iter().find_map(|(f, ps)| ps.iter().find(|p| !p.is_empty() && !detect::confined(&root, p, &allow)).map(|p| (f, p))) {
            return Err(format!("spec17: {field} {p} in spec17.json is outside the repo (see ALLOWED_PATHS)"));
        }
        // (SCAN-12) `specs` may name a folder or a file; left empty, the places come from the repo
        let spec_paths = if cfg.specs.is_empty() {
            let v = if read { detect::spec_paths(&root, &allow, &ignore) } else { detect::spec_places(&root, &allow, &ignore) };
            cfg.specs = v.first().cloned().unwrap_or_default(); // the main one, as spec17.json would name it
            v
        } else if root.join(&cfg.specs).exists() {
            vec![cfg.specs.clone()]
        } else {
            vec![] // a configured path that is not there counts as no specs; `doctor` says so
        };
        // (SCAN-09) what spec17.json leaves out comes from the CodeGraph file list, else from a walk of the repo
        let graph = Graph::open(&root.join(&cfg.codegraph));
        let indexed = graph.as_ref().ok().and_then(Option::as_ref).and_then(|g| g.file_paths().ok());
        let files: Vec<String> = indexed.map(|v| v.into_iter().filter(|f| !ignore.hides(f)).collect()).unwrap_or_else(|| detect::walk(&root, &allow, &ignore).iter().filter_map(|f| f.strip_prefix(&root).ok()).map(|f| f.to_string_lossy().replace('\\', "/")).collect());
        let profiles = detect::profiles(&cfg.profiles)?;
        // a hand-written src/tests list keeps the user's meaning: no profile's test-path regex, unless spec17.json sets it
        let own = |p: &detect::Profile| cfg.profiles.iter().filter(|e| e.get("name").and_then(|n| n.as_str()) == Some(&p.name)).collect::<Vec<_>>();
        let tests_by_path: Vec<bool> = profiles.iter().map(|p| cfg.src.is_empty() || cfg.tests.is_empty() || own(p).iter().any(|e| e.contains_key("test"))).collect();
        // (SCAN-34) a bad regex in spec17.json is an error here, not a panic on first use; `test` too, which `cfg.detect` compiles
        for p in profiles.iter().filter(|p| !own(p).is_empty()) {
            Lang::new(p, true)?;
        }
        let hints = |name: &str, patterns: &[String]| -> Result<Option<Patterns>, String> {
            if patterns.is_empty() { return Ok(None); }
            patterns.iter().map(|s| Regex::new(s).map_err(|e| format!("spec17.json: {name}: bad regex {s}: {e}")).and_then(|r| if r.captures_len() < 2 { Err(format!("spec17.json: {name}: {s} needs a capture group 1 for the value")) } else { Ok(r) })).collect::<Result<_, _>>().map(|v| Some(Patterns(v)))
        };
        let (priority, effort) = (hints("priority", &cfg.priority)?, hints("effort", &cfg.effort)?);
        let main = cfg.detect(&profiles, &files);
        let rx = Rx::new()?;
        let langs = profiles.iter().map(|_| OnceCell::new()).collect();
        let none = Lang::new(&detect::Profile::unknown(""), false)?;
        Ok(Project { root, cfg, rx, profiles, priority, effort, main, langs, none, tests_by_path, allow, spec_paths, doc_dirs, files, ignore, cache: RefCell::default(), walked: RefCell::default(), graph: Some(graph), tests: OnceCell::new(), srcs: OnceCell::new(), specs: OnceCell::new(), gone: OnceCell::new() })
    }

    /// The CodeGraph index `open` already looked at (opened again if it was taken before).
    pub(crate) fn take_graph(&mut self) -> Result<Option<Graph>, GraphError> {
        self.graph.take().unwrap_or_else(|| Graph::open(&self.root.join(&self.cfg.codegraph)))
    }

    /// Whether `.aiignore`, `.spec17ignore` or the built-in `.agents` rule hides this repo-relative path.
    pub fn ignored(&self, rel: &str) -> bool {
        self.ignore.hides(rel)
    }

    pub fn lines(&self, rel: &str) -> Rc<Vec<String>> {
        if let Some(l) = self.cache.borrow().get(rel) {
            return l.clone();
        }
        let text = read_text(&self.root.join(rel)).unwrap_or_default();
        let l = Rc::new(text.lines().map(str::to_owned).collect::<Vec<_>>());
        self.cache.borrow_mut().insert(rel.to_owned(), l.clone());
        l
    }

    fn rel(&self, p: &Path) -> String {
        p.strip_prefix(&self.root).unwrap_or(p).to_string_lossy().replace('\\', "/")
    }

    /// Every file under `dir`, repo-relative and sorted, walked once (`src`, `tests` and the markdown scans share it).
    fn walked(&self, dir: &str) -> Rc<Vec<String>> {
        if let Some(v) = self.walked.borrow().get(dir) {
            return v.clone();
        }
        let at = if dir == "." { self.root.clone() } else { self.root.join(dir) };
        let v = Rc::new(detect::walk(&at, &self.allow, &self.ignore).iter().map(|p| self.rel(p)).collect::<Vec<_>>());
        self.walked.borrow_mut().insert(dir.to_owned(), v.clone());
        v
    }

    /// Files with a configured extension under each dir, sorted per dir. Symlinks count only inside the root or `ALLOWED_PATHS`.
    pub fn files(&self, dirs: &[String]) -> Vec<String> {
        let ext = |p: &&String| Path::new(p.as_str()).extension().and_then(|x| x.to_str()).is_some_and(|x| self.cfg.ext.iter().any(|y| y == x));
        dirs.iter().flat_map(|d| self.walked(d).iter().filter(ext).cloned().collect::<Vec<_>>()).collect()
    }

    fn is_test(&self, rel: &str) -> bool {
        self.lang(rel).test.as_ref().is_some_and(|r| r.is_match(rel))
    }

    /// A file in a tests folder, or one that its profile's `test` says is a test.
    pub fn is_test_path(&self, rel: &str) -> bool {
        self.is_test(rel) || self.cfg.tests.iter().any(|d| d == "." || rel.starts_with(&format!("{d}/")))
    }

    /// A file that exists to hold tests: test-named, or in a tests folder that is not also a source folder (Rust keeps unit tests
    /// inline in `src`, so those files are source that happens to declare tests).
    pub fn is_test_only(&self, rel: &str) -> bool {
        let in_src = self.cfg.src.iter().any(|d| d != "." && rel.starts_with(&format!("{d}/")));
        self.is_test(rel) || (self.is_test_path(rel) && !in_src)
    }

    /// A file under one of the `src` dirs.
    pub fn is_src_path(&self, rel: &str) -> bool {
        self.cfg.src.iter().any(|d| d == "." || rel.starts_with(&format!("{d}/")))
    }

    /// The test files: those under `tests`, plus test-named files that sit in `src` folders.
    pub fn test_files(&self) -> &[String] {
        self.tests.get_or_init(|| {
            let mut v = self.files(&self.cfg.tests);
            v.extend(self.files(&self.cfg.src).into_iter().filter(|f| self.is_test(f)));
            let mut seen = std::collections::HashSet::new();
            v.retain(|f| seen.insert(f.clone()));
            v
        })
    }

    /// The source files: those under `src` that are not tests.
    pub fn src_files(&self) -> &[String] {
        self.srcs.get_or_init(|| self.files(&self.cfg.src).into_iter().filter(|f| !self.is_test(f)).collect())
    }

    /// Folders and files the requirement lines are read from, plus decision folders, repo-relative.
    pub fn spec_paths(&self) -> Vec<&str> {
        self.spec_paths.iter().chain(&self.doc_dirs).map(String::as_str).collect()
    }

    /// Every `*.md` file of `dir` and its subfolders, repo-relative.
    fn md_in(&self, dir: &str) -> Vec<String> {
        self.walked(dir).iter().filter(|p| p.ends_with(".md")).cloned().collect()
    }

    /// The markdown files that may hold requirements: the spec paths (folders, files) and decision folders, repo-relative.
    pub fn spec_files(&self) -> &[String] {
        self.specs.get_or_init(|| {
            let mut seen = std::collections::HashSet::new();
            self.spec_paths.iter().chain(&self.doc_dirs).flat_map(|p| if self.root.join(p).is_file() { vec![p.clone()] } else { self.md_in(p) }).filter(|f| seen.insert(f.clone())).collect()
        })
    }

    // (SCAN-13)
    /// Reference docs and who links to them. `[ABC-12](path.md)` to an existing markdown file names that file by ID, and so does
    /// a decision-folder file `0003-title.md` (`ADR-0003`); IDs a bullet defines are left alone. Links to missing files go to `problems`.
    pub fn docs(&self, s: &IndexMap<String, Spec>, problems: &mut Vec<Issue>) -> (Vec<Spec>, Refs) {
        let mds = self.md_in(".");
        let known: std::collections::HashSet<&str> = mds.iter().map(String::as_str).collect();
        let (mut found, mut linked): (IndexMap<String, String>, Refs) = Default::default();
        for f in &mds {
            for (idx, line) in self.lines(f).iter().enumerate().filter(|(_, l)| l.contains("](")) {
                // a link shown in `code` is an example, not a link
                for c in self.rx.link.captures_iter(&self.rx.code_span.replace_all(line, "")) {
                    let (id, target) = (c[1].to_string(), &c[2]);
                    if target.contains(':') {
                        continue; // https:, mailto: …
                    }
                    linked.entry(id.clone()).or_default().push(Ref { file: f.clone(), line: idx + 1, text: line.trim().to_string() });
                    let Some(t) = resolve(f, target).filter(|t| t.ends_with(".md")) else { continue };
                    if !known.contains(t.as_str()) {
                        if !self.root.join(&t).exists() {
                            problems.push(Issue::new(&id, format!("{f}:{} links to {target}, which does not exist", idx + 1)));
                        }
                    } else if !s.contains_key(&id) {
                        match found.get(&id) {
                            Some(was) if *was != t => problems.push(Issue::new(&id, format!("is linked to two files: {was} and {t}"))),
                            Some(_) => {}
                            None => { found.insert(id, t); }
                        }
                    }
                }
            }
        }
        for d in &self.doc_dirs {
            for f in self.md_in(d) {
                let stem = f.rsplit('/').next().unwrap_or(&f);
                if let Some(m) = self.rx.adr.captures(stem) {
                    let id = format!("ADR-{}", &m[1]);
                    if !s.contains_key(&id) && !found.contains_key(&id) {
                        found.insert(id, f);
                    }
                }
            }
        }
        let docs = found.into_iter().map(|(id, file)| {
            let stem = file.rsplit('/').next().unwrap_or(&file).trim_end_matches(".md").to_string();
            let text = self.lines(&file).iter().find_map(|l| l.strip_prefix("# ").map(|h| h.trim().to_string())).unwrap_or(stem);
            Spec { id, file, line: 1, end: 1, level: "DOC".into(), text, priority: None, effort: None, mark: "ref".into(), done: false }
        }).collect();
        (docs, linked)
    }

    // (SCAN-01)
    /// {id: Spec} from `- **ABC-12** MUST — text` bullets (also `- ABC-12: text MUST …`, `1.`/`*` lists, `| ABC-12 | MUST | text |` rows), wrapped lines joined. A bullet ending in `(manual)`, `(later)` or
    /// `(deferred)` carries mark "manual" or "later"; struck-out bullets (`- ~~**ABC-12**`) are retired.
    /// Spec mistakes found on the way (duplicate ID, no level, marker not at the end) go to `problems`.
    pub fn specs(&self, problems: &mut Vec<Issue>) -> IndexMap<String, Spec> {
        let mut found: IndexMap<String, Spec> = IndexMap::new();
        for f in self.spec_files() {
            let ls = self.lines(f);
            let gone = removed_lines(&ls);
            for (idx, line) in ls.iter().enumerate() {
                let n = idx + 1;
                if gone[idx] {
                    continue; // retired: `retired` lists them
                }
                let Some(m) = LINE.head.captures(line).filter(|m| is_id(&m[1])) else {
                    // (SCAN-27)
                    if LINE.unnamed.is_match(line) {
                        let name = line.trim_start_matches(['#', ' ']).trim();
                        problems.push(Issue::uncounted(name, format!("{f}:{n} is a requirement without an ID, so no test can cite it; it is not counted")));
                    }
                    continue;
                };
                let id = m[1].to_string();
                let j = block_end(&ls, n);
                let rest: Vec<String> = ls[n..j].iter().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect();
                let Some(lv) = level_of(line, &m[2], &rest) else {
                    problems.push(Issue::uncounted(&id, format!("{f}:{n} has no MUST/SHOULD/MAY, so it is not counted")));
                    continue;
                };
                let m1 = m.get(1).unwrap();
                let mut parts = vec![lead(&m[2], &lv)];
                parts.extend(rest);
                let text = parts.join(" ").trim().to_string();
                let mut seg = vec![line[m1.end()..].to_string()];
                seg.extend(parts[1..].iter().cloned());
                let hay = format!("{}{}", &line[..m1.start()], seg.join(" "));
                let done = self.rx.done.is_match(&self.rx.struck.replace_all(&hay, ""));
                let end = self.rx.mark.captures(&text);
                let mark = match &end {
                    None => "",
                    Some(e) if &e[1] == "manual" => "manual",
                    Some(_) => "later",
                };
                // an OpenSpec change restates a requirement of `openspec/specs/` it modifies: the change's text wins
                let delta = |p: &str| p.starts_with("openspec/changes/");
                match found.get(&id) {
                    Some(was) if delta(&was.file) && !delta(f) => continue,
                    Some(was) if delta(&was.file) == delta(f) => problems.push(Issue::new(&id, format!("is defined twice: {}:{} and {f}:{n}", was.file, was.line))),
                    Some(_) => {}
                    None => {
                        let before = match &end {
                            Some(e) => &text[..e.get(0).unwrap().start()],
                            None => &text[..],
                        };
                        if self.rx.later_inside.is_match(&self.rx.struck.replace_all(before, "")) {
                            problems.push(Issue::new(&id, format!("{f}:{n} has (later)/(deferred)/(manual) before the end, so it counts as due")));
                        }
                    }
                }
                if done && !mark.is_empty() {
                    problems.push(Issue::new(&id, format!("{f}:{n} has a check mark but is also marked ({}/deferred)", if mark == "manual" { "manual" } else { "later" })));
                }
                // (SCAN-40)
                let priority = self.priority.as_ref().unwrap_or(&LINE.priority).captures(&text).and_then(|m| m.get(1)).map(|m| m.as_str().to_ascii_uppercase());
                let effort = self.effort.as_ref().unwrap_or(&LINE.effort).0.iter().flat_map(|r| r.captures_iter(&text)).filter_map(|m| m.get(1).map(|v| v.as_str().trim())).find(|v| !v.is_empty()).map(str::to_string);
                found.insert(id.clone(), Spec { id, file: f.clone(), line: n, end: j, level: lv, text, priority, effort, mark: mark.into(), done });
            }
        }
        // a line without a level that names a defined ID is a reference to it (a cross-reference table, a review note)
        problems.retain(|p| !(p.uncounted && found.contains_key(&p.id)));
        // a requirement an OpenSpec change removes is retired
        for i in &self.gone().1 {
            found.shift_remove(i);
        }
        found
    }

    /// The requirement IDs of every `[ID]` or `[ID, ID]` tag on `line`, in order.
    pub fn tagged(&self, line: &str) -> Vec<String> {
        self.rx.tag.captures_iter(line).flat_map(|g| self.rx.mention.captures_iter(&g[1]).map(|m| m[1].to_string()).filter(|i| is_id(i)).collect::<Vec<_>>()).collect()
    }

    /// Whether the CodeGraph index holds this very content of `file`: the SHA-256 it stored equals the file's own. A clock never decides what the text says.
    // (SCAN-35)
    pub(crate) fn index_holds(&self, graph: &Graph, file: &str) -> bool {
        let Some((stored, bytes)) = graph.indexed_hash(file).ok().flatten().zip(std::fs::read(self.root.join(file)).ok()) else { return false };
        stored.eq_ignore_ascii_case(&Sha256::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect::<String>())
    }

    // (SCAN-35) The test cases of a file, in line order.
    pub(crate) fn test_ranges(&self, file: &str, graph: Option<&Graph>) -> Vec<Case> {
        let lang = self.lang(file);
        let ls = self.lines(file);
        let decls = graph.filter(|g| !lang.call && self.index_holds(g, file)).and_then(|g| g.test_decls_in(file).ok()).unwrap_or_default();
        // pass 1: where each case starts
        let mut cases: Vec<Case> = ls.iter().enumerate().filter_map(|(m, line)| {
            // a marker inside a string literal (a fixture that writes a test file) or a trailing comment is not a test case
            let opens = lang.case.0.iter().find(|r| r.is_match(line)).is_some_and(|r| r.find_iter(line).any(|f| lang.call || !lang.inside_literal(line, f.start())));
            if !opens || lang.is_comment(line.trim_start()) { return None; }
            let decl = if lang.call { m } else {
                (m..ls.len()).take_while(|&i| i == m || lang.decl.is_match(&ls[i]) || lang.attaches(&ls[i]))
                    .find(|&i| lang.decl.is_match(&ls[i])).unwrap_or(m)
            };
            let indent = line.len() - line.trim_start().len();
            let leading = ls[..m].iter().rev().take_while(|l| l.len() - l.trim_start().len() == indent && lang.attaches(l)).count();
            Some(Case { marker: m, decl, head: m - leading, end: ls.len() })
        }).collect();
        // pass 2: where each body ends. The search stops at the next case's header, which would cut the body there anyway,
        // so a body that never balances costs its own length and not the rest of the file's.
        let heads: Vec<usize> = cases.iter().map(|c| c.head).collect();
        for (i, c) in cases.iter_mut().enumerate() {
            let limit = heads.get(i + 1).copied().filter(|&next| next > c.decl).unwrap_or(ls.len());
            let indexed = decls.iter().filter(|&&(lo, hi)| lo >= c.head as i64 + 1 && lo <= c.decl as i64 + 1 && hi >= c.decl as i64 + 1 && hi <= ls.len() as i64)
                .min_by_key(|&&(lo, hi)| hi - lo).map(|&(_, hi)| hi as usize);
            c.end = indexed.or_else(|| lang.block_end_of(&ls, c.decl, limit)).unwrap_or_else(|| c.decl + body(&ls, c.decl + 1, limit - c.decl, lang).len()).min(limit);
        }
        cases
    }

    // (SCAN-02)
    /// {id: [ref]} for every `[ID]` or `[ID, ID]` tag in the test folders.
    pub fn tests(&self) -> Refs {
        let mut found = Refs::new();
        for rel in self.test_files() {
            for (i, line) in self.lines(rel).iter().enumerate() {
                for id in self.tagged(line) {
                    found.entry(id).or_default().push(Ref { file: rel.clone(), line: i + 1, text: line.trim().to_string() });
                }
            }
        }
        found
    }

    // (SCAN-33)
    /// How a repo-relative file reads: the profile of its extension, else the dominant one.
    pub fn lang(&self, rel: &str) -> &Lang {
        let Some(i) = detect::profile_of(&self.profiles, rel).or(self.main) else { return &self.none };
        self.langs[i].get_or_init(|| Lang::new(&self.profiles[i], self.tests_by_path[i]).expect("profiles.toml, or checked in open"))
    }

    // (SCAN-03)
    /// {id: [ref]} for IDs mentioned after a line-comment marker in app sources (block comments are not scanned).
    pub fn sources(&self) -> Refs {
        let mut found = Refs::new();
        for rel in self.src_files() {
            let lang = self.lang(rel);
            for (i, line) in self.lines(rel).iter().enumerate() {
                if let Some(c) = lang.comment_of(line) {
                    for m in self.rx.mention.captures_iter(c).filter(|m| is_id(&m[1])) {
                        found.entry(m[1].to_string()).or_default().push(Ref { file: rel.clone(), line: i + 1, text: line.trim().to_string() });
                    }
                }
            }
        }
        found
    }

    /// An issue for every requirement that cites an ID no spec defines (retired IDs live in specs/README.md, not in bullets).
    pub fn check_refs(&self, s: &IndexMap<String, Spec>, issues: &mut Vec<Issue>) {
        let known = self.known_prefixes(s);
        for (i, spec) in s {
            let mut refs: Vec<String> = self.rx.mention.captures_iter(&spec.text).map(|m| m[1].to_string()).filter(|r| !s.contains_key(r)).collect();
            refs.sort();
            refs.dedup();
            for r in refs {
                if known.contains(&prefix(&r)) {
                    issues.push(Issue::new(i, format!("{}:{} cites {r}, which no spec defines", spec.file, spec.line)));
                }
            }
        }
    }

    fn known_prefixes(&self, s: &IndexMap<String, Spec>) -> std::collections::HashSet<String> {
        s.keys().map(|i| prefix(i)).chain(self.cfg.retired_prefixes.iter().cloned()).collect()
    }

    /// IDs a test or source cites that no spec defines, when the prefix is a spec prefix or a retired one (typos, retired IDs).
    pub fn dangling(&self, s: &IndexMap<String, Spec>, cited: &Refs) -> Vec<String> {
        let known = self.known_prefixes(s);
        let mut v: Vec<String> = cited.keys().filter(|i| !s.contains_key(*i) && known.contains(&prefix(i))).cloned().collect();
        v.sort();
        v
    }

    /// IDs whose bullet is struck through (`- ~~**ABC-12** …~~`): retired, never reassigned.
    /// Also those an OpenSpec change lists under `## REMOVED Requirements`.
    pub fn retired(&self) -> Vec<String> {
        let (struck, removed) = self.gone();
        struck.iter().chain(removed).cloned().collect()
    }

    // (SCAN-27)
    /// (struck-out IDs, IDs under an OpenSpec `## REMOVED Requirements` heading), from one pass over the spec files.
    fn gone(&self) -> &(Vec<String>, Vec<String>) {
        self.gone.get_or_init(|| {
            let (mut struck, mut removed) = (vec![], vec![]);
            for f in self.spec_files() {
                let ls = self.lines(f);
                for (line, gone) in ls.iter().zip(removed_lines(&ls)) {
                    if let Some(m) = self.rx.retired.captures(line).filter(|m| is_id(&m[1])) {
                        struck.push(m[1].to_string());
                    }
                    if let Some(m) = LINE.head.captures(line).filter(|m| gone && is_id(&m[1])) {
                        removed.push(m[1].to_string());
                    }
                }
            }
            (struck, removed)
        })
    }

    /// Tagged tests that prove nothing: kind -> [(id, file, line)], in a fixed kind order.
    /// unknown = ID in no spec; retired = struck-through spec; comment-only = every tag of the ID sits in a comment;
    /// disabled = every test is switched off; no-assert = no tagged test body has a direct assertion (it may assert through a helper: verify).
    pub fn test_problems(&self, s: &IndexMap<String, Spec>, t: &Refs, graph: Option<&Graph>) -> Vec<(&'static str, Vec<(String, String, usize)>)> {
        let gone: std::collections::HashSet<String> = self.retired().into_iter().collect();
        let mut out: Vec<(&'static str, Vec<(String, String, usize)>)> = ["unknown", "retired", "comment-only", "comment-tagged", "disabled", "no-assert"].into_iter().map(|k| (k, vec![])).collect();
        let slot = |out: &mut Vec<(&'static str, Vec<_>)>, k: &str, v| out.iter_mut().find(|(n, _)| *n == k).unwrap().1.push(v);
        let mut ids: Vec<&String> = t.keys().collect();
        let ranges: std::collections::HashMap<_, _> = self.test_files().iter().map(|f| (f.as_str(), self.test_ranges(f, graph))).collect();
        ids.sort();
        for i in ids {
            let refs = &t[i];
            let first = (i.clone(), refs[0].file.clone(), refs[0].line);
            if gone.contains(i) {
                slot(&mut out, "retired", first);
            } else if !s.contains_key(i) {
                slot(&mut out, "unknown", first);
            } else {
                // (SCAN-36) Preserve the actual comment location when an ID has several tags.
                let cases = |r: &Ref| ranges.get(r.file.as_str()).map_or(&[][..], Vec::as_slice);
                if let Some(r) = refs.iter().find(|r| self.flag(r, "comment-tagged", cases(r))) {
                    slot(&mut out, "comment-tagged", (i.clone(), r.file.clone(), r.line));
                }
                for kind in ["comment-only", "disabled", "no-assert"] {
                    if refs.iter().all(|r| self.flag(r, kind, cases(r))) {
                        slot(&mut out, kind, first.clone());
                    }
                }
            }
        }
        out
    }

    // (SCAN-35)
    fn flag(&self, r: &Ref, kind: &str, cases: &[Case]) -> bool {
        let (ls, lang) = (self.lines(&r.file), self.lang(&r.file));
        let comment = lang.is_comment(&r.text);
        let start = r.line - 1;
        let case = cases.iter().copied().filter(|c| c.head <= start && start < c.end).last();
        match kind {
            "comment-only" => comment && case.is_none(),
            "comment-tagged" => comment && case.is_some(),
            "disabled" => {
                if let Some(c) = case {
                    ls[c.head..=c.decl].iter().any(|x| lang.off.is_match(x))
                } else {
                    !comment && ls.iter().skip(start).take(4).take_while(|x| !lang.decl.is_match(x)).any(|x| lang.off.is_match(x))
                }
            }
            _ => if let Some(c) = case {
                !ls[c.decl..c.end].iter().any(|x| lang.assert.is_match(x))
            } else {
                !comment && !body(&ls, r.line, 200, lang).iter().any(|x| lang.assert.is_match(x))
            },
        }
    }

    // (SCAN-29)
    /// Name of the declaration a comment at 1-based line `n` of file `rel` belongs to: the next one for a comment line that leads into a
    /// declaration (only comment, doc and attribute lines between), else the nearest above.
    pub fn enclosing(&self, rel: &str, n: usize) -> String {
        let (ls, lang) = (self.lines(rel), self.lang(rel));
        let decl = |l: &String| lang.decl.captures(l).and_then(|m| m.get(1)).map(|m| m.as_str().to_string());
        let lead = |l: &&String| { let l = l.trim_start(); lang.is_comment(l) || l.starts_with("#[") || l.starts_with('@') };
        if lang.is_comment(ls[n - 1].trim_start()) && let Some(d) = ls[n..].iter().skip_while(lead).next().and_then(decl) {
            return d;
        }
        ls[..n].iter().rev().find_map(decl).unwrap_or_default()
    }

    pub(crate) fn git(&self, args: &[&str]) -> Result<String, String> {
        let r = Command::new("git").args(args).current_dir(&self.root).output().map_err(|e| format!("git: {e}"))?;
        if !r.status.success() {
            // the first line only: outside a work tree git follows its warning with a page of usage text
            return Err(format!("spec17: git {} failed: {}", args.join(" "), String::from_utf8_lossy(&r.stderr).lines().next().unwrap_or("").trim()));
        }
        Ok(String::from_utf8_lossy(&r.stdout).into_owned())
    }
}

/// A spec file name without its `.md`.
pub fn stem(file: &str) -> &str {
    file.strip_suffix(".md").unwrap_or(file)
}

pub fn prefix(id: &str) -> String {
    id.split(['-', '_']).next().unwrap_or("").to_string()
}

/// Sort key `(prefix, numbers)`: `AC-1.10` after `AC-1.2`, `REQ_002` after `REQ_001`.
pub fn id_key(id: &str) -> (String, Vec<u32>) {
    let (p, n) = id.rsplit_once(['-', '_']).unwrap_or((id, "0"));
    (p.to_string(), n.split('.').map(|x| x.parse().unwrap_or(0)).collect())
}

pub fn sorted_ids<'a>(ids: impl IntoIterator<Item = &'a String>) -> Vec<String> {
    let mut v: Vec<String> = ids.into_iter().cloned().collect();
    v.sort_by_key(|i| id_key(i));
    v
}

/// Lines of the function a 1-based tagged line `n` belongs to: from the tag to the brace that closes it.
pub fn body<'a>(ls: &'a [String], n: usize, cap: usize, lang: &Lang) -> &'a [String] {
    // shortcut: language-specific raw strings need a lexer when exact boundaries are required.
    // `'` opens a string only when it closes on the same line (so an apostrophe in prose cannot swallow the rest), or, in a language
    // with lifetimes, a char literal. Line comments come from the profile's `comment` markers.
    let closes = |rest: &str| { let mut it = rest.chars(); while let Some(c) = it.next() { if c == '\\' { it.next(); } else if c == '\'' { return true; } } false };
    let (start, mut depth, mut opened) = (n - 1, 0i32, false);
    let (mut quote, mut raw, mut triple, mut block_comment) = (None, None::<usize>, false, false);
    let stop = (start + cap).min(ls.len());
    for (k, line) in ls[start.min(stop)..stop].iter().enumerate() {
        let mut chars = line.chars();
        while let Some(c) = chars.next() {
            let rest = chars.as_str();
            if block_comment {
                if c == '*' && rest.starts_with('/') { chars.next(); block_comment = false; }
            } else if let Some(hashes) = raw {
                if c == '"' && rest.chars().take(hashes).filter(|c| *c == '#').count() == hashes {
                    for _ in 0..hashes { chars.next(); }
                    raw = None;
                }
            } else if triple {
                if c == '"' && rest.starts_with("\"\"") {
                    chars.next(); chars.next(); triple = false;
                }
            } else if let Some(q) = quote {
                if c == '\\' { chars.next(); } else if c == q { quote = None; }
            } else if lang.comment.iter().any(|m| line[line.len() - rest.len() - c.len_utf8()..].starts_with(m.as_str())) {
                break;
            } else if c == '/' && rest.starts_with('*') {
                chars.next(); block_comment = true;
            } else if c == 'r' && rest.trim_start_matches('#').starts_with('"') {
                let hashes = rest.chars().take_while(|c| *c == '#').count();
                for _ in 0..=hashes { chars.next(); }
                raw = Some(hashes);
            } else if c == '"' && rest.starts_with("\"\"") {
                chars.next(); chars.next(); triple = true;
            } else if c == '"' || c == '`' {
                quote = Some(c);
            } else if c == '\'' && (rest.starts_with('\\') || rest.chars().nth(1) == Some('\'') || (!lang.lifetimes && closes(rest))) {
                quote = Some(c);
            } else if c == '{' {
                depth += 1; opened = true;
            } else if c == '}' {
                depth -= 1;
            }
            if depth < 0 || (opened && depth == 0) { return &ls[start..start + k + 1]; }
        }
    }
    &ls[start.min(stop)..stop]
}

/// DOC-01,DOC-02,DOC-03,DOC-07 -> DOC-01..03,DOC-07
pub fn ranges<S: AsRef<str>>(ids: impl IntoIterator<Item = S>) -> String {
    // (prefix, [(number, as written)]): `AC-1.2` runs within `AC-1.`; after a `-` or `_` they print as written, at least two digits (`SEC-04`, `FR-001`), after a `.` as written
    let mut by: Vec<(String, Vec<(u32, String)>)> = vec![];
    for i in ids {
        let i = i.as_ref();
        let cut = i.rfind(['-', '_', '.']).map_or(0, |k| k + 1);
        let (p, n) = (&i[..cut], &i[cut..]);
        let n = (n.parse().unwrap_or(0), n.to_string());
        match by.iter_mut().find(|(q, _)| q == p) {
            Some((_, v)) => v.push(n),
            None => by.push((p.to_string(), vec![n])),
        }
    }
    let mut out = vec![];
    for (p, mut ns) in by {
        ns.sort();
        let mut run = vec![ns[0].clone()];
        for n in ns[1..].iter().cloned().map(Some).chain([None]) {
            if let Some(n) = &n && n.0 == run.last().unwrap().0 + 1 {
                run.push(n.clone());
                continue;
            }
            let num = |x: &(u32, String)| if p.ends_with(['-', '_']) { format!("{:0w$}", x.0, w = x.1.len().max(2)) } else { x.1.clone() };
            out.push(if run.len() == 1 { format!("{p}{}", num(&run[0])) } else { format!("{p}{}..{}", num(&run[0]), num(run.last().unwrap())) });
            run = n.into_iter().collect();
        }
    }
    out.join(",")
}

//! What changed: the working tree against a git ref, or one commit, down to the requirement lines the diff hits, and the commits
//! that are not on a base branch. Evidence of edits, not of "done". Needs git; without it these say "not checked".
use crate::report::{Analysis, Row, chars};
use crate::scan::{Project, id_key};
use indexmap::IndexMap;
use regex::Regex;
use serde::Serialize;
use std::{collections::{HashMap, HashSet}, rc::Rc, sync::LazyLock};

static HUNK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^@@ -\d+(?:,(\d+))? \+(\d+)(?:,(\d+))? @@").unwrap());
static HEADER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^@@ -\d+(?:,\d+)? \+(\d+)(?:,(\d+))? @@").unwrap());

/// The longest diff `file_diff` returns.
pub const DIFF_LINES: usize = 3000;

#[derive(Debug, Default, Clone)]
pub struct ChangeOpts {
    /// The git ref the working tree is compared with (HEAD when empty).
    pub reference: String,
    /// Explain this one commit (against its first parent) instead of the working tree.
    pub commit: String,
    /// Only what is staged (the index against `reference`).
    pub staged: bool,
    pub no_untracked: bool,
    /// Leave out changes that only touch whitespace.
    pub ignore_whitespace: bool,
    /// Also files `.aiignore`, `.spec17ignore` and `.agents` hide. Files git ignores are never listed.
    pub show_ignored: bool,
    /// Also files spec17 does not scan (kind `other`: lockfiles, UI code, …).
    pub show_all: bool,
}

impl ChangeOpts {
    fn reference(&self) -> &str {
        if self.reference.is_empty() { "HEAD" } else { &self.reference }
    }
}

/// A ref or sha that cannot be taken for a git option.
fn safe(r: &str) -> Result<&str, String> {
    if r.starts_with('-') || r.chars().any(|c| c.is_control() || c.is_whitespace()) { Err(format!("{r}: not a git ref")) } else { Ok(r) }
}

/// `git diff` (or `git show` for a commit) arguments; the caller adds paths after the final `--`.
fn diff_args(o: &ChangeOpts, extra: &[&str]) -> Result<Vec<String>, String> {
    let mut a: Vec<String> = if o.commit.is_empty() { vec!["diff".into()] } else { ["show", "--format=", "--first-parent", "-m"].map(String::from).to_vec() };
    a.extend(["--no-color", "--no-ext-diff", "-M"].map(String::from));
    a.extend(extra.iter().map(|s| s.to_string()));
    if o.commit.is_empty() {
        if o.staged {
            a.push("--cached".into());
        }
        a.push(safe(o.reference())?.into());
    } else {
        a.push(safe(&o.commit)?.into());
    }
    a.push("--".into());
    Ok(a)
}

// ───────────────────────────── hunks ─────────────────────────────

/// One `-U0` hunk: where it lands in the new file, and the lines it removes and adds.
#[derive(Debug, Clone)]
pub struct Hunk {
    pub header: String,
    pub new_start: i64,
    pub new_len: i64,
    pub removed: Vec<String>,
    pub added: Vec<String>,
}

// (SCAN-16)
/// {new path: hunks} of a `-U0` diff. A deleted file is keyed by its old path. Lines inside a hunk are counted, so a removed line that
/// starts with `-- ` is not mistaken for a file header.
pub fn parse_hunks(diff: &str) -> IndexMap<String, Vec<Hunk>> {
    let (mut out, mut old, mut new): (IndexMap<String, Vec<Hunk>>, Option<String>, Option<String>) = (IndexMap::new(), None, None);
    let (mut rem_o, mut rem_n) = (0i64, 0i64);
    let mut cur: Option<(String, usize)> = None;
    for line in diff.lines() {
        if rem_o > 0 || rem_n > 0 {
            if let Some((p, k)) = &cur {
                let h = &mut out.get_mut(p).unwrap()[*k];
                if let Some(t) = line.strip_prefix('-') && rem_o > 0 {
                    rem_o -= 1;
                    h.removed.push(t.to_string());
                    continue;
                } else if let Some(t) = line.strip_prefix('+') && rem_n > 0 {
                    rem_n -= 1;
                    h.added.push(t.to_string());
                    continue;
                } else if line.starts_with('\\') {
                    continue;
                }
            }
            (rem_o, rem_n) = (0, 0);
        }
        if line.starts_with("diff ") {
            (old, new) = (None, None);
        } else if let Some(p) = line.strip_prefix("--- ") {
            old = p.trim_end().strip_prefix("a/").map(str::to_owned);
        } else if let Some(p) = line.strip_prefix("+++ ") {
            new = p.trim_end().strip_prefix("b/").map(str::to_owned);
        } else if let Some(m) = HUNK.captures(line) && let Some(path) = new.clone().or(old.clone()) {
            let num = |i: usize, d: i64| m.get(i).map_or(d, |x| x.as_str().parse().unwrap_or(d));
            (rem_o, rem_n) = (num(1, 1), num(3, 1));
            let v = out.entry(path.clone()).or_default();
            v.push(Hunk { header: line.to_string(), new_start: num(2, 0), new_len: rem_n, removed: vec![], added: vec![] });
            cur = Some((path, v.len() - 1));
        }
    }
    out
}

// ───────────────────────────── files ─────────────────────────────

#[derive(Debug, Clone)]
pub struct FileChange {
    pub path: String,
    pub old_path: Option<String>,
    pub change: &'static str,
    pub stage: &'static str,
}

impl Project {
    fn run(&self, args: Vec<String>) -> Result<String, String> {
        self.git(&args.iter().map(String::as_str).collect::<Vec<_>>())
    }

    fn patch_hunks(&self, o: &ChangeOpts) -> Result<IndexMap<String, Vec<Hunk>>, String> {
        let mut extra = vec!["-U0"];
        if o.ignore_whitespace {
            extra.push("-w");
        }
        Ok(parse_hunks(&self.run(diff_args(o, &extra)?)?))
    }

    // (SCAN-18)
    /// Every file the diff lists, as `change` (added, modified, deleted, renamed) and `stage` (staged, unstaged, both, untracked, or
    /// committed: the change is in HEAD or in the explained commit and the working tree has nothing to add). Hidden by the ignore files
    /// unless `show_ignored`; git-ignored files never appear.
    pub fn file_changes(&self, o: &ChangeOpts) -> Result<Vec<FileChange>, String> {
        let patch = if o.ignore_whitespace { Some(self.patch_hunks(o)?) } else { None };
        self.list_changes(o, patch.as_ref())
    }

    /// `file_changes` with the `-U0` hunks already parsed (`patch`, needed for `ignore_whitespace`), so a caller that wants both runs git once.
    fn list_changes(&self, o: &ChangeOpts, patch: Option<&IndexMap<String, Vec<Hunk>>>) -> Result<Vec<FileChange>, String> {
        let names = self.run(diff_args(o, &["--name-status"])?)?;
        let kept: Option<HashSet<&String>> = patch.filter(|_| o.ignore_whitespace).map(|p| p.keys().collect());
        let set = |a: &[&str]| self.git(a).unwrap_or_default().lines().map(str::to_owned).collect::<HashSet<String>>();
        let (cached, unstaged) = if o.commit.is_empty() { (set(&["diff", "--cached", "--name-only", "HEAD", "--"]), set(&["diff", "--name-only", "--"])) } else { Default::default() };
        let mut out = vec![];
        for l in names.lines() {
            let mut it = l.split('\t');
            let (Some(code), Some(first)) = (it.next(), it.next()) else { continue };
            let (old_path, path) = match it.next() {
                Some(n) if code.starts_with('R') || code.starts_with('C') => (Some(first.to_string()), n.to_string()),
                _ => (None, first.to_string()),
            };
            let change = match code.as_bytes()[0] { b'A' | b'C' => "added", b'D' => "deleted", b'R' => "renamed", _ => "modified" };
            if change == "modified" && kept.as_ref().is_some_and(|k| !k.contains(&path)) {
                continue;
            }
            let stage = if !o.commit.is_empty() { "committed" } else {
                match (cached.contains(&path), unstaged.contains(&path)) { (true, true) => "both", (true, false) => "staged", (false, true) => "unstaged", _ => "committed" }
            };
            out.push(FileChange { path, old_path, change, stage });
        }
        if o.commit.is_empty() && !o.staged && !o.no_untracked {
            for p in self.git(&["ls-files", "--others", "--exclude-standard"])?.lines() {
                out.push(FileChange { path: p.to_string(), old_path: None, change: "added", stage: "untracked" });
            }
        }
        out.retain(|f| o.show_ignored || !self.ignored(&f.path));
        out.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(out)
    }

    /// `out` (the `-U0` hunks of the diff) plus one hunk of additions per untracked file, without what the ignore files hide.
    fn hunks(&self, o: &ChangeOpts, files: &[FileChange], mut out: IndexMap<String, Vec<Hunk>>) -> Result<IndexMap<String, Vec<Hunk>>, String> {
        for f in files.iter().filter(|f| f.stage == "untracked") {
            let added: Vec<String> = std::fs::read_to_string(self.root.join(&f.path)).map(|t| t.lines().map(str::to_owned).collect()).unwrap_or_default();
            out.insert(f.path.clone(), vec![Hunk { header: format!("@@ -0,0 +1,{} @@", added.len()), new_start: 1, new_len: added.len() as i64, removed: vec![], added }]);
        }
        out.retain(|p, _| o.show_ignored || !self.ignored(p));
        Ok(out)
    }

    // (SCAN-20)
    /// The commits of `head` (HEAD) that are not on `base` (the first of main, master, origin/main, origin/master that exists when
    /// empty), newest first. Without a base, or when `head` is on it, the last `limit` commits of `head` (`on_base`).
    pub fn history(&self, base: &str, head: &str, limit: usize) -> Result<History, String> {
        let head = if head.is_empty() { "HEAD" } else { safe(head)? };
        let exists = |r: &str| self.git(&["rev-parse", "--verify", "-q", &format!("{r}^{{commit}}")]).is_ok();
        let base = if base.is_empty() { ["main", "master", "origin/main", "origin/master"].into_iter().find(|r| exists(r)).map(str::to_owned) } else { Some(safe(base)?.to_string()) };
        let list = |range: &str| -> Result<Vec<Commit>, String> {
            let lim = limit.to_string();
            Ok(self.git(&["log", "--format=%H%x1f%h%x1f%an%x1f%aI%x1f%s", "-n", &lim, range, "--"])?.lines().filter_map(|l| {
                let f: Vec<&str> = l.split('\u{1f}').collect();
                (f.len() == 5).then(|| Commit { sha: f[0].into(), short: f[1].into(), author: f[2].into(), date: f[3].into(), subject: f[4].into() })
            }).collect())
        };
        let mut commits = match &base { Some(b) => list(&format!("{b}..{head}"))?, None => vec![] };
        let on_base = commits.is_empty();
        if on_base {
            commits = list(head)?;
        }
        let uncommitted = if head == "HEAD" { self.file_changes(&ChangeOpts::default()).map_or(0, |f| f.len()) } else { 0 };
        Ok(History { base, head: head.into(), on_base, uncommitted, commits })
    }
}

/// `YYYY-MM-DD` (UTC) of a unix time.
fn ymd(t: i64) -> String {
    let z = t.div_euclid(86400) + 719468;
    let (era, doe) = (z.div_euclid(146097), z.rem_euclid(146097));
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let (d, m) = (doy - (153 * mp + 2) / 5 + 1, if mp < 10 { mp + 3 } else { mp - 9 });
    format!("{:04}-{m:02}-{d:02}", yoe + era * 400 + i64::from(m <= 2))
}

// ───────────────────────────── what a hunk touches ─────────────────────────────

/// The last line of the `{ }` unit that starts at 1-based line `n`; `None` when there is no such unit (a language without braces).
/// Braces in string and char literals and in `//` comments do not count.
fn unit_end(ls: &[String], n: usize) -> Option<usize> {
    static LIT: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    // ponytail: per line, so a multi-line string with braces still miscounts; a real lexer if that shows up
    let lit = LIT.get_or_init(|| Regex::new(r#""(?:[^"\\]|\\.)*"|'(?:[^'\\]|\\.)'|//.*"#).unwrap());
    let (mut depth, mut opened) = (0i32, false);
    for (k, l) in ls.iter().enumerate().skip(n - 1).take(400) {
        let l = lit.replace_all(l, "");
        depth += l.matches('{').count() as i32 - l.matches('}').count() as i32;
        opened = opened || l.contains('{');
        if depth < 0 || (opened && depth == 0) {
            return Some(k + 1);
        }
    }
    None
}

/// Where a requirement is touched: `role` spec (its bullet), test (a `[ID]` tag, with the test it tags) or src (a `// (ID)` citation,
/// with the code it sits on), and what the hunk did to it.
#[derive(Debug, Clone)]
pub struct Note {
    pub id: String,
    pub role: &'static str,
    pub action: &'static str,
    pub file: String,
    /// The removed bullet, for a requirement that is gone.
    text: Option<String>,
}

/// The distinct roles (spec, test, src, in that order) of the notes of one requirement.
pub fn roles_of<'a>(notes: impl Iterator<Item = &'a Note> + Clone, id: &str) -> Vec<String> {
    ["spec", "test", "src"].into_iter().filter(|r| notes.clone().any(|n| n.id == id && n.role == *r)).map(String::from).collect()
}

/// Which of spec, test and src a file is for the scanner. A file that is gone is judged by its path.
struct Kinds<'a> {
    p: &'a Project,
    specs: HashSet<String>,
    tests: HashSet<String>,
    src: HashSet<String>,
}

impl<'a> Kinds<'a> {
    fn of(p: &'a Project) -> Self {
        Kinds { p, specs: p.spec_files().iter().cloned().collect(), tests: p.test_files().iter().cloned().collect(), src: p.src_files().iter().cloned().collect() }
    }

    fn roles(&self, f: &str) -> Vec<&'static str> {
        let gone = !self.p.root.join(f).exists();
        let under = |d: &str| d == "." || f == d || f.starts_with(&format!("{d}/"));
        let ext = f.rsplit_once('.').is_some_and(|(_, x)| self.p.cfg.ext.iter().any(|y| y == x));
        let test = gone && ext && self.p.is_test_path(f);
        [
            ("spec", self.specs.contains(f) || (gone && f.ends_with(".md") && self.p.spec_paths().into_iter().any(under))),
            ("test", self.tests.contains(f) || test),
            ("src", self.src.contains(f) || (gone && ext && !test && self.p.cfg.src.iter().any(|d| under(d)))),
        ].into_iter().filter(|(_, yes)| *yes).map(|(r, _)| r).collect()
    }
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ChangedFile {
    pub path: String,
    /// The path before a rename.
    pub old_path: Option<String>,
    /// What happened to it: added | modified | deleted | renamed
    pub change: String,
    /// Where: staged | unstaged | both (staged, then edited again) | untracked | committed (in the compared commits, nothing more in the working tree)
    pub stage: String,
    /// spec | test | src | other (a file spec17 does not scan)
    pub kind: String,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ReqNote {
    /// spec | test | src
    pub role: String,
    /// spec: added, edited, removed; test: tag added, changed, tag removed; src: citation added, changed, citation removed
    pub action: String,
    pub file: String,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ReqChange {
    pub id: String,
    /// The requirement's bullet was removed and no spec defines it now.
    pub deleted: bool,
    /// The scanner's state now; `deleted` for a removed bullet, `unknown` for an ID only tests or code mention.
    pub state: String,
    pub level: String,
    pub text: String,
    pub priority: Option<String>,
    pub effort: Option<String>,
    pub file: String,
    pub line: usize,
    /// How it is touched: spec, test, src.
    pub changed: Vec<String>,
    pub notes: Vec<ReqNote>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct Changes {
    /// The ref the working tree is compared with; empty when one commit is explained.
    pub reference: String,
    pub commit: Option<String>,
    pub staged_only: bool,
    pub untracked: bool,
    pub files: Vec<ChangedFile>,
    /// Requirements the changes touch, down to the lines: a hunk in the bullet, in a test that carries its tag or in code its
    /// citation sits on (or the whole file when the language has no braces). Not proof they changed.
    pub requirements: Vec<ReqChange>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct HunkInfo {
    /// The `@@` line of the unified diff.
    pub header: String,
    pub requirements: Vec<HunkReq>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct HunkReq {
    pub id: String,
    pub role: String,
    pub action: String,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct FileDiff {
    pub path: String,
    pub old_path: Option<String>,
    pub change: String,
    pub stage: String,
    pub diff: String,
    /// What each hunk of `diff` touches.
    pub hunks: Vec<HunkInfo>,
}

#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
pub struct Commit {
    pub sha: String,
    pub short: String,
    pub author: String,
    pub date: String,
    pub subject: String,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct History {
    /// The branch the commits are not on; none when the repo has no main or master.
    pub base: Option<String>,
    pub head: String,
    /// There are no commits beyond the base: `commits` is the recent history of `head`.
    pub on_base: bool,
    /// Files with uncommitted changes (the working tree against HEAD).
    pub uncommitted: usize,
    /// Committed, newest first.
    pub commits: Vec<Commit>,
}

impl Analysis {
    /// The new-side text of a file: the working tree, or the explained commit.
    fn new_lines(&self, o: &ChangeOpts, f: &str) -> Rc<Vec<String>> {
        if o.commit.is_empty() {
            return self.p.lines(f);
        }
        Rc::new(self.p.git(&["show", &format!("{}:{f}", o.commit)]).map(|t| crate::scan::clean(&t).lines().map(str::to_owned).collect()).unwrap_or_default())
    }

    /// The base-side text of a file, what the diff is against: the ref, or the first parent of an explained commit. Empty when the file is new.
    fn old_lines(&self, o: &ChangeOpts, f: &str) -> Vec<String> {
        let base = if o.commit.is_empty() { o.reference().to_string() } else { format!("{}^", o.commit) };
        self.p.git(&["show", &format!("{base}:{f}")]).map(|t| crate::scan::clean(&t).lines().map(str::to_owned).collect()).unwrap_or_default()
    }

    /// The text of a removed bullet line: what follows the ID, minus the level that opens it.
    fn bullet_text(&self, line: &str) -> String {
        let rest = crate::scan::LINE.head.captures(line).map_or(line, |m| m.get(2).map_or(line, |r| r.as_str()));
        let lv = crate::scan::LINE.level.find(rest).map_or("", |m| m.as_str());
        chars(&crate::scan::lead(rest, lv), 160)
    }

    fn mentions(&self, file: &str, role: &str, line: &str) -> Vec<String> {
        match role {
            "spec" => crate::scan::LINE.head.captures(line).map(|m| m[1].to_string()).filter(|i| crate::scan::is_id(i)).into_iter().collect(),
            "test" => self.p.tagged(line),
            _ => self.p.lang(file).comment_of(line).map(|c| self.p.rx.mention.captures_iter(c).map(|m| m[1].to_string()).filter(|i| crate::scan::is_id(i)).collect()).unwrap_or_default(),
        }
    }

    /// (id, first line, last line) of each requirement a line of the new side mentions: a bullet with its wrapped lines, or the
    /// tagged test / cited code unit (the whole file when it has no braces).
    fn units(&self, file: &str, role: &str, ls: &[String]) -> Vec<(String, usize, usize)> {
        let mut out = vec![];
        for (i, l) in ls.iter().enumerate() {
            let n = i + 1;
            for id in self.mentions(file, role, l) {
                let end = if role == "spec" {
                    crate::scan::block_end(ls, n)
                } else {
                    unit_end(ls, n).unwrap_or(ls.len())
                };
                out.push((id, n, end));
            }
        }
        out
    }

    // (SCAN-32)
    /// What each hunk of `file` does to the requirements it touches, per role of the file. `old` is the file on the base side: a
    /// requirement whose bullet, tagged test or cited code reads the same there once whitespace and wrapping are collapsed is not changed.
    fn hunk_notes(&self, file: &str, hs: &[Hunk], ls: &[String], old: &[String], roles: &[&'static str]) -> Vec<Vec<Note>> {
        let mut out: Vec<Vec<Note>> = vec![vec![]; hs.len()];
        // per ID, the words of all its units in order
        let sigs = |units: &[(String, usize, usize)], ls: &[String]| {
            let mut m: HashMap<String, String> = HashMap::new();
            for (id, a, b) in units {
                let e = m.entry(id.clone()).or_default();
                for w in ls[a - 1..*b].iter().flat_map(|l| l.split_whitespace()) {
                    e.push_str(w);
                    e.push(' ');
                }
                e.push('\n');
            }
            m
        };
        for &role in roles {
            let units = self.units(file, role, ls);
            let (now, before) = (sigs(&units, ls), sigs(&self.units(file, role, old), old));
            let present: HashSet<&str> = units.iter().map(|u| u.0.as_str()).collect();
            let (added, changed, removed) = match role { "spec" => ("added", "edited", "removed"), "test" => ("tag added", "changed", "tag removed"), _ => ("citation added", "changed", "citation removed") };
            for (k, h) in hs.iter().enumerate() {
                let (lo, hi) = (h.new_start, h.new_start + h.new_len.max(1) - 1);
                let gone: HashMap<String, String> = h.removed.iter().flat_map(|l| self.mentions(file, role, l).into_iter().map(|id| (id, l.trim().to_string()))).collect();
                for (id, a, b) in &units {
                    let (a, b) = (*a as i64, *b as i64);
                    // a pure deletion (no new lines) sits between two lines: it touches only a unit that spans both
                    if (h.new_len > 0 && a <= hi && b >= lo) || (h.new_len == 0 && a <= h.new_start && b > h.new_start) {
                        let fresh = h.new_len > 0 && a >= lo && a <= hi && !gone.contains_key(id);
                        if !fresh && before.get(id) == now.get(id) {
                            continue;
                        }
                        out[k].push(Note { id: id.clone(), role, action: if fresh { added } else { changed }, file: file.into(), text: None });
                    }
                }
                for (id, text) in gone.into_iter().filter(|(id, _)| !present.contains(id.as_str())) {
                    out[k].push(Note { id, role, action: removed, file: file.into(), text: Some(text) });
                }
            }
        }
        out
    }

    fn notes_of(&self, o: &ChangeOpts, files: &[FileChange], kinds: &Kinds, patch: IndexMap<String, Vec<Hunk>>) -> Result<Vec<Note>, String> {
        let mut out = vec![];
        for (f, hs) in self.p.hunks(o, files, patch)? {
            let roles = kinds.roles(&f);
            if !roles.is_empty() {
                let old = self.old_lines(o, files.iter().find(|x| x.path == f).and_then(|x| x.old_path.as_deref()).unwrap_or(&f));
                out.extend(self.hunk_notes(&f, &hs, &self.new_lines(o, &f), &old, &roles).into_iter().flatten());
            }
        }
        Ok(out)
    }

    /// The notes of a change set (what `status changed` maps to requirements).
    pub(crate) fn change_notes(&self, o: &ChangeOpts) -> Result<Vec<Note>, String> {
        let patch = self.p.patch_hunks(o)?;
        let files = self.p.list_changes(o, Some(&patch))?;
        self.notes_of(o, &files, &Kinds::of(&self.p), patch)
    }

    // (SCAN-30)
    /// Per line of `file` (index = line - 1): the commit that last changed it and its author time; `None` for a line not committed yet.
    /// By `git blame --line-porcelain`: a header is `<sha> <orig> <final> [<count>]`, the content line starts with a tab.
    fn blame_lines(&self, file: &str) -> Result<Vec<Option<(Rc<Commit>, i64)>>, String> {
        let out = self.p.git(&["blame", "--line-porcelain", "--", file])?;
        let (mut by, mut order, mut cur): (HashMap<&str, (Commit, i64)>, Vec<&str>, &str) = Default::default();
        for l in out.lines() {
            if !l.starts_with('\t') && l.len() > 40 && l.as_bytes()[40] == b' ' && l[..40].bytes().all(|b| b.is_ascii_hexdigit()) {
                cur = &l[..40];
                by.entry(cur).or_insert_with(|| (Commit { sha: cur.into(), short: cur[..7].into(), author: String::new(), date: String::new(), subject: String::new() }, 0));
                order.push(cur);
            } else if let Some((c, t)) = by.get_mut(cur) {
                if let Some(a) = l.strip_prefix("author ") {
                    c.author = a.into();
                } else if let Some(m) = l.strip_prefix("summary ") {
                    c.subject = m.into();
                } else if let Some(x) = l.strip_prefix("author-time ") {
                    *t = x.parse().unwrap_or(0);
                    c.date = ymd(*t);
                }
            }
        }
        let by: HashMap<&str, (Rc<Commit>, i64)> = by.into_iter().map(|(h, (c, t))| (h, (Rc::new(c), t))).collect();
        Ok(order.into_iter().map(|h| (!h.bytes().all(|b| b == b'0')).then(|| by[h].clone())).collect())
    }

    // (SCAN-30)
    /// {requirement ID: the newest commit that touched its lines}, by `git blame` of each spec file. A requirement with a line not
    /// committed yet, or in a file git does not know, is left out: it has no commit to review.
    pub fn blame_commits(&self) -> Result<IndexMap<String, Commit>, String> {
        let mut by_file: IndexMap<&str, Vec<&crate::scan::Spec>> = IndexMap::new();
        for s in self.s.values().filter(|s| s.mark != "ref") {
            by_file.entry(s.file.as_str()).or_default().push(s);
        }
        let mut out = IndexMap::new();
        for (file, specs) in by_file {
            let lines = self.blame_lines(file)?;
            for s in specs {
                let span: Option<Vec<_>> = lines.get(s.line - 1..s.end.min(lines.len())).unwrap_or_default().iter().map(|l| l.as_ref()).collect();
                if let Some(c) = span.and_then(|v| v.into_iter().max_by_key(|(_, t)| *t)) {
                    out.insert(s.id.clone(), (*c.0).clone());
                }
            }
        }
        Ok(out)
    }

    // (SCAN-31)
    /// {line: the commit that last changed it} for the given lines of a scanned spec, test or source file. A line not committed yet is left out.
    /// ponytail: one line is what a test reference has (its tag or name line), so an edit to the test body elsewhere is not seen.
    pub fn blame_at(&self, path: &str, lines: &[usize]) -> Result<IndexMap<usize, Commit>, String> {
        if ![self.p.spec_files(), self.p.test_files(), self.p.src_files()].iter().any(|fs| fs.iter().any(|f| f == path)) {
            return Err(format!("{path}: not a scanned file"));
        }
        let all = self.blame_lines(path)?;
        Ok(lines.iter().filter_map(|&n| Some((n, (*all.get(n.checked_sub(1)?)?.as_ref()?.0).clone()))).collect())
    }

    // (SCAN-20)
    /// `Project::history` without the commits that change no file the filters in `o` show (whitespace, ignored, unscanned), as `changes` lists them.
    pub fn history(&self, base: &str, head: &str, limit: usize, o: &ChangeOpts) -> Result<History, String> {
        let mut h = self.p.history(base, head, limit)?;
        let kinds = Kinds::of(&self.p);
        h.commits.retain(|c| {
            let o = ChangeOpts { commit: c.sha.clone(), ..o.clone() };
            self.p.file_changes(&o).is_ok_and(|fs| fs.iter().any(|f| o.show_all || !kinds.roles(&f.path).is_empty()))
        });
        Ok(h)
    }

    // (SCAN-18)
    /// What the working tree (or one commit) changed: the files, and the requirements the changed lines touch.
    pub fn changes(&self, o: &ChangeOpts) -> Result<Changes, String> {
        let patch = self.p.patch_hunks(o)?;
        let (files, kinds) = (self.p.list_changes(o, Some(&patch))?, Kinds::of(&self.p));
        let mut notes = self.notes_of(o, &files, &kinds, patch)?;
        // (SCAN-22) a changed source file that a requirement's tagged tests reach in the index: it may be affected though nothing there cites it
        if let Some(links) = &self.links {
            for f in files.iter().filter(|f| kinds.roles(&f.path).contains(&"src")) {
                let mut reached: Vec<&String> = links.iter().filter(|(_, fs)| fs.contains(&f.path)).map(|(id, _)| id).collect();
                reached.sort_by_key(|i| id_key(i));
                for id in reached {
                    if !notes.iter().any(|n| n.id == *id && n.file == f.path) {
                        notes.push(Note { id: id.clone(), role: "src", action: "reached by a test", file: f.path.clone(), text: None });
                    }
                }
            }
        }
        let mut ids: Vec<&str> = notes.iter().map(|n| n.id.as_str()).collect::<HashSet<_>>().into_iter().collect();
        ids.sort_by_key(|i| id_key(i));
        let requirements = ids.into_iter().map(|id| {
            let mine: Vec<&Note> = notes.iter().filter(|n| n.id == id).collect();
            let mut seen = HashSet::new();
            let notes = mine.iter().filter(|n| seen.insert((n.role, n.action, &n.file))).map(|n| ReqNote { role: n.role.into(), action: n.action.into(), file: n.file.clone() }).collect();
            let changed = roles_of(mine.iter().copied(), id);
            if self.s.contains_key(id) {
                let Row { state, level, text, priority, effort, file, line, .. } = self.requirement_row(id);
                return ReqChange { id: id.into(), deleted: false, state, level, text, priority, effort, file, line, changed, notes };
            }
            let gone = mine.iter().find(|n| n.role == "spec" && n.action == "removed");
            ReqChange {
                id: id.into(), deleted: gone.is_some(), state: if gone.is_some() { "deleted" } else { "unknown" }.into(), level: String::new(),
                text: gone.and_then(|n| n.text.as_deref()).map(|t| self.bullet_text(t)).unwrap_or_default(), priority: None, effort: None, file: gone.map(|n| n.file.clone()).unwrap_or_default(), line: 0, changed, notes,
            }
        }).collect();
        let files = files.into_iter().filter_map(|f| {
            let r = kinds.roles(&f.path);
            let kind = ["spec", "test", "src"].into_iter().find(|k| r.contains(k)).unwrap_or("other");
            (o.show_all || kind != "other").then(|| ChangedFile { path: f.path, old_path: f.old_path, change: f.change.into(), stage: f.stage.into(), kind: kind.into() })
        }).collect();
        Ok(Changes { reference: if o.commit.is_empty() { o.reference().into() } else { String::new() }, commit: (!o.commit.is_empty()).then(|| o.commit.clone()), staged_only: o.staged && o.commit.is_empty(), untracked: o.commit.is_empty() && !o.staged && !o.no_untracked, files, requirements })
    }

    // (SCAN-19)
    /// The unified diff of one file `file_changes` lists (a rename shows both paths), cut at `DIFF_LINES` lines, and what each of its
    /// hunks touches. Any other path is refused, so the diff cannot be used to read other files.
    pub fn file_diff(&self, path: &str, o: &ChangeOpts) -> Result<FileDiff, String> {
        let files = self.p.file_changes(o)?;
        let kinds = Kinds::of(&self.p);
        let f = files.iter().find(|f| f.path == path && (o.show_all || !kinds.roles(path).is_empty())).ok_or_else(|| format!("{path}: not a changed file"))?.clone();
        let paths: Vec<String> = f.old_path.iter().cloned().chain([f.path.clone()]).collect();
        let text = if f.stage == "untracked" {
            match std::fs::read_to_string(self.p.root.join(path)) {
                Ok(t) => format!("--- /dev/null\n+++ b/{path}\n@@ -0,0 +1,{} @@\n{}", t.lines().count(), t.lines().map(|l| format!("+{l}\n")).collect::<String>()),
                Err(_) => format!("Binary or unreadable file {path}\n"),
            }
        } else {
            let mut a = diff_args(o, if o.ignore_whitespace { &["-w"] } else { &[] })?;
            a.extend(paths.clone());
            self.p.run(a)?
        };
        // the same file at -U0 says what each hunk touches; a context hunk takes the notes of the -U0 hunks that start inside it
        let mut z = diff_args(o, if o.ignore_whitespace { &["-U0", "-w"] } else { &["-U0"] })?;
        z.extend(paths);
        let zero = if f.stage == "untracked" { self.p.hunks(o, &files, IndexMap::new())?.shift_remove(path).unwrap_or_default() } else { parse_hunks(&self.p.run(z)?).shift_remove(path).unwrap_or_default() };
        let roles = kinds.roles(path);
        let notes = if roles.is_empty() { vec![vec![]; zero.len()] } else { self.hunk_notes(path, &zero, &self.new_lines(o, path), &self.old_lines(o, f.old_path.as_deref().unwrap_or(path)), &roles) };
        let mut hunks: Vec<(i64, i64, HunkInfo)> = text.lines().filter(|l| l.starts_with("@@")).filter_map(|l| {
            let m = HEADER.captures(l)?;
            let (s, n): (i64, i64) = (m[1].parse().ok()?, m.get(2).map_or(1, |x| x.as_str().parse().unwrap_or(1)));
            Some((s, s + n.max(1), HunkInfo { header: l.to_string(), requirements: vec![] }))
        }).collect();
        for (h, ns) in zero.iter().zip(notes) {
            if let Some((.., info)) = hunks.iter_mut().find(|(s, e, _)| h.new_start >= *s && h.new_start <= *e) {
                for n in ns {
                    if !info.requirements.iter().any(|r| r.id == n.id && r.role == n.role && r.action == n.action) {
                        info.requirements.push(HunkReq { id: n.id, role: n.role.into(), action: n.action.into() });
                    }
                }
            }
        }
        let more = text.lines().count().saturating_sub(DIFF_LINES);
        let mut diff: String = text.lines().take(DIFF_LINES).map(|l| format!("{l}\n")).collect();
        if more > 0 {
            diff += &format!("… {more} more lines\n");
        }
        Ok(FileDiff { path: f.path, old_path: f.old_path, change: f.change.into(), stage: f.stage.into(), diff, hunks: hunks.into_iter().map(|h| h.2).collect() })
    }
}

// ───────────────────────────── text ─────────────────────────────

impl Changes {
    pub fn text(&self) -> String {
        let what = match &self.commit {
            Some(c) => format!("commit {c}"),
            None => format!("{} ({})", self.reference, if self.staged_only { "staged" } else if self.untracked { "staged, unstaged and untracked" } else { "staged and unstaged" }),
        };
        let mut out = format!("changes in {what}: {} files, {} requirements touched\n", self.files.len(), self.requirements.len());
        for f in &self.files {
            out += &format!("  {:<8} {:<9} {:<5} {}{}\n", f.change, f.stage, f.kind, f.path, f.old_path.as_ref().map_or(String::new(), |o| format!(" (from {o})")));
        }
        for r in &self.requirements {
            out += &format!("{} {:<8} {}:{} [changed: {}]  {}\n", r.id, r.state, r.file, r.line, r.changed.join(","), chars(&r.text, 100));
            for n in &r.notes {
                out += &format!("    {} {}: {}\n", n.role, n.action, n.file);
            }
        }
        out += "Evidence, not truth: a changed line may touch a requirement; it does not say the requirement is done.\n";
        out
    }
}

impl FileDiff {
    /// The diff, with a line under each hunk header naming the requirements it touches.
    pub fn text(&self) -> String {
        let mut out = String::new();
        for l in self.diff.lines() {
            out += l;
            out.push('\n');
            if let Some(h) = self.hunks.iter().find(|h| h.header == l && !h.requirements.is_empty()) {
                out += &format!("  ↳ {}\n", h.requirements.iter().map(|r| format!("{} {} {}", r.id, r.role, r.action)).collect::<Vec<_>>().join(" · "));
            }
        }
        out
    }
}

impl History {
    pub fn text(&self) -> String {
        let mut out = match (&self.base, self.on_base) {
            (Some(b), false) => format!("{} commits on {} that are not on {b}; {} files uncommitted\n", self.commits.len(), self.head, self.uncommitted),
            (b, _) => format!("{} is on {}: recent commits; {} files uncommitted\n", self.head, b.as_deref().unwrap_or("(no main or master found)"), self.uncommitted),
        };
        for c in &self.commits {
            out += &format!("{} {} {}  {}\n", c.short, c.date.get(..10).unwrap_or(&c.date), c.author, c.subject);
        }
        out
    }
}

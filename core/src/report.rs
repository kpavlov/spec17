//! Analysis of a repo and its three views: the whole-project `Report`, the per-requirement `Status`, one requirement's `Trace`.
//! Each view is a serde struct (for `--json`, the API and MCP) with a `text()` that matches the Python scripts line for line.
use crate::codegraph::Graph;
use crate::changes::{ChangeOpts, roles_of};
use crate::coverage::{OverlapRow, Uncovered};
use crate::scan::{Issue, Project, Ref, Refs, Spec, id_key, ranges, sorted_ids, stem};
use indexmap::IndexMap;
use serde::Serialize;
use std::{
    collections::{BTreeSet, HashMap, HashSet},
    path::Path,
};

pub type Links = HashMap<String, BTreeSet<String>>;

pub struct Analysis {
    pub p: Project,
    pub s: IndexMap<String, Spec>,
    pub t: Refs,
    pub c: Refs,
    pub issues: Vec<Issue>,
    pub state: HashMap<String, String>,
    pub graph: Option<Graph>,
    /// Why the CodeGraph index is not used (schema drift, unreadable); `None` when it is used or absent.
    pub graph_note: Option<String>,
    pub links: Option<Links>,
    /// Who links to each ID with `[ID](path.md)`.
    pub linked: Refs,
}

pub fn chars(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

impl Analysis {
    pub fn load(root: Option<&Path>) -> Result<Analysis, String> {
        let mut p = Project::open(root)?;
        let mut issues = vec![];
        let mut s = p.specs(&mut issues);
        let (docs, linked) = p.docs(&s, &mut issues);
        s.extend(docs.into_iter().map(|d| (d.id.clone(), d)));
        p.check_refs(&s, &mut issues);
        let (t, c) = (p.tests(), p.sources());
        let state = s.iter().map(|(i, sp)| (i.clone(), if !sp.mark.is_empty() { sp.mark.clone() } else if t.contains_key(i) { "tested".into() } else if c.contains_key(i) { "src-only".into() } else { "none".into() })).collect();
        let (graph, graph_note) = match p.take_graph() {
            Ok(g) => (g, None),
            Err(e) => (None, Some(e.to_string())),
        };
        let mut a = Analysis { p, s, t, c, issues, state, graph, graph_note, links: None, linked };
        if let Some(g) = &a.graph {
            match g.code_links(&a.t, &|f| a.p.is_src_path(f) && !a.p.is_test_only(f), &|f| a.p.is_test_path(f)) {
                Ok(l) => a.links = Some(l),
                Err(e) => a.graph_note = Some(e.to_string()),
            }
        }
        Ok(a)
    }

    /// `core/src/`: the first src dir with a trailing slash (empty for `.`).
    pub fn app(&self) -> String {
        match self.p.cfg.src.first().map(String::as_str) {
            None | Some(".") => String::new(),
            Some(d) => format!("{d}/"),
        }
    }

    /// Evidence for a check-marked requirement: NO-TEST is strong; NO-LINK means its tests reach no app code in the graph; unchecked = no index.
    pub fn verdict(&self, i: &str) -> &'static str {
        if !self.t.contains_key(i) {
            "NO-TEST"
        } else if let Some(l) = &self.links {
            if l.get(i).is_some_and(|f| !f.is_empty()) { "ok" } else { "NO-LINK" }
        } else {
            "unchecked"
        }
    }

    pub fn built(&self, i: &str) -> bool {
        self.state[i] == "later" && (self.t.contains_key(i) || self.c.contains_key(i))
    }

    fn per_spec(&self, ids: &[String]) -> Vec<String> {
        let mut by: std::collections::BTreeMap<&str, Vec<&String>> = Default::default();
        for i in ids {
            by.entry(self.s[i].file.as_str()).or_default().push(i);
        }
        by.into_iter().map(|(f, v)| format!("  {}: {}", stem(f), ranges(v))).collect()
    }

    pub(crate) fn requirement_row(&self, i: &str) -> Row {
        let s = &self.s[i];
        Row {
            id: i.to_string(),
            state: self.state[i].clone(),
            file: s.file.clone(),
            line: s.line,
            level: s.level.clone(),
            text: s.text.clone(),
            priority: s.priority.clone(),
            effort: s.effort.clone(),
            references: self.p.rx.mention.captures_iter(&s.text).map(|m| m[1].to_string()).filter(|id| self.s.contains_key(id)).collect::<BTreeSet<_>>().into_iter().collect(), // (WEB-47)
            mark: s.mark.clone(),
            done: s.done,
            verdict: self.verdict(i).to_string(),
            built: self.built(i),
            changed: vec![],
            issues: self.issues.iter().filter(|x| x.id == i).map(|x| x.msg.clone()).collect(),
            tests: self.t.get(i).cloned().unwrap_or_default(),
            cites: self.c.get(i).cloned().unwrap_or_default(),
            graph: self.links.as_ref().and_then(|l| l.get(i)).map(|f| { let app = self.app(); f.iter().map(|x| x.strip_prefix(&app).unwrap_or(x).to_string()).collect() }).unwrap_or_default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
pub struct Row {
    pub id: String,
    pub state: String,
    pub file: String,
    pub line: usize,
    pub level: String,
    pub text: String,
    pub priority: Option<String>,
    pub effort: Option<String>,
    pub references: Vec<String>,
    pub mark: String,
    pub done: bool,
    pub verdict: String,
    pub built: bool,
    pub changed: Vec<String>,
    pub issues: Vec<String>,
    pub tests: Vec<Ref>,
    pub cites: Vec<Ref>,
    pub graph: Vec<String>,
}

// ───────────────────────────── Report ─────────────────────────────

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct Unproven {
    pub id: String,
    pub verdict: String,
    pub file: String,
    pub line: usize,
    pub text: String,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct TestProblem {
    pub id: String,
    pub file: String,
    pub line: usize,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct Report {
    pub spec_files: usize,
    pub requirements: usize,
    pub due: usize,
    pub tested: usize,
    pub tested_pct: usize,
    pub src_only: usize,
    pub none: usize,
    pub manual: usize,
    pub later: usize,
    /// Reference docs (linked markdown, decision records): never due.
    pub refs: usize,
    pub retired: usize,
    pub problems: IndexMap<String, usize>,
    pub total_problems: usize,
    pub index: bool,
    pub index_note: Option<String>,
    pub unimplemented: Vec<String>,
    pub untested: Vec<String>,
    pub unproven: Vec<Unproven>,
    pub built: Vec<String>,
    pub invalid_tests: IndexMap<String, Vec<TestProblem>>,
    /// IDs tagged in comments attached to detected tests; evidence, not an invalidity.
    pub comment_tagged: Vec<TestProblem>,
    pub spec_issues: Vec<String>,
    pub dangling_tests: Vec<String>,
    pub dangling_sources: Vec<String>,
    pub overlaps: Vec<OverlapRow>,
    pub uncovered: Option<Uncovered>,
}

impl Analysis {
    pub fn report(&self) -> Report {
        let by = |f: &dyn Fn(&str) -> bool| sorted_ids(self.s.keys().filter(|i| f(i)));
        let none = by(&|i| self.state[i] == "none");
        let src_only = by(&|i| self.state[i] == "src-only");
        let n_tested = self.s.keys().filter(|i| self.state[*i] == "tested").count();
        let due = n_tested + none.len() + src_only.len();
        let refs = self.s.keys().filter(|i| self.state[*i] == "ref").count();
        let built = by(&|i| self.built(i));
        let unproven: Vec<Unproven> = by(&|_| true).into_iter().filter(|i| self.s[i].done && matches!(self.verdict(i), "NO-TEST" | "NO-LINK")).map(|i| {
            let s = &self.s[&i];
            Unproven { verdict: self.verdict(&i).into(), file: s.file.clone(), line: s.line, text: s.text.clone(), id: i }
        }).collect();
        let mut invalid_tests: IndexMap<String, Vec<TestProblem>> = self.p.test_problems(&self.s, &self.t, self.graph.as_ref()).into_iter().map(|(k, v)| (k.to_string(), v.into_iter().map(|(id, file, line)| TestProblem { id, file, line }).collect())).collect();
        // (SCAN-36) Comment tags are useful evidence, not problems.
        let comment_tagged = invalid_tests.shift_remove("comment-tagged").unwrap_or_default();
        let (dt, ds) = (self.p.dangling(&self.s, &self.t), self.p.dangling(&self.s, &self.c));
        let overlaps: Vec<OverlapRow> = self.overlap("", false).ok().flatten().unwrap_or_default().into_iter().filter(|r| !r.pairs.iter().all(|p| p.2)).collect();
        let uncovered = self.uncovered("", 5, 10).ok().flatten();
        let tests_bad: usize = invalid_tests.values().map(Vec::len).sum();
        let problems: IndexMap<String, usize> = [
            ("unimplemented", none.len()),
            ("untested", src_only.len()),
            ("unproven_done", unproven.len()),
            ("later_but_built", built.len()),
            ("invalid_tests", tests_bad),
            ("spec_issues", self.issues.len()),
            ("dangling_citations", ds.len() + dt.len()),
            ("specs_to_compare", overlaps.len()),
        ].into_iter().map(|(k, v)| (k.to_string(), v)).collect();
        Report {
            spec_files: self.s.values().filter(|s| s.mark != "ref").map(|s| &s.file).collect::<HashSet<_>>().len(),
            requirements: self.s.len() - refs,
            due,
            tested: n_tested,
            tested_pct: if due > 0 { 100 * n_tested / due } else { 0 },
            src_only: src_only.len(),
            none: none.len(),
            manual: self.s.keys().filter(|i| self.state[*i] == "manual").count(),
            later: self.s.keys().filter(|i| self.state[*i] == "later").count(),
            refs,
            retired: self.p.retired().len(),
            total_problems: problems.values().sum(),
            problems,
            index: self.graph.is_some() && self.links.is_some(),
            index_note: self.graph_note.clone(),
            unimplemented: none,
            untested: src_only,
            unproven,
            built,
            invalid_tests,
            comment_tagged,
            spec_issues: self.issues.iter().map(Issue::to_string).collect(),
            dangling_tests: dt,
            dangling_sources: ds,
            overlaps,
            uncovered,
        }
    }
}

impl Report {
    // (SCAN-05)
    /// Plain text, identical on every run for the same tree. `limit` lines per problem section.
    pub fn text(&self, a: &Analysis, limit: usize) -> String {
        let mut o = String::new();
        let section = |o: &mut String, title: &str, count: usize, lines: Vec<String>, hint: &str| {
            if count > 0 { o.push_str(&format!("\n[{title}] {count}\n")) } else { o.push_str(&format!("\n[{title}] none\n")) }
            for x in lines.iter().take(limit) {
                o.push_str(x);
                o.push('\n');
            }
            if lines.len() > limit {
                o.push_str(&format!("  … {} more{}\n", lines.len() - limit, if hint.is_empty() { String::new() } else { format!(" ({hint})") }));
            } else if count > 0 && !hint.is_empty() {
                o.push_str(&format!("  -> {hint}\n"));
            }
        };
        o.push_str("SDD STATUS\n");
        o.push_str(&format!(
            "specs: {} files, {} requirements | due {}: tested {} ({}%), src-only {}, none {} | manual {} | later {}{} | retired {}\n",
            self.spec_files, self.requirements, self.due, self.tested, self.tested_pct, self.src_only, self.none, self.manual, self.later,
            if self.refs > 0 { format!(" | ref {}", self.refs) } else { String::new() }, self.retired
        ));
        let probs: Vec<String> = self.problems.iter().map(|(k, v)| format!("{} {v}", k.replace('_', " "))).collect();
        o.push_str(&format!("problems: {} | total {}\n", probs.join(" | "), self.total_problems));
        // (SCAN-28)
        if self.requirements + self.refs == 0 {
            o.push_str("note: no specs found, so there is nothing to measure yet: draft requirements with the user first (`spec17 doctor` says where spec17 looks and what a requirement line is)\n");
        }
        if !self.index {
            o.push_str("note: no CodeGraph index, so test-to-code links and uncovered code are not checked\n");
            if let Some(n) = &self.index_note {
                o.push_str(&format!("note: {n}\n"));
            }
        }
        section(&mut o, "unimplemented: due, no test and no code", self.unimplemented.len(), a.per_spec(&self.unimplemented), "spec17 status --missing");
        section(&mut o, "untested: due, cited in code but no test", self.untested.len(), a.per_spec(&self.untested), "write the [ID] tests; spec17 status --missing");
        section(&mut o, "check-marked done without proof", self.unproven.len(),
            self.unproven.iter().map(|u| format!("  {} {} {}:{}  {}", u.id, u.verdict, u.file, u.line, chars(&u.text, 70))).collect(),
            "NO-TEST = not done unless proven; NO-LINK = verify; spec17 status --unproven --effort high");
        section(&mut o, "marked later but already has tests or code", self.built.len(), a.per_spec(&self.built), "assess: spec17 status --built, SKILL.md effort modes");
        let bt: Vec<String> = self.invalid_tests.iter().filter(|(_, v)| !v.is_empty()).map(|(k, v)| {
            let head: Vec<String> = v.iter().take(6).map(|p| format!("{} ({}:{})", p.id, p.file, p.line)).collect();
            format!("  {k}: {}{}", head.join(", "), if v.len() > 6 { format!(", … {} more", v.len() - 6) } else { String::new() })
        }).collect();
        let tests_bad: usize = self.invalid_tests.values().map(Vec::len).sum();
        section(&mut o, "invalid tests", tests_bad, bt, "unknown/retired = fix the tag; comment-only = no test carries it; disabled = switched off; no-assert = no direct assertion, verify");
        section(&mut o, "comment-tagged", self.comment_tagged.len(), self.comment_tagged.iter().map(|p| format!("  {} ({}:{})", p.id, p.file, p.line)).collect(), "comment attached to a detected test; verify the evidence");
        let app = a.app();
        section(&mut o, "specs to compare: code covered by 2+ requirements that do not cite each other", self.overlaps.len(),
            self.overlaps.iter().map(|r| format!("  {}:{} {}  [{}]", r.file.strip_prefix(&app).unwrap_or(&r.file), r.line, r.name, r.ids.join(", "))).collect(),
            "spec17 status --overlap; conflicting or complementary? see SKILL.md");
        section(&mut o, "spec issues", self.spec_issues.len(), self.spec_issues.iter().map(|x| format!("  {x}")).collect(), "fix the spec bullet");
        let dl: Vec<String> = [("tests", &self.dangling_tests), ("sources", &self.dangling_sources)].into_iter().filter(|(_, v)| !v.is_empty()).map(|(k, v)| format!("  {k}: {}", ranges(v))).collect();
        section(&mut o, "dangling citations (ID in no spec)", self.dangling_sources.len() + self.dangling_tests.len(), dl, "repoint to a live ID or remove");
        match &self.uncovered {
            Some(u) => {
                let t = u.text();
                let mut ls = t.lines();
                o.push_str(&format!("\n[code with no requirement] {}\n", ls.next().unwrap_or("")));
                for x in ls.take(5).filter(|x| x.starts_with("  ")) {
                    o.push_str(&chars(x, 140));
                    o.push('\n');
                }
            }
            None => o.push_str("\n[code with no requirement] not checked (no CodeGraph index)\n"),
        }
        o.push_str("  -> spec17 status --uncovered [PATH]; triage per SKILL.md before writing requirements\n");
        o
    }
}

// ───────────────────────────── Status ─────────────────────────────

#[derive(Debug, Default, Clone)]
pub struct StatusOpts {
    pub spec: Option<String>,
    pub prefix: Option<String>,
    pub all: bool,
    pub missing: bool,
    /// Git ref to diff the working tree against; `Some` turns the filter on.
    pub changed: Option<String>,
    /// With `changed` (HEAD when it is not given): only what is staged.
    pub staged: bool,
    /// With `changed`: leave untracked files out (they count by default).
    pub no_untracked: bool,
    /// With `changed`: leave out changes that only touch whitespace.
    pub ignore_whitespace: bool,
    pub built: bool,
    pub done: bool,
    pub unproven: bool,
    pub issues: bool,
    /// low | medium | high
    pub effort: String,
}

impl StatusOpts {
    /// The ref to diff against: `changed`, or HEAD when only `staged` is asked for.
    fn reference(&self) -> Option<&str> {
        self.changed.as_deref().or(self.staged.then_some("HEAD"))
    }
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct SpecLine {
    pub file: String,
    pub tested: usize,
    pub due: usize,
    pub src_only: Vec<String>,
    pub none: Vec<String>,
    pub manual: usize,
    pub later: usize,
    pub refs: usize,
    pub later_built: usize,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct Status {
    pub listing: bool,
    pub effort: String,
    pub rows: Vec<Row>,
    pub by_spec: Vec<SpecLine>,
    pub tested: usize,
    pub due: usize,
    pub src_only: usize,
    pub none: usize,
    pub manual: usize,
    pub later: usize,
    pub refs: usize,
    pub later_built: usize,
    pub marked: usize,
    pub marked_no_test: Vec<String>,
    pub marked_no_link: Vec<String>,
    pub links_checked: bool,
    pub issues: Vec<String>,
    /// Issues about bullets that have no requirement entry (no MUST/SHOULD/MAY).
    pub uncounted: Vec<String>,
    pub dangling_tests: Vec<String>,
    pub dangling_sources: Vec<String>,
}

impl Analysis {
    pub fn status(&self, o: &StatusOpts) -> Result<Status, String> {
        let mut ids: Vec<String> = self.s.iter().filter(|(i, s)| !(o.spec.as_ref().is_some_and(|x| !s.file.contains(x.as_str())) || o.prefix.as_ref().is_some_and(|x| !i.starts_with(&format!("{x}-"))))).map(|(i, _)| i.clone()).collect();
        if ids.is_empty() && (o.spec.is_some() || o.prefix.is_some()) {
            return Err("spec17: no requirement matches --spec/--prefix".into());
        }
        ids.sort_by_key(|i| id_key(i));
        let listing = o.all || o.missing || o.issues || o.reference().is_some() || o.built || o.done || o.unproven;
        let issue_ids: HashSet<&str> = self.issues.iter().map(|x| x.id.as_str()).collect();
        let notes = match o.reference() {
            Some(r) => self.change_notes(&ChangeOpts { reference: r.into(), staged: o.staged, no_untracked: o.no_untracked, ignore_whitespace: o.ignore_whitespace, ..Default::default() })?,
            None => vec![],
        };
        let mut rows = vec![];
        if listing {
            for i in &ids {
                let verdict = self.verdict(i);
                let mut row = self.requirement_row(i);
                if o.missing && !matches!(self.state[i].as_str(), "src-only" | "none") { continue; }
                if o.issues && !issue_ids.contains(i.as_str()) { continue; }
                if o.built && !row.built { continue; }
                if (o.done || o.unproven) && !row.done { continue; }
                if o.unproven && matches!(verdict, "ok" | "unchecked") { continue; }
                if o.reference().is_some() {
                    row.changed = roles_of(notes.iter(), i);
                    if row.changed.is_empty() { continue; }
                }
                rows.push(row);
            }
        }
        let count = |ids: &[String], k: &str| ids.iter().filter(|i| self.state[*i] == k).count();
        let mut by: IndexMap<&str, Vec<&String>> = IndexMap::new();
        for i in &ids {
            by.entry(self.s[i].file.as_str()).or_default().push(i);
        }
        let mut by_spec: Vec<SpecLine> = by.into_iter().map(|(f, v)| {
            let owned: Vec<String> = v.iter().map(|x| x.to_string()).collect();
            let of = |k: &str| owned.iter().filter(|i| self.state[*i] == k).cloned().collect::<Vec<_>>();
            let (manual, later, refs) = (of("manual").len(), of("later"), of("ref").len());
            SpecLine { file: f.to_string(), tested: of("tested").len(), due: owned.len() - manual - later.len() - refs, src_only: of("src-only"), none: of("none"), manual, later: later.len(), refs, later_built: later.iter().filter(|i| self.built(i)).count() }
        }).collect();
        by_spec.sort_by(|a, b| a.file.cmp(&b.file));
        let (manual, later, refs) = (count(&ids, "manual"), count(&ids, "later"), count(&ids, "ref"));
        let marked: Vec<&String> = ids.iter().filter(|i| self.s[*i].done).collect();
        Ok(Status {
            listing,
            effort: if o.effort.is_empty() { "low".into() } else { o.effort.clone() },
            rows,
            by_spec,
            tested: count(&ids, "tested"),
            due: ids.len() - manual - later - refs,
            src_only: count(&ids, "src-only"),
            none: count(&ids, "none"),
            manual,
            later,
            refs,
            later_built: ids.iter().filter(|i| self.built(i)).count(),
            marked: marked.len(),
            marked_no_test: marked.iter().filter(|i| self.verdict(i) == "NO-TEST").map(|i| i.to_string()).collect(),
            marked_no_link: marked.iter().filter(|i| self.verdict(i) == "NO-LINK").map(|i| i.to_string()).collect(),
            links_checked: self.links.is_some(),
            issues: self.issues.iter().map(Issue::to_string).collect(),
            uncounted: self.issues.iter().filter(|x| x.uncounted && !self.s.contains_key(&x.id)).map(Issue::to_string).collect(),
            dangling_tests: self.p.dangling(&self.s, &self.t),
            dangling_sources: self.p.dangling(&self.s, &self.c),
        })
    }
}

impl Status {
    pub fn text(&self, o: &StatusOpts) -> String {
        let mut out = String::new();
        let high = self.effort == "high";
        if self.listing {
            for r in &self.rows {
                let mut flags = String::new();
                if !r.changed.is_empty() { flags += &format!(" [changed: {}]", r.changed.join(",")); }
                if r.built { flags += &format!(" [built?: {} tests, {} src]", r.tests.len(), r.cites.len()); }
                if r.done { flags += &format!(" [DONE: {}]", r.verdict); }
                if !r.issues.is_empty() { flags += " [issue]"; }
                let text = if high { r.text.clone() } else { chars(&r.text, 100) };
                out += &format!("{} {:<8} {}:{}{}  {}\n", r.id, r.state, r.file, r.line, flags, text);
                if self.effort != "low" {
                    for t in r.tests.iter().take(5) { out += &format!("    test: {}:{}\n", t.file, t.line); }
                    for c in r.cites.iter().take(5) { out += &format!("    cite: {}:{}\n", c.file, c.line); }
                    if !r.graph.is_empty() { out += &format!("    graph: tests reach {}\n", r.graph.iter().take(8).cloned().collect::<Vec<_>>().join(", ")); }
                }
                if o.issues || o.reference().is_some() {
                    for x in &r.issues { out += &format!("    issue: {x}\n"); }
                }
            }
            if o.issues {
                for x in &self.uncounted {
                    let (id, rest) = x.split_once(' ').unwrap_or((x, ""));
                    out += &format!("{id} (not counted)  issue: {rest}\n");
                }
            }
        } else {
            for l in &self.by_spec {
                let name = stem(&l.file);
                let mut line = format!("{name:<28} {}/{} tested", l.tested, l.due);
                if !l.src_only.is_empty() { line += &format!(" | src-only: {}", ranges(&l.src_only)); }
                if !l.none.is_empty() { line += &format!(" | none: {}", ranges(&l.none)); }
                if l.manual > 0 { line += &format!(" | manual: {}", l.manual); }
                if l.later > 0 {
                    line += &format!(" | later: {}", l.later);
                    if l.later_built > 0 { line += &format!(" ({} with tests/code)", l.later_built); }
                }
                out += &line;
                out.push('\n');
            }
        }
        out += &format!("TOTAL {}/{} tested ({}%) | src-only {} | none {} | manual {} | later {} ({} with tests/code)\n",
            self.tested, self.due, if self.due > 0 { 100 * self.tested / self.due } else { 0 }, self.src_only, self.none, self.manual, self.later, self.later_built);
        if self.marked > 0 {
            out += &format!("check-marked done: {} | NO TEST: {}", self.marked, self.marked_no_test.len());
            if !self.marked_no_test.is_empty() { out += &format!(" ({})", ranges(&self.marked_no_test)); }
            out += &format!(" | tests not linked to code: {}", self.marked_no_link.len());
            if !self.marked_no_link.is_empty() { out += &format!(" ({})", ranges(&self.marked_no_link)); }
            if !self.links_checked { out += " | no CodeGraph index, links unchecked"; }
            out.push('\n');
        }
        if !self.listing {
            if !self.issues.is_empty() {
                out += &format!("spec issues ({}):\n", self.issues.len());
                for x in self.issues.iter().take(10) { out += &format!("  {x}\n"); }
                if self.issues.len() > 10 { out += &format!("  … {} more (--issues)\n", self.issues.len() - 10); }
            }
            if self.src_only > 0 || self.none > 0 {
                out += "next: --missing lists them; spec17 trace <ID> shows tests, citations and related code\n";
            }
        }
        for (label, d) in [("tests", &self.dangling_tests), ("sources", &self.dangling_sources)] {
            if !d.is_empty() { out += &format!("{label} cite IDs not in specs: {}\n", ranges(d)); }
        }
        out
    }
}

// ───────────────────────────── Trace ─────────────────────────────

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct SourceCite {
    #[serde(flatten)]
    pub r: Ref,
    pub decl: String,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct Related {
    pub file: String,
    pub types: Vec<String>,
    /// The file also cites the ID.
    pub cites: bool,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct Trace {
    pub id: String,
    pub spec: Option<Spec>,
    pub cites: Vec<String>,
    pub tests: Vec<Ref>,
    pub source_cites: Vec<SourceCite>,
    /// Markdown lines that link the ID with `[ID](path.md)`.
    pub linked_from: Vec<Ref>,
    pub related: Vec<Related>,
    pub related_total: usize,
    /// `codegraph explore` query to read the real flow.
    pub query: String,
}

/// A declaration line that declares a type, whatever the language: what `trace` lists as related code.
static TYPE_KEYWORD: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| regex::Regex::new(r"\b(?:struct|class|enum|trait|interface|protocol|actor|record|object)\b").unwrap());

impl Analysis {
    // (SCAN-08)
    /// `None` when the ID is in no spec, test or source.
    pub fn trace(&self, id: &str) -> Option<Trace> {
        let want = id.to_uppercase();
        let spec = self.s.get(&want).cloned();
        if spec.is_none() && !self.t.contains_key(&want) && !self.c.contains_key(&want) && !self.linked.contains_key(&want) {
            return None;
        }
        let cites: Vec<String> = spec.as_ref().map(|s| {
            let set: BTreeSet<String> = self.p.rx.mention.captures_iter(&s.text).map(|m| m[1].to_string()).filter(|x| *x != want).collect();
            set.into_iter().collect()
        }).unwrap_or_default();
        let tests = self.t.get(&want).cloned().unwrap_or_default();
        let mut idents: HashSet<String> = HashSet::new();
        for r in &tests {
            for line in crate::scan::body(&self.p.lines(&r.file), r.line, 200) {
                idents.extend(crate::scan::WORD.find_iter(line).map(|m| m.as_str().to_string()));
            }
        }
        let (mut cited, mut names) = (HashSet::new(), BTreeSet::new());
        let mut source_cites = vec![];
        for r in self.c.get(&want).cloned().unwrap_or_default() {
            cited.insert(r.file.clone());
            let decl = self.p.enclosing(&r.file, r.line);
            if !decl.is_empty() { names.insert(decl.clone()); }
            source_cites.push(SourceCite { r, decl });
        }
        let mut defs: IndexMap<String, Vec<String>> = IndexMap::new();
        for f in self.p.src_files() {
            let (mut types, decl) = (BTreeSet::new(), &self.p.lang(f).decl);
            for line in self.p.lines(f).iter() {
                if let Some(m) = decl.captures(line).and_then(|c| c.get(1)) && idents.contains(m.as_str()) && TYPE_KEYWORD.is_match(line) {
                    types.insert(m.as_str().to_string());
                }
            }
            if !types.is_empty() {
                names.extend(types.iter().cloned());
                defs.insert(f.clone(), types.into_iter().collect());
            }
        }
        let mut order: Vec<&String> = defs.keys().collect();
        order.sort_by_key(|r| (!cited.contains(*r), (*r).clone()));
        let related = order.iter().take(12).map(|r| Related { file: (*r).clone(), types: defs[*r].clone(), cites: cited.contains(*r) }).collect();
        let linked_from = self.linked.get(&want).cloned().unwrap_or_default();
        Some(Trace { id: want, spec, cites, tests, source_cites, linked_from, related, related_total: defs.len(), query: names.iter().take(8).cloned().collect::<Vec<_>>().join(" ") })
    }
}

impl Trace {
    pub fn text(&self) -> String {
        let mut o = String::new();
        match &self.spec {
            Some(s) => {
                o += &format!("{} {} {}:{}{}\n  {}\n", self.id, s.level, s.file, s.line, if s.mark.is_empty() { String::new() } else { format!(" ({})", s.mark) }, s.text);
                if !self.cites.is_empty() { o += &format!("  cites: {}\n", self.cites.join(" ")); }
            }
            None => o += &format!("{}: not in specs (tests/sources still cite it)\n", self.id),
        }
        o += "tests:\n";
        for t in &self.tests { o += &format!("  {}:{}  {}\n", t.file, t.line, chars(&t.text, 110)); }
        if self.tests.is_empty() { o += "  none\n"; }
        o += "source cites (file:line  declaration):\n";
        for c in &self.source_cites { o += &format!("  {}:{}  {}  {}\n", c.r.file, c.r.line, if c.decl.is_empty() { "-" } else { &c.decl }, chars(&c.r.text, 90)); }
        if self.source_cites.is_empty() { o += "  none\n"; }
        if !self.linked_from.is_empty() {
            o += "linked from:\n";
            for l in &self.linked_from { o += &format!("  {}:{}  {}\n", l.file, l.line, chars(&l.text, 110)); }
        }
        o += "related code (types the tests name; * = also cites the ID):\n";
        for r in &self.related { o += &format!("  {} {}  {}\n", if r.cites { '*' } else { ' ' }, r.file, r.types.join(" ")); }
        if self.related.is_empty() { o += "  none found\n"; }
        if self.related_total > 12 { o += &format!("  … {} more files\n", self.related_total - 12); }
        if !self.query.is_empty() { o += &format!("next: codegraph explore \"{}\"\n", self.query); }
        o
    }
}

//! Code vs requirements, from the CodeGraph index: `overlap` (code several requirements cover) and `uncovered` (code none covers).
//! Ports of overlap.py and uncovered.py. Pure logic over the rows `codegraph.rs` returns.
use crate::codegraph::{GraphError, UNIT_KINDS};
use crate::report::Analysis;
use crate::scan::{WORD, body, id_key, sorted_ids, stem};
use indexmap::IndexMap;
use serde::Serialize;
use std::collections::{BTreeSet, HashMap, HashSet};

#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
pub struct OverlapRow {
    pub file: String,
    pub line: i64,
    pub name: String,
    pub ids: Vec<String>,
    /// (a, b, linked): linked = one spec cites the other, or both cite a third ID.
    pub pairs: Vec<(String, String, bool)>,
}

#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
pub struct Unit {
    pub name: String,
    pub file: String,
    pub lo: i64,
    pub hi: i64,
    pub visibility: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
pub struct UncoveredFile {
    /// Relative to the app dir.
    pub file: String,
    pub lines: i64,
    pub units: Vec<Unit>,
}

#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
pub struct Skipped {
    pub covered: usize,
    pub shared: usize,
    pub infra: usize,
    pub small: usize,
}

#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
pub struct Uncovered {
    pub units: usize,
    pub lines: i64,
    pub files: Vec<UncoveredFile>,
    pub skipped: Skipped,
    pub fanin: usize,
}

impl Analysis {
    /// Declarations covered by two or more requirements, unlinked cross-file pairs first. `Ok(None)` without an index.
    pub fn overlap(&self, only: &str, same_file: bool) -> Result<Option<Vec<OverlapRow>>, GraphError> {
        let Some(g) = &self.graph else { return Ok(None) };
        let mut nodes: HashMap<&str, Vec<(i64, i64, String)>> = HashMap::new();
        for refs in self.c.values() {
            for r in refs {
                if !nodes.contains_key(r.file.as_str()) {
                    nodes.insert(&r.file, g.decls_in(&r.file)?);
                }
            }
        }
        let cites: HashMap<&str, BTreeSet<String>> = self.s.iter().map(|(i, s)| (i.as_str(), self.p.rx.mention.captures_iter(&s.text).map(|m| m[1].to_string()).filter(|x| x != i).collect())).collect();
        let mut covered: IndexMap<(String, i64, String), BTreeSet<String>> = IndexMap::new();
        for i in sorted_ids(self.c.keys()) {
            if !self.s.contains_key(&i) {
                continue;
            }
            for r in &self.c[&i] {
                let ns = &nodes[r.file.as_str()];
                let n = r.line as i64;
                let node = if r.text.trim_start().starts_with("///") {
                    // a doc comment belongs to the declaration below it
                    ns.iter().filter(|x| n < x.0 && x.0 <= n + 8).min_by_key(|x| x.0)
                } else {
                    ns.iter().filter(|x| x.0 <= n && n <= x.1).min_by_key(|x| x.1 - x.0)
                };
                if let Some(node) = node {
                    covered.entry((r.file.clone(), node.0, node.2.clone())).or_default().insert(i.clone());
                }
            }
        }
        let mut out = vec![];
        for ((file, line, name), ids) in covered {
            if ids.len() < 2 || !file.contains(only) {
                continue;
            }
            let mut sorted: Vec<String> = ids.into_iter().collect();
            sorted.sort_by_key(|i| id_key(i));
            let mut pairs = vec![];
            for (x, a) in sorted.iter().enumerate() {
                for b in &sorted[x + 1..] {
                    let (ca, cb) = (&cites[a.as_str()], &cites[b.as_str()]);
                    let linked = ca.contains(b) || cb.contains(a) || ca.intersection(cb).next().is_some();
                    if self.s[a].file != self.s[b].file || same_file {
                        pairs.push((a.clone(), b.clone(), linked));
                    }
                }
            }
            if !pairs.is_empty() {
                out.push(OverlapRow { file, line, name, ids: sorted, pairs });
            }
        }
        out.sort_by(|a, b| (a.pairs.iter().all(|p| p.2), &a.file, a.line).cmp(&(b.pairs.iter().all(|p| p.2), &b.file, b.line))); // any unlinked pair first
        Ok(Some(out))
    }

    pub fn overlap_text(&self, found: &[OverlapRow], limit: usize) -> String {
        let app = self.app();
        let unlinked = found.iter().filter(|r| !r.pairs.iter().all(|p| p.2)).count();
        let mut o = format!("{} declarations are covered by 2+ requirements; {unlinked} have a pair whose specs do not cite each other (read those first)\n", found.len());
        for r in found.iter().take(limit) {
            o += &format!("  {}:{}  {}  [{}]\n", r.file.strip_prefix(&app).unwrap_or(&r.file), r.line, r.name, r.ids.join(", "));
            for (a, b, linked) in r.pairs.iter().take(6) {
                let (fa, fb) = (&self.s[a].file, &self.s[b].file);
                o += &format!("    {a} {} {b}  {}  {} / {}\n", if *linked { "~" } else { "?" }, if *linked { "linked" } else { "UNLINKED" }, stem(fa), stem(fb));
            }
        }
        if found.len() > limit {
            o += &format!("  … {} more (give a PATH to narrow)\n", found.len() - limit);
        }
        if !found.is_empty() {
            o += "next: spec17 trace <ID> for each spec's text; decide conflicting / complementary / duplicate / incidental per SKILL.md\n";
        }
        o
    }

    /// Top-level units of the app that no requirement covers. `Ok(None)` without an index.
    pub fn uncovered(&self, only: &str, fanin: usize, min_lines: i64) -> Result<Option<Uncovered>, GraphError> {
        let Some(g) = &self.graph else { return Ok(None) };
        let app = self.app();
        let nodes = g.nodes_under(&app)?;
        let by_id: HashMap<&str, &crate::codegraph::Node> = nodes.iter().map(|n| (n.id.as_str(), n)).collect();
        // the top-level declaration a node lives in
        let unit_of = |id: &str| by_id.get(id).filter(|n| !matches!(n.kind.as_str(), "file" | "import")).map(|n| (n.file.clone(), n.qualified.split("::").next().unwrap_or("").to_string()));

        let mut units: IndexMap<(String, String), Unit> = IndexMap::new();
        for n in &nodes {
            if UNIT_KINDS.contains(&n.kind.as_str()) && !n.qualified.contains("::") {
                units.entry((n.file.clone(), n.name.clone())).or_insert(Unit { name: n.name.clone(), file: n.file.clone(), lo: n.lo, hi: n.hi, visibility: n.visibility.clone(), kind: n.kind.clone() });
            }
        }
        let cited: HashSet<(&str, i64)> = self.c.values().flatten().map(|r| (r.file.as_str(), r.line as i64)).collect();
        let mut tested: HashSet<String> = HashSet::new(); // names that appear in the body of a test tagged with a requirement
        for refs in self.t.values() {
            for r in refs {
                for line in body(&self.p.lines(&r.file), r.line, 200, self.p.lang(&r.file)) {
                    tested.extend(WORD.find_iter(line).map(|m| m.as_str().to_string()));
                }
            }
        }
        let covered: HashSet<(String, String)> = units.iter().filter(|(k, u)| cited.iter().any(|(f, n)| *f == k.0 && u.lo - 4 <= *n && *n <= u.hi)).map(|(k, _)| k.clone()).collect();
        let (mut fan, mut used_by_cited): (HashMap<(String, String), HashSet<String>>, HashSet<(String, String)>) = Default::default();
        for (src, dst) in g.use_edges()? {
            let (Some(a), Some(b)) = (unit_of(&src), unit_of(&dst)) else { continue };
            if a == b {
                continue;
            }
            if a.0 != b.0 {
                fan.entry(b.clone()).or_default().insert(a.0.clone());
            }
            if covered.contains(&a) {
                used_by_cited.insert(b);
            }
        }
        let infra = &self.p.cfg.infra_paths;
        let mut skipped = Skipped { covered: 0, shared: 0, infra: 0, small: 0 };
        let mut out: Vec<&Unit> = vec![];
        for (k, u) in &units {
            let rel = u.file.strip_prefix(&app).unwrap_or(&u.file);
            if covered.contains(k) || used_by_cited.contains(k) || tested.contains(&u.name) {
                skipped.covered += 1;
            } else if infra.iter().any(|p| rel.starts_with(p.as_str())) {
                skipped.infra += 1;
            } else if fan.get(k).map_or(0, HashSet::len) >= fanin {
                skipped.shared += 1;
            } else if matches!(u.visibility.as_str(), "private" | "fileprivate") || u.hi - u.lo + 1 < min_lines {
                skipped.small += 1;
            } else if u.file.contains(only) {
                out.push(u);
            }
        }
        let mut by: IndexMap<String, Vec<Unit>> = IndexMap::new();
        for u in &out {
            by.entry(u.file.strip_prefix(&app).unwrap_or(&u.file).to_string()).or_default().push((*u).clone());
        }
        let size = |us: &[Unit]| us.iter().map(|u| u.hi - u.lo + 1).sum::<i64>();
        let mut files: Vec<UncoveredFile> = by.into_iter().map(|(file, mut units)| {
            units.sort_by_key(|u| u.lo);
            UncoveredFile { lines: size(&units), file, units }
        }).collect();
        files.sort_by_key(|f| -f.lines); // stable: ties keep index order
        Ok(Some(Uncovered { units: out.len(), lines: out.iter().map(|u| u.hi - u.lo + 1).sum(), files, skipped, fanin }))
    }
}

impl Uncovered {
    pub fn text(&self) -> String {
        let mut o = format!(
            "{} units / {} lines in {} files have no requirement (skipped: {} covered, {} shared by >={} files, {} controls/styling, {} small or private)\n",
            self.units, self.lines, self.files.len(), self.skipped.covered, self.skipped.shared, self.fanin, self.skipped.infra, self.skipped.small
        );
        for f in self.files.iter().take(30) {
            let us: Vec<String> = f.units.iter().map(|u| format!("{}:{}-{}", u.name, u.lo, u.hi)).collect();
            o += &format!("  {}  {} lines: {}\n", f.file, f.lines, us.join(", "));
        }
        if self.files.len() > 30 {
            o += &format!("  … {} more files (give a PATH to narrow)\n", self.files.len() - 30);
        }
        if self.units > 0 {
            o += "next: codegraph explore \"<Name>\" to read one, then decide: business rule -> write the requirement; plumbing -> ignore\n";
        }
        o
    }
}

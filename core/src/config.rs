//! `spec17.json` at the repo root. Every field is optional: an empty one is detected from the repo (see `detect.rs`).
use crate::detect::{self, Layout, Profile};
use serde_json::{Map, Value};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Folder of `*.md` specs, relative to the root (also in subfolders).
    pub specs: String,
    pub tests: Vec<String>,
    pub src: Vec<String>,
    /// File extensions scanned in `tests` and `src`.
    pub ext: Vec<String>,
    /// Ecosystem profiles shaped like a `[[profile]]` of `resources/profiles.toml`: one named like a built-in one changes the fields it
    /// sets, any other adds an ecosystem (its comment markers, declaration, assertion and switched-off regexes).
    pub profiles: Vec<Map<String, Value>>,
    pub retired_prefixes: Vec<String>,
    /// Priority and effort hints in requirement text: ordered regexes, capture group 1 is the value, the first match wins.
    /// Empty = the built-in patterns of `resources/spec_format.toml`; a list replaces them.
    pub priority: Vec<String>,
    pub effort: Vec<String>,
    /// CodeGraph index; empty = `DEFAULT_DB`.
    pub codegraph: String,
    /// Path prefixes under `src[0]` that `uncovered` treats as plumbing.
    pub infra_paths: Vec<String>,
}

/// The CodeGraph index when `spec17.json` names none.
pub const DEFAULT_DB: &str = ".codegraph/codegraph.db";

impl Config {
    // (SCAN-06)
    /// `spec17.json` of `root`, or the defaults; `codegraph` is never empty after this.
    pub fn load(root: &Path) -> Result<Config, String> {
        let f = root.join("spec17.json");
        let mut cfg = if f.exists() {
            let text = std::fs::read_to_string(&f).map_err(|e| format!("{}: {e}", f.display()))?;
            let v: Value = serde_json::from_str(&text).map_err(|e| format!("{}: {e}", f.display()))?;
            if let Some(k) = ["comment", "decl", "assert", "off", "case", "test_files"].into_iter().find(|k| v.get(k).is_some()) {
                return Err(format!("{}: `{k}` moved into `profiles`: an entry like a [[profile]] of profiles.toml, e.g. {{\"name\": \"rust\", \"{}\": …}}", f.display(), if k == "test_files" { "test" } else { k }));
            }
            serde_json::from_value(v).map_err(|e| format!("{}: {e}", f.display()))?
        } else {
            Config::default()
        };
        if cfg.codegraph.is_empty() {
            cfg.codegraph = DEFAULT_DB.into();
        }
        Ok(cfg)
    }

    /// Fill what the file left empty from the repo's file list (CodeGraph's, else a walk); what it set stays as it is.
    /// Returns the index in `profiles` of the dominant ecosystem.
    /// (SCAN-09)
    pub fn detect(&mut self, profiles: &[Profile], files: &[String]) -> Option<usize> {
        let Layout { profile: p, src, tests } = detect::layout(profiles, files)?;
        for (field, auto) in [(&mut self.src, src), (&mut self.tests, tests), (&mut self.ext, p.ext.clone())] {
            if field.is_empty() {
                *field = auto;
            }
        }
        profiles.iter().position(|x| std::ptr::eq(x, p))
    }
}

use rusqlite::Connection;
use spec17_core::{Analysis, StatusOpts, codegraph::{Graph, GraphError}, scan::Project};
use std::{fs, path::{Path, PathBuf}};

fn write(dir: &Path, rel: &str, text: &str) {
    let p = dir.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, text).unwrap();
}

/// A tiny repo that exercises every state and issue kind.
fn fixture(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("spec17-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    write(&d, "spec17.json", r#"{"tests":["tests"],"src":["src"],"ext":["swift"]}"#);
    write(&d, "specs/01-demo.md", "\
- **DEM-01** MUST — tested thing.
- **DEM-02** MUST — only cited in src.
- **DEM-03** SHOULD — tagged only in a comment.
- **DEM-04** MAY — checked by a person (manual)
- **DEM-05** MUST — built already (later)
- **DEM-06** MUST — first definition.
- **DEM-06** MUST — second definition.
- **DEM-07** has no level.
- ✅ **DEM-08** MUST — marked done without tests.
- **DEM-09** MUST — wraps
  onto a second line (later)
- ~~**DEM-10** MUST — retired.~~
- **DEM-11** MAY — nothing at all.
");
    write(&d, "tests/DemoTests.swift", "\
@Test(\"[DEM\x2D01] works\") func a() {
    #expect(true)
}
@Test(\"[DEM\x2D05] built already\") func b() {
    let x = 1
}
@Test(\"[DEM\x2D99] unknown\") func c() {
    #expect(true)
}
@Test(\"[DEM\x2D10] retired\") func e() {
    #expect(true)
}
// [DEM\x2D03] only in a comment
");
    write(&d, "src/Demo.swift", "\
/// (DEM-02) the rule
struct Demo {
    func run() {}
}
// DEM-98 typo
");
    d
}

#[doc = "[SCAN-01, SCAN-02, SCAN-03, SCAN-04]"]
#[test]
fn states_and_issues() {
    let a = Analysis::load(Some(&fixture("states"))).unwrap();
    let st = |i: &str| a.state[i].as_str();
    assert_eq!((st("DEM-01"), st("DEM-02"), st("DEM-03"), st("DEM-04"), st("DEM-05")), ("tested", "src-only", "tested", "manual", "later"));
    assert_eq!(a.s["DEM-09"].mark, "later");
    assert_eq!(a.s["DEM-09"].text, "wraps onto a second line (later)");
    assert!(!a.s.contains_key("DEM-07") && !a.s.contains_key("DEM-10"));
    assert!(a.issues.iter().any(|x| x.to_string().starts_with("DEM-06 is defined twice")));
    assert!(a.issues.iter().any(|x| x.id == "DEM-07" && x.uncounted));
    assert_eq!(a.verdict("DEM-08"), "NO-TEST");
    assert!(a.built("DEM-05"));
}

#[doc = "[SCAN-05]"]
#[test]
fn same_tree_same_text() {
    let d = fixture("determinism");
    let text = || {
        let a = Analysis::load(Some(&d)).unwrap();
        (a.report().text(&a, 12), a.status(&StatusOpts { all: true, ..Default::default() }).unwrap().text(&StatusOpts { all: true, ..Default::default() }))
    };
    let first = text();
    assert_eq!(first, text());
    assert_eq!(first, text());
}

#[doc = "[SCAN-06]"]
#[test]
fn config_comes_from_spec17_json_or_defaults() {
    let a = Analysis::load(Some(&fixture("config"))).unwrap();
    assert_eq!((a.p.cfg.tests.clone(), a.p.cfg.src.clone(), a.p.cfg.ext.clone()), (vec!["tests".to_string()], vec!["src".to_string()], vec!["swift".to_string()]));

    let bare = std::env::temp_dir().join(format!("spec17-{}-bare", std::process::id()));
    let _ = fs::remove_dir_all(&bare);
    write(&bare, "specs/01-a.md", "- **AAA-01** MUST — one.\n");
    let a = Analysis::load(Some(&bare)).unwrap();
    assert_eq!(a.p.cfg.specs, "specs");
    assert!(a.p.cfg.ext.is_empty() && a.p.cfg.src.is_empty()); // nothing to detect from

    write(&bare, "spec17.json", r#"{"profiles": [{"name": "swift", "case": "("}]}"#);
    let err = Analysis::load(Some(&bare)).err().expect("a bad case regex is an error");
    assert!(err.contains("bad regex"), "{err}");

    write(&bare, "spec17.json", "{not json");
    let err = Analysis::load(Some(&bare)).err().expect("a broken spec17.json is an error");
    assert!(err.contains("spec17.json"), "{err}");
}

#[doc = "[GRAPH-03]"]
#[test]
fn a_missing_index_is_not_checked_not_an_error() {
    let d = fixture("noindex");
    assert!(Graph::open(&d.join(".codegraph/codegraph.db")).unwrap().is_none());
    let a = Analysis::load(Some(&d)).unwrap();
    let r = a.report();
    assert!(!r.index);
    assert!(r.text(&a, 12).contains("not checked"), "{}", r.text(&a, 12));
}

#[doc = "[SCAN-07]"]
#[test]
fn report_problems() {
    let a = Analysis::load(Some(&fixture("report"))).unwrap();
    let r = a.report();
    assert_eq!(r.unimplemented, ["DEM-06", "DEM-08", "DEM-11"]);
    assert_eq!(r.untested, ["DEM-02"]);
    assert_eq!(r.built, ["DEM-05"]);
    assert_eq!(r.dangling_tests, ["DEM-10", "DEM-99"]); // a retired ID is a leftover too
    assert_eq!(r.dangling_sources, ["DEM-98"]);
    let bad = |k: &str| r.invalid_tests[k].iter().map(|p| p.id.as_str()).collect::<Vec<_>>();
    assert_eq!(bad("unknown"), ["DEM-99"]);
    assert_eq!(bad("retired"), ["DEM-10"]);
    assert_eq!(bad("no-assert"), ["DEM-05"]);
    assert_eq!(bad("comment-only"), ["DEM-03"]);
    assert!(!r.index); // no CodeGraph index: reported as not checked, never an error
    assert!(r.text(&a, 12).contains("[code with no requirement] not checked"));
}

#[doc = "[SCAN-08]"]
#[test]
fn status_listing_and_trace() {
    let a = Analysis::load(Some(&fixture("status"))).unwrap();
    let o = StatusOpts { missing: true, effort: "low".into(), ..Default::default() };
    let rows = a.status(&o).unwrap().rows;
    assert_eq!(rows.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(), ["DEM-02", "DEM-06", "DEM-08", "DEM-11"]);
    assert!(a.status(&StatusOpts { prefix: Some("NOPE".into()), ..Default::default() }).is_err());
    let t = a.trace("dem-02").unwrap();
    assert_eq!(t.source_cites[0].decl, "Demo");
    assert!(a.trace("ZZZ-1").is_none());
}

#[doc = "[WEB-47]"]
#[test]
fn requirement_rows_include_defined_references() {
    let d = std::env::temp_dir().join(format!("spec17-{}-requirement-references", std::process::id()));
    write(&d, "specs/01-demo.md", "- **DEM-01** MUST — see DEM-02, DEM-02, DEM-01 and DEM-99.\n- **DEM-02** MUST — another requirement.\n");
    let a = Analysis::load(Some(&d)).unwrap();
    let rows = a.status(&StatusOpts { all: true, ..Default::default() }).unwrap().rows;
    assert_eq!(rows.iter().find(|r| r.id == "DEM-01").unwrap().references, ["DEM-01", "DEM-02"]);
    assert!(rows.iter().find(|r| r.id == "DEM-02").unwrap().references.is_empty());
}

#[doc = "[SCAN-29]"]
#[test]
fn a_citation_above_a_doc_comment_belongs_to_the_declaration_below() {
    let d = std::env::temp_dir().join(format!("spec17-{}-enclosing", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    write(&d, "spec17.json", r#"{"tests":["tests"],"src":["src"],"ext":["rs"]}"#);
    write(&d, "specs/01-demo.md", "- **DEM-01** MUST — a.\n- **DEM-02** MUST — b.\n- **DEM-03** MUST — c.\n- **DEM-04** MUST — d.\n");
    write(&d, "src/lib.rs", "\
fn before() {}

// (DEM-01)
/// Doc of the next one.
#[inline]
pub fn after() {}

fn body() {
    let x = 1;
    // (DEM-02) mid-body: not above a declaration
    let y = 2;
}

/// Doc comment.
// (DEM-03) after the doc, above the declaration
fn third() {}

fn last() {
    let z = 3; // (DEM-04) trailing
}
");
    let a = Analysis::load(Some(&d)).unwrap();
    let decl = |id: &str| a.trace(id).unwrap().source_cites[0].decl.clone();
    assert_eq!([decl("DEM-01"), decl("DEM-02"), decl("DEM-03"), decl("DEM-04")], ["after", "body", "third", "last"]);
}

#[doc = "[SCAN-33]"]
#[test]
fn each_file_reads_with_its_own_profile() {
    let d = std::env::temp_dir().join(format!("spec17-{}-mixed", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    write(&d, "spec17.json", r#"{"tests":["tests"],"src":["src"],"ext":["rs","py"]}"#);
    write(&d, "specs/01-demo.md", "- **DEM-01** MUST — a.\n- **DEM-02** MUST — b.\n- **DEM-03** MUST — c.\n- **DEM-04** MUST — d.\n");
    write(&d, "tests/a.rs", "#[doc = \"[DEM\x2D01]\"]\n#[test]\nfn a() {\n    assert!(true);\n}\n");
    write(&d, "tests/test_b.py", "# [DEM\x2D02]\ndef test_b():\n    assert 1 == 1\n\ndef test_c():\n    \"\"\"[DEM\x2D03]\"\"\"\n    assert 1 == 1\n");
    write(&d, "src/lib.rs", "pub fn f() {} // (DEM\x2D04) #[not a comment here]\n");
    write(&d, "src/c.py", "def g():  # (DEM\x2D04)\n    pass\n");
    let a = Analysis::load(Some(&d)).unwrap();
    let r = a.report();
    let bad = |k: &str| r.invalid_tests[k].iter().map(|p| p.id.as_str()).collect::<Vec<_>>();
    assert!(bad("comment-only").is_empty()); // the Python comment is attached to the test
    assert!(bad("no-assert").is_empty()); // a Python `assert` counts in a .py file
    let mut decls: Vec<String> = a.trace("DEM-04").unwrap().source_cites.iter().map(|c| c.decl.clone()).collect();
    decls.sort();
    assert_eq!(decls, ["f", "g"]);
}

#[doc = "[SCAN-39]"]
#[test]
fn svelte_citations_work_without_project_configuration() {
    let d = scratch("svelte-profile");
    write(&d, "specs/01-demo.md", "- **DEM-01** MUST — a.\n- **DEM-02** MUST — b.\n");
    write(&d, "src/App.svelte", "<script>\n// (DEM\x2D01)\nconst value = 1;\n</script>\n<!-- (DEM\x2D02) -->\n<p>{value}</p>\n");
    let a = Analysis::load(Some(&d)).unwrap();
    for id in ["DEM-01", "DEM-02"] {
        assert_eq!(a.trace(id).unwrap().source_cites[0].r.file, "src/App.svelte");
    }
}

#[doc = "[SCAN-07]"]
#[test]
fn rust_matches_alone_is_not_an_assertion() {
    let d = scratch("rust-matches");
    write(&d, "SPEC.md", "- **DEM-01** MUST — a.\n- **DEM-02** MUST — b.\n");
    write(&d, "tests/example.rs", "#[doc = \"[DEM\x2D01]\"]\n#[test]\nfn a() { matches!(1, 1); }\n#[doc = \"[DEM\x2D02]\"]\n#[test]\nfn b() { assert!(matches!(1, 1)); }\n");
    let a = Analysis::load(Some(&d)).unwrap();
    assert_eq!(a.report().invalid_tests["no-assert"].iter().map(|p| p.id.as_str()).collect::<Vec<_>>(), ["DEM-01"]);
}

#[doc = "[SCAN-35]"]
#[test]
fn attached_comment_tags_are_assessed_with_the_test() {
    let d = scratch("comment-tags");
    write(&d, "spec17.json", r#"{"tests":["Tests"],"ext":["swift"]}"#);
    write(&d, "SPEC.md", "- DEM-01 MUST — a.\n- DEM-02 MUST — b.\n- DEM-03 MUST — c.\n");
    write(&d, "Tests/ExampleTests.swift", "// [DEM\x2D01, DEM\x2D02]\n@MainActor\nfunc testExample() {\n    XCTAssertTrue(true)\n}\n\n// [DEM\x2D03]\n\nfunc testOther() {}\n");
    let a = Analysis::load(Some(&d)).unwrap();
    let r = a.report();
    assert_eq!(r.invalid_tests["comment-only"].iter().map(|p| p.id.as_str()).collect::<Vec<_>>(), ["DEM-03"]);
    assert!(r.invalid_tests["no-assert"].is_empty());
    assert_eq!(a.tests_view().modules[0].files[0].tests[0].ids, ["DEM-01", "DEM-02"]);
    let schema = serde_json::to_value(schemars::schema_for!(spec17_core::modules::TestCase)).unwrap();
    assert!(!schema["required"].as_array().unwrap().iter().any(|field| field == "qualified"));
}

#[doc = "[SCAN-35]"]
#[test]
fn comment_tags_use_their_own_test_for_disabled_and_assertion_checks() {
    let d = scratch("python-comment-tags");
    write(&d, "spec17.json", r#"{"tests":["tests"],"ext":["py"]}"#);
    write(&d, "SPEC.md", "- DEM-01 MUST — disabled.\n- DEM-02 MUST — no assertion.\n- DEM-03 MUST — orphan.\n");
    write(&d, "tests/test_example.py", "@pytest.mark.skip\n# [DEM\x2D01]\ndef test_skipped():\n    assert True\n\n# [DEM\x2D02]\ndef test_without_assertion():\n    pass\n\n# [DEM\x2D03]\n\nvalue = 1\n\ndef test_other():\n    assert True\n");
    let a = Analysis::load(Some(&d)).unwrap();
    let report = a.report();
    let ids = |kind: &str| report.invalid_tests[kind].iter().map(|p| p.id.as_str()).collect::<Vec<_>>();
    assert_eq!(ids("disabled"), ["DEM-01"]);
    assert_eq!(ids("no-assert"), ["DEM-02"]);
    assert_eq!(ids("comment-only"), ["DEM-03"]);
    let cases = &a.tests_view().modules[0].files[0].tests;
    assert_eq!(cases[0].ids, ["DEM-01"]);
    assert_eq!(cases[1].ids, ["DEM-02"]);
    assert!(cases[2].ids.is_empty());
}

#[doc = "[SCAN-35]"]
#[test]
fn comment_tag_inside_a_test_belongs_to_that_test() {
    let d = scratch("body-comment-tag");
    write(&d, "spec17.json", r#"{"tests":["Tests"],"ext":["swift"]}"#);
    write(&d, "SPEC.md", "- DEM-01 MUST — a.\n");
    write(&d, "Tests/ExampleTests.swift", "func testExample() {\n    // [DEM\x2D01]\n    XCTAssertTrue(true)\n}\n");
    let a = Analysis::load(Some(&d)).unwrap();
    let report = a.report();
    assert!(report.invalid_tests["comment-only"].is_empty());
    assert!(report.invalid_tests["no-assert"].is_empty());
    assert_eq!(a.tests_view().modules[0].files[0].tests[0].ids, ["DEM-01"]);
}

#[doc = "[SCAN-36]"]
#[test]
fn report_distinguishes_attached_and_orphan_comment_tags() {
    let d = scratch("comment-tag-state");
    write(&d, "spec17.json", r#"{"tests":["tests"],"ext":["py"]}"#);
    write(&d, "SPEC.md", "- DEM-01 MUST — a.\n- DEM-02 MUST — b.\n");
    write(&d, "tests/test_a.py", "# [DEM\x2D01]\ndef test_a():\n    assert True\n\n# [DEM\x2D02]\n\nvalue = 1\n");
    let a = Analysis::load(Some(&d)).unwrap();
    let r = a.report();
    assert_eq!(r.comment_tagged.iter().map(|p| p.id.as_str()).collect::<Vec<_>>(), ["DEM-01"]);
    assert_eq!(r.invalid_tests["comment-only"].iter().map(|p| p.id.as_str()).collect::<Vec<_>>(), ["DEM-02"]);
    assert!(r.text(&a, 12).contains("comment-tagged"));
}

#[doc = "[SCAN-35]"]
#[test]
fn non_brace_test_bodies_end_before_following_code() {
    for (ext, first, second) in [
        ("rb", "it 'first' do\n  # [DEM\x2D01]\nend\n", "it 'second' do\n  # [DEM\x2D02]\n  expect(true)\nend\n"),
        ("lua", "it('first', function()\n  -- [DEM\x2D01]\nend)\n", "it('second', function()\n  -- [DEM\x2D02]\n  assert(true)\nend)\n"),
        ("vb", "<Test>\nSub first()\n  ' [DEM\x2D01]\nEnd Sub\n", "<Test>\nSub second()\n  ' [DEM\x2D02]\n  Assert.IsTrue(True)\nEnd Sub\n"),
        ("pas", "procedure TestFirst;\nbegin\n  // [DEM\x2D01]\nend;\n", "procedure TestSecond;\nbegin\n  // [DEM\x2D02]\n  AssertTrue(true);\nend;\n"),
        ("erl", "first_test() ->\n  % [DEM\x2D01]\n  ok.\n", "second_test() ->\n  % [DEM\x2D02]\n  ?assert(true).\n"),
        ("sql", "CREATE FUNCTION test_first() RETURNS void AS $$\n  -- [DEM\x2D01]\n  SELECT 1;\n$$ LANGUAGE SQL;\n", "CREATE FUNCTION test_second() RETURNS void AS $$\n  -- [DEM\x2D02]\n  SELECT ok(true);\n$$ LANGUAGE SQL;\n"),
    ] {
        let d = scratch(&format!("body-{ext}"));
        write(&d, "spec17.json", &format!(r#"{{"tests":["tests"],"ext":["{ext}"]}}"#));
        write(&d, "SPEC.md", "- DEM-01 MUST — a.\n- DEM-02 MUST — b.\n");
        write(&d, &format!("tests/test_example.{ext}"), &format!("{first}\n{second}"));
        let a = Analysis::load(Some(&d)).unwrap();
        let cases = &a.tests_view().modules[0].files[0].tests;
        assert_eq!(cases.len(), 2, "{ext}");
        assert_eq!(cases[0].ids, ["DEM-01"], "{ext}");
        assert_eq!(cases[1].ids, ["DEM-02"], "{ext}");
        assert_eq!(a.report().invalid_tests["no-assert"].iter().map(|p| p.id.as_str()).collect::<Vec<_>>(), ["DEM-01"], "{ext}");
    }
}

#[doc = "[SCAN-35]"]
#[test]
fn test_boundaries_ignore_literals_and_nested_blocks() {
    for (ext, text) in [
        ("swift", "func testFirst() {\n  // [DEM\x2D01]\n  let s = \"}\"\n  /* } */\n  XCTAssertTrue(true)\n}\n\n// [DEM\x2D02]\n\nfunc helper() {}\n"),
        ("sql", "CREATE FUNCTION test_first() RETURNS void AS $body$\n  -- [DEM\x2D01]\n  SELECT ok(true);\n$body$ LANGUAGE SQL;\n\n-- [DEM\x2D02]\n\nSELECT 1;\n"),
        ("sql", "CREATE PROCEDURE test_first AS\nBEGIN\n  -- [DEM\x2D01]\n  SELECT CASE WHEN 1=1 THEN 1 ELSE 0 END;\n  EXEC tSQLt.AssertEquals 1, 1;\nEND;\n\n-- [DEM\x2D02]\n\nSELECT 1;\n"),
        ("rb", "it 'first' do\nif true\nend\n  # [DEM\x2D01]\n  expect(true)\nend\n\n# [DEM\x2D02]\n\nputs 'outside'\n"),
        ("lua", "it('first', function()\nif true then\nend\n  -- [DEM\x2D01]\n  assert(true)\nend)\n\n-- [DEM\x2D02]\n\nprint('outside')\n"),
        ("pas", "procedure TestFirst;\nbegin\nbegin\nend;\n  // [DEM\x2D01]\n  AssertTrue(true);\nend;\n\n// [DEM\x2D02]\n\nprocedure helper;\n"),
    ] {
        let d = scratch(&format!("nested-{ext}"));
        write(&d, "spec17.json", &format!(r#"{{"tests":["tests"],"ext":["{ext}"]}}"#));
        write(&d, "SPEC.md", "- DEM-01 MUST — a.\n- DEM-02 MUST — b.\n");
        write(&d, &format!("tests/test_example.{ext}"), text);
        let a = Analysis::load(Some(&d)).unwrap();
        let cases = &a.tests_view().modules[0].files[0].tests;
        assert_eq!(cases[0].ids, ["DEM-01"], "{ext}");
        let r = a.report();
        assert!(r.invalid_tests["no-assert"].is_empty(), "{ext}");
        assert_eq!(r.invalid_tests["comment-only"].iter().map(|p| p.id.as_str()).collect::<Vec<_>>(), ["DEM-02"], "{ext}");
    }
}

#[doc = "[SCAN-37]"]
#[test]
fn swift_backticked_test_names_are_detected() {
    let d = scratch("swift-backticks");
    write(&d, "spec17.json", r#"{"tests":["Tests"],"ext":["swift"]}"#);
    write(&d, "SPEC.md", "- DEM-01 MUST — a.\n");
    write(&d, "Tests/ExampleTests.swift", "// [DEM\x2D01]\n@objc(testWorks) func `test does work`() { XCTAssertTrue(true) }\n");
    let a = Analysis::load(Some(&d)).unwrap();
    let cases = &a.tests_view().modules[0].files[0].tests;
    assert_eq!(cases.len(), 1);
    assert_eq!(cases[0].name, "test does work");
    assert_eq!(cases[0].ids, ["DEM-01"]);
}

#[doc = "[SCAN-38]"]
#[test]
fn example_profile_configuration_loads() {
    let d = scratch("example-config");
    write(&d, "spec17.json", include_str!("../../spec17.example.json"));
    assert!(Project::open(Some(&d)).is_ok());
}

#[doc = "[SCAN-34]"]
#[test]
fn profile_patterns_accept_ordered_lists_and_validate_every_entry() {
    let d = scratch("profile-pattern-lists");
    write(&d, "SPEC.md", "- DEM-01 MUST — a.\n- DEM-02 MUST — b.\n");
    let config = r#"{"profiles":[{"name":"example","ext":["xyz"],"comment":["//"],"decl":["^fn (a)","^fn (\\w+)"],"assert":["unmatched!","check!"],"off":["NEVER_SKIP","SKIP"],"test":["^other/","^tests/"],"case":["^CASE$","^OTHER$"]}]}"#;
    write(&d, "spec17.json", config);
    write(&d, "src/app.xyz", "fn app() {}\n");
    write(&d, "tests/example.xyz", "// [DEM\x2D01]\nCASE\nfn alpha() {\n check!(true)\n}\n\n@SKIP\n// [DEM\x2D02]\nOTHER\nfn beta() {\n check!(true)\n}\n");
    let a = Analysis::load(Some(&d)).unwrap();
    let cases = &a.tests_view().modules[0].files[0].tests;
    assert_eq!(cases.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(), ["a", "beta"]);
    assert_eq!(cases[0].ids, ["DEM-01"]);
    assert_eq!(a.report().invalid_tests["disabled"][0].id, "DEM-02");
    assert!(a.report().invalid_tests["no-assert"].is_empty());
    write(&d, "spec17.json", &config.replace("\"check!\"", "\"(\""));
    assert!(Analysis::load(Some(&d)).err().unwrap().contains("bad regex"));
}

#[doc = "[SCAN-34]"]
#[test]
fn spec17_json_adds_and_merges_profiles() {
    let d = std::env::temp_dir().join(format!("spec17-{}-profiles", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    write(&d, "specs/01-demo.md", "- **DEM-01** MUST — a.\n- **DEM-02** MUST — b.\n- **DEM-03** MUST — c.\n- **DEM-04** MUST — d.\n");
    write(&d, "spec17.json", r#"{"tests":["tests"],"src":["src"],"ext":["rs","as","sql","pgt"],"profiles":[
        {"name":"actionscript","ext":["as"],"comment":["//"],"decl":"^\\s*(?:public\\s+)?function\\s+(\\w+)"},
        {"name":"rust","assert":"check!"}]}"#);
    write(&d, "src/a.as", "public function f():void {} // (DEM\x2D01)\n");
    write(&d, "src/b.sql", "CREATE OR REPLACE FUNCTION h() -- (DEM\x2D04)\n");
    write(&d, "tests/b_test.sql", "-- [DEM\x2D04]\nCREATE FUNCTION test_h() RETURNS SETOF TEXT AS $$\n  SELECT ok(h());\n$$ LANGUAGE sql;\n");
    write(&d, "tests/d.pgt", "CREATE FUNCTION test_d() RETURNS SETOF TEXT AS $$ SELECT ok(true); $$ LANGUAGE sql;\n");
    write(&d, "tests/c_test.sql", "-- [DEM\x2D04]\nCREATE PROCEDURE [Orders].[test saves order] AS\n  EXEC tSQLt.AssertEquals 1, 1;\n");
    write(&d, "src/lib.rs", "pub fn g() {} // (DEM\x2D02)\n");
    write(&d, "tests/a.rs", "#[doc = \"[DEM\x2D03]\"]\n#[test]\nfn a() {\n    check!(true);\n}\n");
    let a = Analysis::load(Some(&d)).unwrap();
    let cite = |id: &str| a.trace(id).unwrap().source_cites.iter().map(|c| c.decl.clone()).collect::<Vec<_>>();
    assert_eq!(cite("DEM-01"), ["f"]); // a new ecosystem (Flash ActionScript has no built-in profile)
    assert_eq!(cite("DEM-04"), ["h"]); // SQL has one
    assert_eq!(cite("DEM-02"), ["g"]); // the merged Rust profile keeps its `//` and its `decl`
    assert!(a.report().invalid_tests["no-assert"].is_empty()); // and takes the `assert` it was given
    let sql = a.tests_view().modules.iter().flat_map(|m| &m.files).find(|f| f.file == "tests/b_test.sql").map(|f| f.tests.iter().map(|c| (c.name.clone(), c.ids.clone())).collect::<Vec<_>>());
    assert_eq!(sql, Some(vec![("test_h".to_string(), vec!["DEM-04".to_string()])])); // a pgTAP test case
    assert_eq!(a.tests_view().modules.iter().flat_map(|m| &m.files).find(|f| f.file == "tests/c_test.sql").map(|f| f.tests.len()), Some(1)); // a tSQLt one, schema in brackets
    assert_eq!(a.tests_view().modules.iter().flat_map(|m| &m.files).find(|f| f.file == "tests/d.pgt").map(|f| f.tests[0].name.clone()), Some("test_d".into())); // no profile knows `.pgt`: the dominant one (SQL) reads it

    write(&d, "spec17.json", r#"{"comment":["//"]}"#);
    let err = Analysis::load(Some(&d)).err().expect("a retired field is an error");
    assert!(err.contains("`comment` moved into `profiles`"), "{err}");
    write(&d, "spec17.json", r#"{"test_files":"x"}"#);
    let err = Analysis::load(Some(&d)).err().expect("a retired field is an error");
    assert!(err.contains("`test_files` moved into `profiles`") && err.contains(r#""test": …"#), "{err}");
    write(&d, "spec17.json", r#"{"profiles":[{"name":"rust","inline":false}]}"#);
    let a = Analysis::load(Some(&d)).unwrap();
    assert_eq!(a.p.cfg.tests, ["tests"]); // a bool turned off: Rust tests no longer live in `src` too
    write(&d, "spec17.json", r#"{"profiles":[{"name":"rust","inline":false,"asert":"x"}]}"#);
    let err = Analysis::load(Some(&d)).err().expect("an unknown profile field is an error");
    assert!(err.contains("profile rust") && err.contains("asert"), "{err}");
    write(&d, "spec17.json", r#"{"profiles":[{"name":"rust","decl":"^fn \\w+"}]}"#);
    let err = Analysis::load(Some(&d)).err().expect("a decl without group 1 is an error");
    assert!(err.contains("profile rust") && err.contains("capture group 1"), "{err}");
    // the `test` of a second same-named entry is checked too, also where `src` and `tests` are listed by hand
    write(&d, "spec17.json", r#"{"tests":["tests"],"src":["src"],"profiles":[{"name":"rust","inline":false},{"name":"rust","test":"("}]}"#);
    let err = Analysis::load(Some(&d)).err().expect("a bad test regex is an error");
    assert!(err.contains("profile rust") && err.contains("bad regex"), "{err}");
}

/// A minimal CodeGraph index: the tagged test calls `Demo.run`.
fn index(d: &Path, with_edges_kind: bool) {
    fs::create_dir_all(d.join(".codegraph")).unwrap();
    let c = Connection::open(d.join(".codegraph/codegraph.db")).unwrap();
    c.execute_batch(&format!("
        create table nodes(id text primary key, kind text, name text, qualified_name text, file_path text, start_line int, end_line int, visibility text);
        create table edges(id integer primary key, source text, target text, {});
        create virtual table nodes_fts using fts5(name, qualified_name, content='nodes', content_rowid='rowid');
        create table schema_versions(version int, applied_at int, description text);
        insert into schema_versions values (11, 0, 'x');
        insert into nodes values ('t1','function','a','a','tests/DemoTests.swift',1,3,null),
                                 ('n1','struct','Demo','Demo','src/Demo.swift',2,4,'internal'),
                                 ('n2','method','run','Demo::run','src/Demo.swift',3,3,'internal');
        {}
        insert into nodes_fts(rowid, name, qualified_name) select rowid, name, qualified_name from nodes;
    ", if with_edges_kind { "kind text" } else { "other text" },
        if with_edges_kind { "insert into edges(source,target,kind) values ('t1','n2','calls'),('n2','n1','references');" } else { "" })).unwrap();
}

#[doc = "[SCAN-35]"]
#[test]
fn indexed_test_boundaries_are_shared_and_stale_indexes_fall_back() {
    let d = scratch("indexed-test-boundaries");
    write(&d, "spec17.json", r#"{"tests":["tests"],"ext":["swift"]}"#);
    write(&d, "SPEC.md", "- DEM-01 MUST — a.\n");
    let test = "func testExample() {\n  let text = #\"a \" } b\"#\n  // [DEM\x2D01]\n  XCTAssertTrue(true)\n}\n";
    write(&d, "tests/DemoTests.swift", test);
    index(&d, true);
    let c = Connection::open(d.join(".codegraph/codegraph.db")).unwrap();
    c.execute_batch("delete from nodes; insert into nodes values ('t','method','testExample','testExample','tests/DemoTests.swift',1,5,null);").unwrap();
    drop(c);
    let a = Analysis::load(Some(&d)).unwrap();
    assert_eq!(a.tests_view().modules[0].files[0].tests[0].ids, ["DEM-01"]);
    assert!(a.report().invalid_tests["comment-only"].is_empty());
    assert!(a.report().invalid_tests["no-assert"].is_empty());
    drop(a);
    let file = fs::OpenOptions::new().write(true).open(d.join("tests/DemoTests.swift")).unwrap();
    file.set_times(fs::FileTimes::new().set_modified(std::time::SystemTime::now() + std::time::Duration::from_secs(60))).unwrap();
    let a = Analysis::load(Some(&d)).unwrap();
    assert!(a.tests_view().modules[0].files[0].tests[0].ids.is_empty());
    assert_eq!(a.report().invalid_tests["comment-only"][0].id, "DEM-01");
    drop(a);
    file.set_times(fs::FileTimes::new().set_modified(std::time::SystemTime::now() - std::time::Duration::from_secs(60))).unwrap();
    let c = Connection::open(d.join(".codegraph/codegraph.db")).unwrap();
    c.execute_batch("update nodes set kind='class' where id='t'").unwrap();
    drop(c);
    let a = Analysis::load(Some(&d)).unwrap();
    assert!(a.tests_view().modules[0].files[0].tests[0].ids.is_empty());
}

#[doc = "[GRAPH-01, GRAPH-04]"]
#[test]
fn codegraph_links_tests_to_code() {
    let d = fixture("graph");
    index(&d, true);
    let a = Analysis::load(Some(&d)).unwrap();
    assert_eq!(a.graph.as_ref().unwrap().version, Some(11));
    assert_eq!(a.links.as_ref().unwrap()["DEM-01"].iter().collect::<Vec<_>>(), ["src/Demo.swift"]);
    assert_eq!(a.verdict("DEM-01"), "ok");
    // a tag on the line above the test function (Rust's `#[doc = "[ID]"]`) links too: `b` starts one line below DEM-05's tag
    // and code in a second src dir links too, shown repo-relative (the first src dir's prefix is dropped)
    write(&d, "spec17.json", r#"{"tests":["tests"],"src":["src","lib"],"ext":["swift"]}"#);
    Connection::open(d.join(".codegraph/codegraph.db")).unwrap().execute_batch("insert into nodes values ('t2','function','b','b','tests/DemoTests.swift',5,6,null), ('n3','function','c','c','lib/Other.swift',1,1,null); insert into edges(source,target,kind) values ('t2','n2','calls'), ('t2','n3','calls');").unwrap();
    let a = Analysis::load(Some(&d)).unwrap();
    assert_eq!(a.links.as_ref().unwrap()["DEM-05"].iter().collect::<Vec<_>>(), ["lib/Other.swift", "src/Demo.swift"]);
    assert_eq!(a.status(&StatusOpts { all: true, ..Default::default() }).unwrap().rows.iter().find(|r| r.id == "DEM-05").unwrap().graph, ["lib/Other.swift", "Demo.swift"]);
    // a dir that is both src and tests (Rust's inline tests): its file is linked and its calls are followed to `d` in src/Deep.swift
    write(&d, "spec17.json", r#"{"tests":["tests","lib"],"src":["src","lib"],"ext":["swift"]}"#);
    Connection::open(d.join(".codegraph/codegraph.db")).unwrap().execute_batch("insert into nodes values ('n4','function','d','d','src/Deep.swift',1,1,null); insert into edges(source,target,kind) values ('n3','n4','calls');").unwrap();
    let a = Analysis::load(Some(&d)).unwrap();
    assert_eq!(a.links.as_ref().unwrap()["DEM-05"].iter().collect::<Vec<_>>(), ["lib/Other.swift", "src/Deep.swift", "src/Demo.swift"]);
}

#[doc = "[GRAPH-02]"]
#[test]
fn codegraph_schema_drift_is_an_error_not_a_panic() {
    let d = fixture("drift");
    index(&d, false);
    match Graph::open(&d.join(".codegraph/codegraph.db")) {
        Err(GraphError::SchemaDrift(m)) => assert!(m.contains("edges lacks kind")),
        other => panic!("{:?}", other.map(|_| ())),
    }
    let a = Analysis::load(Some(&d)).unwrap(); // the rest still works
    assert!(a.graph.is_none() && a.graph_note.unwrap().contains("schema not supported"));
}

#[doc = "[SCAN-09]"]
#[test]
fn every_profile_compiles() {
    for p in spec17_core::detect::PROFILES.iter() {
        for rx in [&p.decl, &p.assert, &p.off, &p.test, &p.case].into_iter().flatten() {
            regex::Regex::new(rx).unwrap_or_else(|e| panic!("{}: {rx}: {e}", p.name));
        }
        assert!(p.decl.iter().all(|s| s.contains('(')), "{}: decl needs a name group", p.name);
    }
    // spec_format.toml: group 1 of a requirement line is the ID, so the patterns may not capture
    let f = &*spec17_core::scan::FORMAT;
    for rx in [&f.level, &f.block_level, &f.done, &f.removed, &f.unnamed].into_iter().chain(&f.id).chain(&f.line).chain(&f.block) {
        let r = regex::Regex::new(rx).unwrap_or_else(|e| panic!("spec_format.toml: {rx}: {e}"));
        assert_eq!(r.captures_len(), 1, "spec_format.toml: {rx} needs (?:...) groups");
    }
}

#[doc = "[SCAN-09]"]
#[test]
fn layout_is_detected_when_spec17_json_is_missing() {
    use spec17_core::detect::{PROFILES, layout, spec_dir};
    let d = std::env::temp_dir().join(format!("spec17-{}-detect", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    write(&d, "docs/specs/01-a.md", "- **AAA-01** MUST — one.\n");
    write(&d, "docs/guide.md", "no requirements here\n");
    write(&d, "pkg/mod.py", "# (AAA-01)\ndef f(): pass\n");
    write(&d, "tests/test_mod.py", "# [AAA\x2D01]\ndef test_f():\n    assert True\n");
    let a = Analysis::load(Some(&d)).unwrap();
    let c = &a.p.cfg;
    assert_eq!((c.specs.as_str(), c.src.clone(), c.tests.clone(), c.ext.clone(), a.p.lang("pkg/mod.py").comment.clone()), ("docs/specs", vec!["pkg".to_string()], vec!["tests".to_string()], vec!["py".to_string()], vec!["#".to_string()]));
    assert_eq!(a.s.len(), 1);
    assert!(a.c.contains_key("AAA-01"));

    // spec folders: a generic docs/ counts only with a requirement bullet; Ruby's spec/ holds tests, not specs
    let allow = |p: &Path| spec17_core::detect::allow_list(p);
    let e = std::env::temp_dir().join(format!("spec17-{}-detect2", std::process::id()));
    let _ = fs::remove_dir_all(&e);
    write(&e, "docs/guide.md", "prose\n");
    write(&e, "spec/a_spec.rb", "describe 1\n");
    assert_eq!(spec_dir(&e, &allow(&e)), None);
    write(&e, "openspec/specs/auth/spec.md", "- **AUT-01** MUST — x.\n");
    assert_eq!(spec_dir(&e, &allow(&e)).as_deref(), Some("openspec/specs"));

    // ecosystems, from file lists alone
    let l = |fs: &[&str]| { let v: Vec<String> = fs.iter().map(|s| s.to_string()).collect(); layout(&PROFILES, &v).map(|l| (l.profile.name.as_str(), l.src, l.tests)) };
    let s = |xs: &[&str]| xs.iter().map(|x| x.to_string()).collect::<Vec<_>>();
    assert_eq!(l(&["RunSolo/RunSolo/A.swift", "RunSolo/RunSoloTests/ATests.swift"]), Some(("swift", s(&["RunSolo"]), s(&["RunSolo/RunSoloTests"]))));
    assert_eq!(l(&["Sources/App/A.swift", "Tests/AppTests/ATests.swift"]), Some(("swift", s(&["Sources"]), s(&["Tests"]))));
    assert_eq!(l(&["core/src/a.rs", "core/tests/t.rs", "server/src/b.rs", "web/src/x.ts"]), Some(("rust", s(&["core/src", "server/src"]), s(&["core/src", "core/tests", "server/src"]))));
    assert_eq!(l(&["cmd/x/main.go", "internal/a.go", "internal/a_test.go"]), Some(("go", s(&["cmd", "internal"]), vec![])));
    assert_eq!(l(&["src/main/java/A.java", "src/test/java/ATest.java"]), Some(("jvm", s(&["src/main"]), s(&["src/test"]))));
    assert_eq!(l(&["web/src/a.ts", "web/src/a.test.ts", "web/e2e/b.ts", "web/node_modules/x/y.js"]), Some(("typescript", s(&["web/src"]), s(&["web/e2e"]))));
    assert_eq!(l(&["lib/a.rb", "spec/a_spec.rb", ".hidden/x.rb"]), Some(("ruby", s(&["lib"]), s(&["spec"]))));
    assert_eq!(l(&["lib/a.dart", "test/a_test.dart"]), Some(("dart", s(&["lib"]), s(&["test"]))));
    assert_eq!(l(&["R/a.R", "tests/testthat/test-a.R"]), Some(("r", s(&["R"]), s(&["tests"]))));
    assert_eq!(l(&["README.md"]), None);

    // colocated tests (Go, TS) are told apart from sources by name, so a citation in a test is not a source citation
    let g = std::env::temp_dir().join(format!("spec17-{}-detect3", std::process::id()));
    let _ = fs::remove_dir_all(&g);
    write(&g, "specs/01.md", "- **GOO-01** MUST — x.\n");
    write(&g, "internal/a.go", "package a\n// (GOO-01)\nfunc A() {}\n");
    write(&g, "internal/a_test.go", "package a\n// [GOO\x2D01]\nfunc TestA(t *testing.T) {\n\tt.Fatal(1)\n}\n");
    let a = Analysis::load(Some(&g)).unwrap();
    assert_eq!((a.p.src_files().to_vec(), a.p.test_files().to_vec()), (vec!["internal/a.go".to_string()], vec!["internal/a_test.go".to_string()]));
    assert_eq!(a.state["GOO-01"], "tested");
}

#[cfg(unix)]
#[doc = "[SCAN-11]"]
#[test]
fn lenient_spec_formats() {
    let d = std::env::temp_dir().join(format!("spec17-{}-lenient", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    write(&d, "specs/a.md", "\
- LEN-01 MUST — plain.
* **LEN-02**: The system SHALL do it.
1. `LEN-03` SHOULD: numbered.
- **LEN-04:** MAY wrap
  onto a line (manual)
| ID | Level | Text |
|---|---|---|
| LEN-05 | MUST | in a table |
- LEN-06 has no level.
");
    let a = Analysis::load(Some(&d)).unwrap();
    let got: Vec<(&str, &str, &str)> = a.s.values().map(|s| (s.id.as_str(), s.level.as_str(), s.text.as_str())).collect();
    assert_eq!(got, [("LEN-01", "MUST", "plain."), ("LEN-02", "SHALL", "The system SHALL do it."), ("LEN-03", "SHOULD", "numbered."), ("LEN-04", "MAY", "wrap onto a line (manual)"), ("LEN-05", "MUST", "in a table")]);
    assert_eq!(a.s["LEN-04"].mark, "manual");
    assert!(a.issues.iter().any(|x| x.id == "LEN-06" && x.uncounted));
}

/// A small repo of `core/test-data` (see its README).
fn data(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("test-data").join(name)
}

#[doc = "[SCAN-23, SCAN-24, SCAN-25]"]
#[test]
fn every_requirement_style_of_the_test_data_parses() {
    let a = Analysis::load(Some(&data("styles"))).unwrap();
    let got: Vec<(&str, &str)> = a.s.values().map(|s| (s.id.as_str(), s.level.as_str())).collect();
    assert_eq!(got, [
        ("STY-01", "MUST"), ("STY-02", "SHALL"), ("STY-03", "SHOULD"), ("STY-04", "MAY"), ("STY-05", "MUST"), ("HTTP-01", "MUST"),
        ("HDG-1", "SHALL"), ("HDG-2", "SHALL"), ("RUL-1", "MUST"),
        ("PAR-1.1", "SHALL"), ("PAR-1.2", "SHALL"), ("PAR-1.10", "SHALL"),
        ("FR-001", "MUST"), ("FR-002", "MUST"), ("NFR-001", "SHOULD"),
        ("REQ_001", "MUST"), ("REQ_002", "SHOULD"), ("REQ_003.1", "MUST"), ("REQ_003.2", "MAY"),
        ("UNI-01", "MUST"), ("UNI-02", "MUST"), ("UNI-03", "SHOULD"),
    ]);
    // the negatives (A1 table rows, B-1, **Covers:**, UTF-8, ISO-8601, TLS-1.3, a level-less mention of STY-01) raise nothing
    assert!(a.issues.is_empty(), "{:?}", a.issues);
    assert_eq!(a.s["HDG-1"].text, "Input directory from a property **Covers:** B-1 When the application starts with `importer.input-dir` set, the importer shall use that path.");
    assert_eq!(a.s["PAR-1.1"].text, "Happy path WHEN the input directory has CSV files THEN the batch job executes SHALL read all `*.csv` files");
    assert!(a.s["UNI-01"].text.contains("первая строка") && a.s["UNI-01"].text.contains("日本語, café"));
    assert!(a.s["UNI-02"].done);
    // new ID forms trace end to end: test tags and source citations, a pasted non-breaking hyphen too
    let state = |i: &str| a.state[i].as_str();
    assert_eq!([state("REQ_001"), state("PAR-1.2"), state("FR-001"), state("HDG-1"), state("UNI-03"), state("REQ_002"), state("REQ_003.1"), state("REQ_003.2")], ["tested", "tested", "tested", "src-only", "src-only", "none", "tested", "none"]);
    assert!(!a.t.contains_key("UTF-8"));
    let r = a.report();
    assert!(r.invalid_tests.values().all(Vec::is_empty), "{:?}", r.invalid_tests);
    assert_eq!(spec17_core::scan::sorted_ids(a.s.keys().filter(|i| i.starts_with("PAR"))), ["PAR-1.1", "PAR-1.2", "PAR-1.10"]);
    assert_eq!(spec17_core::scan::ranges(["FR-001", "FR-002", "REQ_001", "REQ_002", "SEC-4"]), "FR-001..002,REQ_001..002,SEC-04");
    assert_eq!(spec17_core::scan::ranges(["PAR-1.1", "PAR-1.2", "HDG-1"]), "PAR-1.1..2,HDG-01");
    assert_eq!(spec17_core::scan::sorted_ids(a.s.keys().filter(|i| i.starts_with("REQ_003"))), ["REQ_003.1", "REQ_003.2"]);
    assert_eq!(spec17_core::scan::ranges(["REQ_003.1", "REQ_003.2"]), "REQ_003.1..2");
    assert_eq!((a.s["HDG-1"].line, a.s["HDG-1"].end), (3, 6)); // a heading runs to the next heading, trailing blank lines left out
    // a heading with an ID but no level is reported, not counted
    let d = scratch("blocks");
    write(&d, "specs/a.md", "### DEM\x2D1: no level here\n\nJust prose.\n");
    let un: Vec<String> = Analysis::load(Some(&d)).unwrap().issues.iter().filter(|x| x.uncounted).map(|x| x.id.clone()).collect();
    assert_eq!(un, ["DEM-1"]);
}

#[doc = "[SCAN-26, SCAN-27]"]
#[test]
fn openspec_superpowers_kiro_and_fallback_layouts() {
    // spec/ and OpenSpec together; a change modifies, adds and removes; archive/ and tasks.md are not specs
    let a = Analysis::load(Some(&data("openspec"))).unwrap();
    assert_eq!(a.s.keys().collect::<Vec<_>>(), ["CRT-01", "UIX-01", "UIX-03"]);
    assert_eq!(a.s["UIX-01"].file, "openspec/changes/add-chart/specs/ui/spec.md");
    assert!(a.s["UIX-01"].text.ends_with("with a chart canvas."));
    assert_eq!(a.p.retired(), ["UIX-02"]);
    let issues: Vec<String> = a.issues.iter().map(|x| x.to_string()).collect();
    assert_eq!(issues, ["Requirement: Filter form without an ID openspec/specs/ui/spec.md:18 is a requirement without an ID, so no test can cite it; it is not counted"]);

    // Superpowers: design docs next to specs/, never plans/
    let a = Analysis::load(Some(&data("superpowers"))).unwrap();
    assert_eq!(a.s.keys().collect::<Vec<_>>(), ["MAN-01", "SUP-01"]);
    assert_eq!(a.p.cfg.specs, "specs");

    // Kiro numbers its requirements but gives them no ID
    let a = Analysis::load(Some(&data("kiro"))).unwrap();
    assert!(a.s.is_empty());
    assert!(a.issues.iter().any(|x| x.id == "Requirement 1" && x.uncounted), "{:?}", a.issues);

    // a struck bullet retires only that line: a live definition elsewhere still counts (tests on it are flagged)
    let d = scratch("struck");
    write(&d, "specs/a.md", "- **DEM-01** MUST — live.\n");
    write(&d, "specs/b.md", "- ~~**DEM-01** MUST — old wording.~~\n");
    assert!(Analysis::load(Some(&d)).unwrap().s.contains_key("DEM-01"));

    // a spec folder that a symlink takes out of the repo is not read (SCAN-10)
    #[cfg(unix)]
    {
        let (d, out) = (scratch("os-link"), scratch("os-out"));
        write(&d, "specs/a.md", "- **DEM-01** MUST — in.\n");
        write(&out, "specs/x.md", "- **DEM-02** MUST — outside.\n");
        fs::create_dir_all(d.join("openspec/changes")).unwrap();
        std::os::unix::fs::symlink(&out, d.join("openspec/changes/x")).unwrap();
        assert_eq!(Analysis::load(Some(&d)).unwrap().s.keys().collect::<Vec<_>>(), ["DEM-01"]);
    }

    // no spec folder: any markdown with a requirement, a heading block with its level further down too
    let a = Analysis::load(Some(&data("fallback"))).unwrap();
    assert_eq!(a.s.values().map(|s| (s.id.as_str(), s.file.as_str())).collect::<Vec<_>>(), [("FBK-1", "notes/design.md")]);

    // change polling reads no spec file, yet lists every file `open` reads the requirements from
    for name in ["styles", "openspec", "superpowers", "kiro", "fallback"] {
        let (full, unread) = (Project::open(Some(&data(name))).unwrap(), Project::open_unread(Some(&data(name))).unwrap());
        assert!(full.spec_files().iter().all(|f| unread.spec_files().contains(f)), "{name}: {:?} vs {:?}", full.spec_files(), unread.spec_files());
    }
}

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("spec17-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    d
}

#[doc = "[SCAN-12]"]
#[test]
fn specs_in_a_single_file_or_anywhere() {
    let one = scratch("onefile");
    write(&one, "SPEC.md", "- REQ-0001 MUST — one.\n");
    let a = Analysis::load(Some(&one)).unwrap();
    assert_eq!(a.s.keys().collect::<Vec<_>>(), ["REQ-0001"]);
    assert_eq!(a.s["REQ-0001"].file, "SPEC.md");

    let named = scratch("namedfile");
    write(&named, "spec17.json", r#"{"specs":"design/all.md"}"#);
    write(&named, "design/all.md", "1. NAM-01 SHALL do it.\n");
    assert_eq!(Analysis::load(Some(&named)).unwrap().s["NAM-01"].file, "design/all.md");

    let any = scratch("anymd");
    write(&any, "notes/deep/a.md", "- **ANY-01** MUST — somewhere.\n");
    let a = Analysis::load(Some(&any)).unwrap();
    assert_eq!(a.s["ANY-01"].file, "notes/deep/a.md");
}

#[doc = "[SCAN-13]"]
#[test]
fn linked_docs_and_adrs_are_reference_docs() {
    let d = scratch("adr");
    write(&d, "spec17.json", r#"{"src":["src"],"tests":["tests"],"ext":["kt"]}"#);
    write(&d, "docs/arch/drivers.md", "\
The fold ([state-machine.md](state-machine.md)) decides nothing; see
[ADR\x2D0003](../../decisions/0003-patterns.md) \u{a7}5 and [ADR\x2D0009](../../decisions/0009-gone.md).
Also [site](https://example.com/ADR\x2D0004.md).
");
    write(&d, "docs/arch/state-machine.md", "# Fold\n");
    write(&d, "decisions/0003-patterns.md", "# Architectural patterns\n");
    write(&d, "decisions/0007-other.md", "# Other\n");
    write(&d, "src/A.kt", "// (ADR-0003) the driver\nclass A\n");
    let a = Analysis::load(Some(&d)).unwrap();
    assert_eq!(a.s.keys().collect::<Vec<_>>(), ["ADR-0003", "ADR-0007"]);
    let adr = &a.s["ADR-0003"];
    assert_eq!((adr.file.as_str(), adr.level.as_str(), adr.text.as_str(), a.state["ADR-0003"].as_str()), ("decisions/0003-patterns.md", "DOC", "Architectural patterns", "ref"));
    assert!(a.issues.iter().any(|x| x.id == "ADR-0009" && x.msg.contains("does not exist")), "{:?}", a.issues);
    let r = a.report();
    assert_eq!((r.requirements, r.refs, r.due), (0, 2, 0));
    let t = a.trace("ADR-0003").unwrap();
    assert_eq!((t.linked_from.len(), t.source_cites.len()), (1, 1));
    assert_eq!(t.linked_from[0].file, "docs/arch/drivers.md");
}

#[doc = "[SCAN-14, GRAPH-05]"]
#[test]
fn modules_and_test_cases() {
    let d = scratch("modules");
    write(&d, "spec17.json", r#"{"specs":"SPEC.md","src":["a/src","b/src","web/src"],"tests":["a/tests","b/src"],"ext":["rs","ts"],"profiles":[{"name":"typescript","test":"\\.test\\.ts$"}]}"#);
    write(&d, "SPEC.md", "- MOD-01 MUST — x.\n");
    for m in ["a", "b"] {
        write(&d, &format!("{m}/Cargo.toml"), "[package]\n");
        write(&d, &format!("{m}/src/lib.rs"), "pub fn f() {}\n");
    }
    // unit tests inline in `src`: source that declares a test; a marker in a string is not a test
    write(&d, "b/src/lib.rs", "pub fn f() {}\n#[test]\nfn inline() {}\nconst S: &str = \"#[test]\\nfn fake() {}\";\n");
    write(&d, "web/package.json", "{}\n");
    write(&d, "a/tests/t.rs", "#[doc = \"[MOD\x2D01]\"]\n#[test]\nfn works() {\n    assert!(true);\n}\n\n#[test]\nfn plain() {}\n");
    write(&d, "web/src/x.test.ts", "describe('x', () => {\n  it('does x', () => {});\n});\n");
    let a = Analysis::load(Some(&d)).unwrap();
    let t = a.tests_view();
    assert_eq!(t.total, 4);
    let names = |m: &str| t.modules.iter().find(|x| x.module == m).unwrap().files[0].tests.iter().map(|c| (c.name.clone(), c.line, c.ids.clone())).collect::<Vec<_>>();
    assert_eq!(names("a"), [("works".to_string(), 2, vec!["MOD-01".to_string()]), ("plain".to_string(), 7, vec![])]);
    assert_eq!(names("b"), [("inline".to_string(), 2, vec![])]);
    assert_eq!(names("web"), [("does x".to_string(), 2, vec![])]);

    let m = a.modules_view();
    assert!(!m.index && m.note.unwrap().contains("no CodeGraph index"));
    let by = |p: &str| m.modules.iter().find(|x| x.path == p).unwrap();
    assert_eq!((by("a").manifest.as_deref(), by("a").src_files, by("a").test_files, by("a").tests), (Some("Cargo.toml"), 1, 1, 2));
    assert_eq!((by("b").src_files, by("b").test_files, by("b").tests), (1, 0, 1));
    assert_eq!((by("web").manifest.as_deref(), by("web").languages.clone()), (Some("package.json"), vec!["typescript".to_string()]));

    // with an index: symbols per module and the dependency between modules
    fs::create_dir_all(d.join(".codegraph")).unwrap();
    Connection::open(d.join(".codegraph/codegraph.db")).unwrap().execute_batch("
        create table nodes(id text primary key, kind text, name text, qualified_name text, file_path text, start_line int, end_line int, visibility text);
        create table edges(id integer primary key, source text, target text, kind text);
        create table schema_versions(version int, applied_at int, description text);
        insert into schema_versions values (11, 0, 'x');
        insert into nodes values ('a1','function','f','a::f','a/src/lib.rs',1,1,null), ('b1','function','f','b::f','b/src/lib.rs',1,1,null),
                                 ('w1','function','works','a::works','a/tests/t.rs',3,5,null);
        insert into edges(source,target,kind) values ('a1','b1','calls'), ('w1','a1','calls');
    ").unwrap();
    let m = Analysis::load(Some(&d)).unwrap().modules_view();
    assert!(m.index);
    let a = m.modules.iter().find(|x| x.path == "a").unwrap();
    assert_eq!((a.symbols["function"], a.depends_on.iter().map(|x| (x.module.as_str(), x.edges)).collect::<Vec<_>>()), (2, vec![("b", 1)]));
    let q = Analysis::load(Some(&d)).unwrap().tests_view();
    assert_eq!(q.modules.iter().find(|x| x.module == "a").unwrap().files[0].tests[0].qualified, "a::works");
}

#[doc = "[SCAN-10]"]
#[test]
fn symlinks_are_followed_only_inside_root_or_allowed_paths() {
    let d = std::env::temp_dir().join(format!("spec17-{}-links", std::process::id()));
    let out = std::env::temp_dir().join(format!("spec17-{}-links-out", std::process::id()));
    let ok = std::env::temp_dir().join(format!("spec17-{}-links-ok", std::process::id()));
    for x in [&d, &out, &ok] {
        let _ = fs::remove_dir_all(x);
    }
    write(&d, "specs/01.md", "- **LNK-01** MUST — x.\n");
    write(&d, "src/a.py", "# (LNK-01)\n");
    write(&d, "real/b.py", "# (LNK-01)\n");
    write(&out, "c.py", "# (LNK-01)\n");
    write(&ok, "e.py", "# (LNK-01)\n");
    write(&d, "spec17.json", r#"{"src":["src"],"tests":["tests"],"ext":["py"]}"#);
    let (inside, outside, allowed) = (d.join("src/inside"), d.join("src/outside"), d.join("src/allowed"));
    std::os::unix::fs::symlink(d.join("real"), &inside).unwrap();
    std::os::unix::fs::symlink(&out, &outside).unwrap();
    std::os::unix::fs::symlink(&ok, &allowed).unwrap();
    std::os::unix::fs::symlink(out.join("c.py"), d.join("src/file.py")).unwrap(); // a file link, too
    let files = || Analysis::load(Some(&d)).unwrap().p.src_files().to_vec();
    unsafe { std::env::remove_var("ALLOWED_PATHS") };
    assert_eq!(files(), ["src/a.py", "src/inside/b.py"]);
    unsafe { std::env::set_var("ALLOWED_PATHS", std::env::join_paths([&ok]).unwrap()) };
    assert_eq!(files(), ["src/a.py", "src/allowed/e.py", "src/inside/b.py"]);
    unsafe { std::env::remove_var("ALLOWED_PATHS") };
}

#[doc = "[SCAN-15]"]
#[test]
fn ignore_files_and_agents_are_left_out() {
    let d = scratch("ignored");
    write(&d, "spec17.json", r#"{"src":["src"],"tests":["tests"],"ext":["kt"]}"#);
    write(&d, "specs/a.md", "- AAA-01 MUST — one.\n");
    write(&d, ".aiignore", "# generated\nsrc/gen/\n");
    write(&d, ".spec17ignore", "src/skip.kt\n*.tmp.kt\n!src/keep.tmp.kt\n");
    for f in ["src/a.kt", "src/skip.kt", "src/x.tmp.kt", "src/keep.tmp.kt", "src/gen/g.kt", "src/.agents/s.kt"] {
        write(&d, f, "class X\n");
    }
    let p = spec17_core::scan::Project::open(Some(&d)).unwrap();
    assert_eq!(p.src_files(), ["src/a.kt", "src/keep.tmp.kt"]);
    for hidden in ["src/skip.kt", "src/gen/g.kt", ".agents/skills/spec17/SKILL.md", "src/.agents"] {
        assert!(p.ignored(hidden), "{hidden}");
    }
    assert!(!p.ignored("src/a.kt") && !p.ignored("src/keep.tmp.kt"));
}

#[doc = "[SCAN-16]"]
#[test]
fn hunks_map_to_changed_lines() {
    let h = spec17_core::changes::parse_hunks("--- a/specs/a.md\n+++ b/specs/a.md\n@@ -1,0 +2,3 @@\n+x\n+y\n+z\n@@ -9 +11 @@\n-old\n+new\n--- a/gone.rs\n+++ /dev/null\n@@ -5,2 +4,0 @@\n-a\n-b\n");
    let a = &h["specs/a.md"];
    assert_eq!((a[0].new_start, a[0].new_len, a[0].added.len()), (2, 3, 3));
    assert_eq!((a[1].new_start, a[1].removed.as_slice(), a[1].added.as_slice()), (11, ["old".to_string()].as_slice(), ["new".to_string()].as_slice()));
    // a deleted file is keyed by its old path, and a pure deletion has no new lines
    assert_eq!((h["gone.rs"][0].new_start, h["gone.rs"][0].new_len, h["gone.rs"][0].removed.len()), (4, 0, 2));
}

#[doc = "[SCAN-21]"]
#[test]
fn config_paths_cannot_leave_the_repo() {
    let d = std::env::temp_dir().join(format!("spec17-{}-confined", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    write(&d, "specs/01-a.md", "- **AAA-01** MUST — one.\n");
    for (field, value) in [("src", r#"["../outside"]"#), ("tests", r#"["/etc"]"#), ("codegraph", r#""../x.db""#), ("specs", r#""../specs""#)] {
        write(&d, "spec17.json", &format!(r#"{{"{field}": {value}}}"#));
        let err = Analysis::load(Some(&d)).err().unwrap_or_else(|| panic!("{field} outside the repo must be refused"));
        assert!(err.contains(field) && err.contains("outside the repo"), "{err}");
    }
    write(&d, "spec17.json", r#"{"src": ["src"], "tests": ["missing/tests"]}"#); // inside, even if absent: doctor says "not found"
    assert!(Analysis::load(Some(&d)).is_ok());
}

#[doc = "[SCAN-17]"]
#[test]
fn consecutive_ids_print_as_a_range() {
    assert_eq!(spec17_core::scan::ranges(["DOC-01", "DOC-02", "DOC-03", "DOC-07", "SEC-4"]), "DOC-01..03,DOC-07,SEC-04");
}

#[doc = "[SCAN-08]"]
#[test]
fn a_tests_body_stops_at_its_closing_brace() {
    let ls: Vec<String> = ["@Test(\"[A-1]\")", "func f() {", "  x()", "}", "other"].iter().map(|s| s.to_string()).collect();
    assert_eq!(spec17_core::scan::body(&ls, 1, 200).len(), 4);
}

#[doc = "[SCAN-35]"]
#[test]
fn test_body_ignores_braces_in_rust_raw_strings_and_comments() {
    let ls: Vec<String> = ["fn test_example() {", "  let x = r##\"first \" }", "  { second\"##;", "  /* }", "  { */", "  assert!(true);", "}", "after"].into_iter().map(str::to_string).collect();
    assert_eq!(spec17_core::scan::body(&ls, 1, 200).len(), 7);
}

#[doc = "[SCAN-35]"]
#[test]
fn test_body_ignores_braces_in_multiline_quoted_strings() {
    let ls: Vec<String> = ["func testExample() {", "  let x = \"\"\"", "  a \" }", "  \"\"\"", "  XCTAssertTrue(true)", "}", "after"].into_iter().map(str::to_string).collect();
    assert_eq!(spec17_core::scan::body(&ls, 1, 200).len(), 6);
}

#[doc = "[SCAN-18, SCAN-19]"]
#[test]
fn changes_split_staged_unstaged_and_untracked() {
    use spec17_core::ChangeOpts;
    let d = scratch("changes");
    write(&d, "spec17.json", r#"{"src":["src"],"tests":["tests"],"ext":["kt"]}"#);
    write(&d, "specs/a.md", "- **AAA-01** MUST — one.\n- **AAA-02** MUST — two.\n");
    write(&d, "src/a.kt", "// (AAA-01)\nclass A\n");
    write(&d, "src/b.kt", "// (AAA-02)\nclass B\n");
    let git = |a: &[&str]| assert!(std::process::Command::new("git").args(["-c", "user.name=t", "-c", "user.email=t@t"]).args(a).current_dir(&d).output().unwrap().status.success(), "{a:?}");
    git(&["init", "-q"]);
    git(&["add", "."]);
    git(&["commit", "-qm", "x"]);
    write(&d, "src/a.kt", "// (AAA-01)\nclass A2\n"); // staged below
    git(&["add", "src/a.kt"]);
    write(&d, "specs/a.md", "- **AAA-01** MUST — one.\n- **AAA-02** MUST — two, edited.\n"); // unstaged
    write(&d, "src/c.kt", "// (AAA-01)\nclass C\n"); // untracked
    let changes = |o: ChangeOpts| Analysis::load(Some(&d)).unwrap().changes(&o).unwrap();
    let states = |c: &spec17_core::Changes| c.files.iter().map(|f| format!("{}:{}:{}:{}", f.path, f.change, f.stage, f.kind)).collect::<Vec<_>>();
    let all = changes(ChangeOpts::default());
    assert_eq!(states(&all), ["specs/a.md:modified:unstaged:spec", "src/a.kt:modified:staged:src", "src/c.kt:added:untracked:src"]);
    assert_eq!(all.requirements.iter().map(|r| (r.id.as_str(), r.changed.join(","))).collect::<Vec<_>>(), [("AAA-01", "src".to_string()), ("AAA-02", "spec".to_string())]);
    let tracked = changes(ChangeOpts { no_untracked: true, ..Default::default() });
    assert_eq!(states(&tracked), ["specs/a.md:modified:unstaged:spec", "src/a.kt:modified:staged:src"]);
    let staged = changes(ChangeOpts { staged: true, ..Default::default() });
    assert_eq!(states(&staged), ["src/a.kt:modified:staged:src"]);
    assert_eq!(staged.requirements.len(), 1);
    let a = Analysis::load(Some(&d)).unwrap();
    let o = ChangeOpts::default();
    let c = a.file_diff("src/c.kt", &o).unwrap();
    assert!(c.change == "added" && c.diff.contains("+class C") && c.diff.contains("@@ -0,0 +1,2 @@"), "{}", c.diff);
    assert!(a.file_diff("src/a.kt", &o).unwrap().diff.contains("+class A2"));
    assert!(a.file_diff("src/b.kt", &o).is_err() && a.file_diff("../x", &o).is_err());
    // status takes the same switches
    let o = StatusOpts { staged: true, ..Default::default() };
    assert_eq!(Analysis::load(Some(&d)).unwrap().status(&o).unwrap().rows.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(), ["AAA-01"]);
    let o = StatusOpts { changed: Some("HEAD".into()), no_untracked: true, ..Default::default() };
    assert_eq!(Analysis::load(Some(&d)).unwrap().status(&o).unwrap().rows.len(), 2);
}

#[doc = "[SCAN-22]"]
#[test]
fn changes_list_requirements_whose_tests_reach_a_changed_file() {
    use spec17_core::ChangeOpts;
    let d = fixture("reach");
    index(&d, true);
    let git = |a: &[&str]| assert!(std::process::Command::new("git").args(["-c", "user.name=t", "-c", "user.email=t@t"]).args(a).current_dir(&d).output().unwrap().status.success(), "{a:?}");
    git(&["init", "-q"]);
    git(&["add", "."]);
    git(&["commit", "-qm", "x"]);
    // the test tagged DEM-01 calls Demo.run; the edit is far from the DEM-02 citation
    write(&d, "src/Demo.swift", "/// (DEM-02) the rule\nstruct Demo {\n    func run() { print(1) }\n}\n// DEM-98 typo\n");
    let c = Analysis::load(Some(&d)).unwrap().changes(&ChangeOpts { no_untracked: true, ..Default::default() }).unwrap();
    let r = c.requirements.iter().find(|r| r.id == "DEM-01").expect("DEM-01 is reached by its test");
    assert!(r.notes.iter().any(|n| n.role == "src" && n.action == "reached by a test" && n.file == "src/Demo.swift"), "{:?}", r.notes);
    // without an index nothing is reached
    fs::remove_dir_all(d.join(".codegraph")).unwrap();
    let c = Analysis::load(Some(&d)).unwrap().changes(&ChangeOpts { no_untracked: true, ..Default::default() }).unwrap();
    assert!(c.requirements.iter().all(|r| r.id != "DEM-01"));
}

#[doc = "[SCAN-18, SCAN-19, SCAN-20]"]
#[test]
fn changes_by_ref_commit_rename_delete_whitespace_and_history() {
    use spec17_core::ChangeOpts;
    let d = scratch("changes2");
    write(&d, "spec17.json", r#"{"src":["src"],"tests":["tests"],"ext":["kt"]}"#);
    write(&d, ".spec17ignore", "hidden/\n");
    write(&d, "specs/a.md", "- **AAA-01** MUST — one.\n- **AAA-02** MUST — two.\n- **AAA-03** MUST — three.\n");
    write(&d, "tests/AT.kt", "// [AAA\x2D01]\nfun t1() {\n    check(1)\n}\n\n// [AAA\x2D02]\nfun t2() {\n    check(2)\n}\n");
    write(&d, "src/old.kt", "// (AAA-01)\nfun f() {\n    1\n}\n");
    write(&d, "notes.txt", "n\n");
    write(&d, "hidden/h.kt", "x\n");
    let git = |a: &[&str]| {
        let r = std::process::Command::new("git").args(["-c", "user.name=t", "-c", "user.email=t@t"]).args(a).current_dir(&d).output().unwrap();
        assert!(r.status.success(), "{a:?}");
        String::from_utf8_lossy(&r.stdout).trim().to_string()
    };
    git(&["init", "-q", "-b", "main"]);
    git(&["add", "."]);
    git(&["commit", "-qm", "base"]);
    git(&["checkout", "-qb", "feature"]);
    // commit 1: edit the AAA-02 test only, drop AAA-03, rename old.kt
    write(&d, "tests/AT.kt", "// [AAA\x2D01]\nfun t1() {\n    check(1)\n}\n\n// [AAA\x2D02]\nfun t2() {\n    check(22)\n}\n");
    write(&d, "specs/a.md", "- **AAA-01** MUST — one.\n- **AAA-02** MUST — two.\n");
    git(&["mv", "src/old.kt", "src/new.kt"]);
    write(&d, "notes.txt", "n2\n");
    write(&d, "hidden/h.kt", "y\n");
    git(&["add", "."]);
    git(&["commit", "-qm", "work"]);
    let sha = git(&["rev-parse", "HEAD"]);
    // working tree: whitespace-only edit
    write(&d, "src/new.kt", "// (AAA-01)\nfun f() {\n      1\n}\n");
    let a = Analysis::load(Some(&d)).unwrap();
    let ids = |c: &spec17_core::Changes| c.requirements.iter().map(|r| format!("{}:{}", r.id, r.changed.join("+"))).collect::<Vec<_>>();
    let paths = |c: &spec17_core::Changes| c.files.iter().map(|f| format!("{}:{}:{}", f.path, f.change, f.stage)).collect::<Vec<_>>();

    // against HEAD: only the whitespace edit; ignoring whitespace leaves nothing
    let head = a.changes(&ChangeOpts::default()).unwrap();
    assert_eq!(paths(&head), ["src/new.kt:modified:unstaged"]);
    assert!(a.changes(&ChangeOpts { ignore_whitespace: true, ..Default::default() }).unwrap().files.is_empty());

    // against main: committed + working tree; renames, deleted requirement, one test touched (not the other)
    let o = ChangeOpts { reference: "main".into(), ..Default::default() };
    let c = a.changes(&o).unwrap();
    assert_eq!(paths(&c), ["specs/a.md:modified:committed", "src/new.kt:renamed:unstaged", "tests/AT.kt:modified:committed"]);
    assert_eq!(c.files[1].old_path.as_deref(), Some("src/old.kt"));
    // (the whitespace-only edit of the renamed cited function is no change: SCAN-32)
    assert_eq!(ids(&c), ["AAA-02:test", "AAA-03:spec"], "{c:?}");
    let gone = c.requirements.iter().find(|r| r.id == "AAA-03").unwrap();
    assert!(gone.deleted && gone.state == "deleted" && gone.text.contains("three"), "{gone:?}");
    assert!(c.text().contains("(from src/old.kt)"));
    // show_ignored and show_all widen the list
    let wide = a.changes(&ChangeOpts { show_ignored: true, show_all: true, ..o.clone() }).unwrap();
    assert!(paths(&wide).contains(&"hidden/h.kt:modified:committed".to_string()) && paths(&wide).contains(&"notes.txt:modified:committed".to_string()), "{:?}", paths(&wide));
    assert!(!paths(&c).iter().any(|p| p.starts_with("notes.txt") || p.starts_with("hidden")));

    // the diff of the test file says which requirement each hunk touches
    let fd = a.file_diff("tests/AT.kt", &o).unwrap();
    assert!(fd.diff.contains("+    check(22)") && fd.hunks.len() == 1, "{}", fd.diff);
    assert_eq!(fd.hunks[0].requirements.iter().map(|r| format!("{}:{}", r.id, r.role)).collect::<Vec<_>>(), ["AAA-02:test"]);
    assert!(fd.text().contains("↳ AAA-02 test changed"), "{}", fd.text());
    assert!(a.file_diff("src/new.kt", &o).unwrap().diff.contains("rename from src/old.kt"));
    assert!(a.file_diff("notes.txt", &o).is_err(), "an unlisted (non-scanned) file is refused");

    // one commit explained
    let one = a.changes(&ChangeOpts { commit: sha.clone(), ..Default::default() }).unwrap();
    assert_eq!(one.commit.as_deref(), Some(sha.as_str()));
    assert_eq!(ids(&one), ["AAA-02:test", "AAA-03:spec"]);
    assert!(one.files.iter().all(|f| f.stage == "committed"));
    assert!(a.changes(&ChangeOpts { commit: "--output=x".into(), ..Default::default() }).is_err());

    // history: the branch commit is not on main
    let h = a.p.history("", "", 10).unwrap();
    assert_eq!((h.base.as_deref(), h.on_base, h.commits.len(), h.commits[0].subject.as_str()), (Some("main"), false, 1, "work"));
    assert_eq!(h.uncommitted, 1);
    // the filters apply: the branch commit touches scanned files, so it stays
    let h = a.history("", "", 10, &ChangeOpts::default()).unwrap();
    assert_eq!(h.commits.len(), 1);
    git(&["checkout", "-qf", "main"]);
    let h = Analysis::load(Some(&d)).unwrap().p.history("", "", 10).unwrap();
    assert!(h.on_base && h.commits[0].subject == "base");
}

#[doc = "[SCAN-20]"]
#[test]
fn history_leaves_out_commits_the_filters_hide() {
    use spec17_core::ChangeOpts;
    let d = scratch("history_filters");
    write(&d, "spec17.json", r#"{"src":["src"],"tests":["tests"],"ext":["kt"]}"#);
    write(&d, "specs/a.md", "- **AAA-01** MUST — one.\n");
    write(&d, "notes.txt", "n\n");
    let git = |a: &[&str]| {
        let r = std::process::Command::new("git").args(["-c", "user.name=t", "-c", "user.email=t@t"]).args(a).current_dir(&d).output().unwrap();
        assert!(r.status.success(), "{a:?}");
    };
    git(&["init", "-q", "-b", "main"]);
    git(&["add", "."]);
    git(&["commit", "-qm", "base"]);
    git(&["checkout", "-qb", "feature"]);
    write(&d, "notes.txt", "n2\n");
    git(&["commit", "-qam", "notes only"]);
    write(&d, "specs/a.md", "- **AAA-01** MUST — one!\n");
    git(&["commit", "-qam", "spec"]);
    let a = Analysis::load(Some(&d)).unwrap();
    let subjects = |o: &ChangeOpts| a.history("", "", 10, o).unwrap().commits.into_iter().map(|c| c.subject).collect::<Vec<_>>();
    assert_eq!(subjects(&ChangeOpts::default()), ["spec"]);
    assert_eq!(subjects(&ChangeOpts { show_all: true, ..Default::default() }), ["spec", "notes only"]);
}

#[doc = "[SCAN-18]"]
#[test]
fn braces_in_strings_do_not_stretch_a_unit() {
    use spec17_core::ChangeOpts;
    let d = scratch("braces");
    write(&d, "spec17.json", r#"{"src":["src"],"tests":["tests"],"ext":["kt"]}"#);
    write(&d, "specs/a.md", "- **AAA-01** MUST — one.\n- **AAA-02** MUST — two.\n");
    // the first test opens a brace inside a string and never closes it in code
    let t = |b: &str| format!("// [AAA\\x2D01]\nfun t1() {{\n    check(\"{{\")  // {{\n}}\n\n// [AAA\\x2D02]\nfun t2() {{\n    check({b})\n}}\n").replace("\\x2D", "-");
    write(&d, "tests/AT.kt", &t("1"));
    let git = |a: &[&str]| assert!(std::process::Command::new("git").args(["-c", "user.name=t", "-c", "user.email=t@t"]).args(a).current_dir(&d).output().unwrap().status.success(), "{a:?}");
    git(&["init", "-q"]);
    git(&["add", "."]);
    git(&["commit", "-qm", "x"]);
    write(&d, "tests/AT.kt", &t("2"));
    let c = Analysis::load(Some(&d)).unwrap().changes(&ChangeOpts::default()).unwrap();
    assert_eq!(c.requirements.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(), ["AAA-02"], "{c:?}");
}

#[doc = "[SCAN-28, GRAPH-06]"]
#[test]
fn a_repo_with_no_specs_still_opens_and_lists_its_sources_and_tests() {
    let a = Analysis::load(Some(&data("nospecs"))).unwrap();
    let r = a.report();
    assert_eq!((r.requirements, r.spec_files, r.refs), (0, 0, 0));
    assert!(a.p.spec_files().is_empty() && a.p.spec_paths().is_empty());
    // no filter: nothing to list is not an error; a filter that matches nothing still is
    assert!(a.status(&StatusOpts { all: true, ..Default::default() }).unwrap().rows.is_empty());
    assert!(a.status(&StatusOpts { prefix: Some("NOPE".into()), ..Default::default() }).is_err());
    assert!(a.changes(&Default::default()).is_ok());
    assert!(r.text(&a, 12).contains("note: no specs found"));
    let m = a.modules_view();
    let src = m.modules.iter().find(|x| x.sources == ["src/greeter.py"]).expect("the source file is listed");
    assert_eq!(src.src_files, 1);
    assert_eq!((m.modules.iter().map(|x| x.test_files).sum::<usize>(), m.modules.iter().map(|x| x.tests).sum::<usize>()), (1, 1));
    assert_eq!(a.tests_view().total, 1);
}

#[doc = "[SCAN-28]"]
#[test]
fn a_configured_specs_path_that_is_missing_counts_as_no_specs() {
    let d = std::env::temp_dir().join(format!("spec17-{}-nospec-cfg", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    write(&d, "spec17.json", r#"{"specs":"nothing-here"}"#);
    write(&d, "src/a.py", "def a():\n    pass\n");
    assert_eq!(Analysis::load(Some(&d)).unwrap().report().requirements, 0);
}

#[doc = "[SCAN-30, SCAN-31]"]
#[test]
fn a_requirement_names_the_newest_commit_that_touched_it() {
    let d = scratch("blame");
    write(&d, "specs/01-a.md", "- **BLM-01** MUST — one.\n- **BLM-02** MUST — two.\n");
    let git = |a: &[&str]| assert!(std::process::Command::new("git").args(["-c", "user.name=Ann Author", "-c", "user.email=t@t"]).args(a).current_dir(&d).output().unwrap().status.success(), "{a:?}");
    git(&["init", "-q"]);
    git(&["add", "."]);
    git(&["commit", "-qm", "one"]);
    write(&d, "specs/01-a.md", "- **BLM-01** MUST — one.\n- **BLM-02** MUST — two, edited.\n");
    git(&["commit", "-qam", "two"]);
    let a = Analysis::load(Some(&d)).unwrap();
    let c = a.blame_commits().unwrap();
    assert!(c["BLM-01"].sha.len() == 40 && c["BLM-01"].sha != c["BLM-02"].sha, "{c:?}");
    assert_eq!((c["BLM-02"].author.as_str(), c["BLM-02"].subject.as_str(), c["BLM-02"].date.len()), ("Ann Author", "two", 10), "{c:?}");
    let b = a.blame_at("specs/01-a.md", &[1, 2, 9]).unwrap();
    assert!(b[&1].subject == "one" && b[&2].subject == "two" && !b.contains_key(&9), "{b:?}");
    assert!(a.blame_at("/etc/passwd", &[1]).is_err());
    write(&d, "specs/01-a.md", "- **BLM-01** MUST — one, uncommitted.\n- **BLM-02** MUST — two, edited.\n");
    let c = Analysis::load(Some(&d)).unwrap().blame_commits().unwrap();
    assert!(!c.contains_key("BLM-01") && c.contains_key("BLM-02"), "{c:?}");
}

#[doc = "[SCAN-32]"]
#[test]
fn a_requirement_changes_only_when_its_text_differs_from_the_base() {
    use spec17_core::ChangeOpts;
    let d = scratch("realchange");
    write(&d, "spec17.json", r#"{"src":["src"],"tests":["tests"],"ext":["kt"]}"#);
    write(&d, "specs/a.md", "- **REA-01** MUST — one long\n  wrapped bullet.\n- **REA-02** MUST — two.\n- **REA-03** MUST — three.\n");
    write(&d, "src/a.kt", "// (REA-03)\nfun f() {\n    1\n}\n");
    let git = |a: &[&str]| assert!(std::process::Command::new("git").args(["-c", "user.name=t", "-c", "user.email=t@t"]).args(a).current_dir(&d).output().unwrap().status.success(), "{a:?}");
    git(&["init", "-q"]);
    git(&["add", "."]);
    git(&["commit", "-qm", "base"]);
    // REA-01: rewrapped only; REA-02: really edited; REA-03: its cited code re-indented only
    write(&d, "specs/a.md", "- **REA-01** MUST — one long wrapped\n  bullet.\n- **REA-02** MUST — two, edited.\n- **REA-03** MUST — three.\n");
    write(&d, "src/a.kt", "// (REA-03)\nfun f() {\n        1\n}\n");
    let c = Analysis::load(Some(&d)).unwrap().changes(&ChangeOpts::default()).unwrap();
    assert_eq!(c.requirements.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(), ["REA-02"], "{c:?}");
    write(&d, "src/a.kt", "// (REA-03)\nfun f() {\n    2\n}\n");
    let c = Analysis::load(Some(&d)).unwrap().changes(&ChangeOpts::default()).unwrap();
    assert_eq!(c.requirements.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(), ["REA-02", "REA-03"], "{c:?}");
}

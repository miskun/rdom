//! `@import` through a host-provided loader (CSS Cascade 5 §3): the
//! imported sheet's rules sit at the import's position, `layer` /
//! `layer(name)` put them in a layer, `supports()` and media conditions
//! are recorded, a late `@import` is ignored, cycles are cut, and a
//! missing loader or a failed load warns and imports nothing.

use std::cell::RefCell;
use std::collections::HashMap;

use rdom_css::{
    ImportLoader, LoadedSheet, MAX_IMPORT_DEPTH, WarningKind, parse, parse_with_loader,
    parse_with_loader_at,
};

/// A loader over an in-memory map of `url → source`, recording the
/// URLs it was asked for.
struct Files {
    files: HashMap<&'static str, &'static str>,
    asked: RefCell<Vec<String>>,
}

impl Files {
    fn new(files: &[(&'static str, &'static str)]) -> Self {
        Files {
            files: files.iter().copied().collect(),
            asked: RefCell::new(Vec::new()),
        }
    }
}

impl ImportLoader for Files {
    fn load(&self, url: &str) -> Result<String, String> {
        self.asked.borrow_mut().push(url.to_string());
        self.files
            .get(url)
            .map(|s| s.to_string())
            .ok_or_else(|| format!("no such file: {url}"))
    }
}

fn texts(sheet: &rdom_style::Stylesheet) -> Vec<&str> {
    sheet
        .rules()
        .iter()
        .map(|r| r.source_text.as_str())
        .collect()
}

/// Cascade 5 §3: the imported sheet's rules come in at the position of
/// the `@import`, before the importing sheet's own rules; `url()` and a
/// plain string both name the sheet.
#[test]
fn imported_rules_sit_at_the_import() {
    let files = Files::new(&[("a.css", ".a { width: 1 }"), ("b.css", ".b { width: 2 }")]);
    let r = parse_with_loader(
        "@import \"a.css\"; @import url(b.css); .own { width: 3 }",
        &files,
    );
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    assert_eq!(texts(&r.stylesheet), vec![".a", ".b", ".own"]);
    assert_eq!(files.asked.borrow().as_slice(), ["a.css", "b.css"]);
    // Imports nest.
    let files = Files::new(&[
        ("a.css", "@import 'b.css'; .a { width: 1 }"),
        ("b.css", ".b { width: 2 }"),
    ]);
    let r = parse_with_loader("@import 'a.css'; .own { width: 3 }", &files);
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    assert_eq!(texts(&r.stylesheet), vec![".b", ".a", ".own"]);
}

/// Cascade 5 §3: `layer(name)` imports into the named layer (the
/// sheet's own layers nest inside it), bare `layer` into a new
/// anonymous layer.
#[test]
fn layer_conditions_put_the_sheet_in_a_layer() {
    let files = Files::new(&[("a.css", ".a { width: 1 } @layer inner { .i { width: 1 } }")]);
    let r = parse_with_loader(
        "@import 'a.css' layer(base); @import 'a.css' layer;",
        &files,
    );
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    let sheet = &r.stylesheet;
    let rules = sheet.rules();
    assert_eq!(rules.len(), 4);
    let base = rules[0].layer.expect("in base");
    assert_eq!(sheet.layers()[base.index()].name.as_deref(), Some("base"));
    let inner = rules[1].layer.expect("in base.inner");
    assert_eq!(sheet.layers()[inner.index()].parent, Some(base));
    let anon = rules[2].layer.expect("anonymous");
    assert_eq!(sheet.layers()[anon.index()].name, None);
}

/// Cascade 5 §3: `supports()` and a media query list are parsed and
/// recorded on the import (`Stylesheet::imports`); until conditional
/// rules land (C14) they count as true and the sheet is imported.
#[test]
fn supports_and_media_conditions_are_recorded() {
    let files = Files::new(&[("a.css", ".a { width: 1 }")]);
    let r = parse_with_loader(
        "@import url(\"a.css\") layer(x) supports(display: grid) screen and (min-width: 40);",
        &files,
    );
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    let import = &r.stylesheet.imports()[0];
    assert_eq!(import.url, "a.css");
    assert_eq!(import.supports.as_deref(), Some("display: grid"));
    assert_eq!(import.media.as_deref(), Some("screen and (min-width: 40)"));
    assert_eq!(r.stylesheet.rules().len(), 1);
}

/// Cascade 5 §3: `@import` must precede every rule but `@charset` and
/// `@layer` statements; a later one is ignored with a warning.
#[test]
fn a_late_import_is_ignored() {
    let files = Files::new(&[("a.css", ".a { width: 1 }")]);
    let r = parse_with_loader(
        "@charset \"utf-8\"; @layer x, y; @import 'a.css'; .b { width: 1 } @import 'a.css';",
        &files,
    );
    assert_eq!(texts(&r.stylesheet), vec![".a", ".b"]);
    assert_eq!(files.asked.borrow().len(), 1);
    assert!(
        r.warnings
            .iter()
            .any(|w| matches!(&w.kind, WarningKind::ImportIgnored(url) if url == "a.css")),
        "{:?}",
        r.warnings
    );
    let r = parse_with_loader("@layer x { } @import 'a.css';", &files);
    assert!(r.stylesheet.rules().is_empty());
}

/// An import cycle is cut where it closes: each sheet is imported once.
#[test]
fn import_cycles_are_detected() {
    let files = Files::new(&[
        ("a.css", "@import 'b.css'; .a { width: 1 }"),
        ("b.css", "@import 'a.css'; .b { width: 1 }"),
    ]);
    let r = parse_with_loader("@import 'a.css';", &files);
    assert_eq!(texts(&r.stylesheet), vec![".b", ".a"]);
    assert!(
        r.warnings
            .iter()
            .any(|w| matches!(&w.kind, WarningKind::ImportCycle(url) if url == "a.css")),
        "{:?}",
        r.warnings
    );
}

/// No loader, or a loader error: a warning, and no sheet.
#[test]
fn a_failed_import_warns_and_imports_nothing() {
    let r = parse("@import 'a.css'; .own { width: 1 }");
    assert_eq!(texts(&r.stylesheet), vec![".own"]);
    assert!(
        matches!(&r.warnings[..], [w] if matches!(&w.kind, WarningKind::ImportFailed { url, .. } if url == "a.css"))
    );
    let files = Files::new(&[]);
    let r = parse_with_loader("@import 'missing.css'; .own { width: 1 }", &files);
    assert_eq!(texts(&r.stylesheet), vec![".own"]);
    assert!(
        matches!(&r.warnings[..], [w] if matches!(&w.kind, WarningKind::ImportFailed { reason, .. } if reason.contains("no such file")))
    );
}

/// An `@import` without a URL is an invalid prelude.
#[test]
fn an_import_without_a_url_is_invalid() {
    let r = parse("@import layer(x); .own { width: 1 }");
    assert_eq!(texts(&r.stylesheet), vec![".own"]);
    assert!(
        matches!(&r.warnings[0].kind, WarningKind::InvalidAtRulePrelude { name, .. } if name == "import")
    );
}

// ── C1G-IMPORT-EDGES ────────────────────────────────────────────────

/// A loader over files in directories: it resolves a URL against the
/// importing sheet's URL (`base`) — a path join with `.` / `..`
/// normalised — and returns the resolved path as the sheet's URL, as a
/// host serving an asset tree would.
struct Tree {
    files: HashMap<&'static str, &'static str>,
    asked: RefCell<Vec<(String, Option<String>)>>,
}

impl Tree {
    fn new(files: &[(&'static str, &'static str)]) -> Self {
        Tree {
            files: files.iter().copied().collect(),
            asked: RefCell::new(Vec::new()),
        }
    }
}

fn resolve(url: &str, base: Option<&str>) -> String {
    let dir = base
        .and_then(|b| b.rfind('/').map(|i| &b[..=i]))
        .unwrap_or("");
    let joined = format!("{dir}{url}");
    let mut parts: Vec<&str> = Vec::new();
    for seg in joined.split('/') {
        match seg {
            "." | "" => {}
            ".." => {
                parts.pop();
            }
            s => parts.push(s),
        }
    }
    parts.join("/")
}

impl ImportLoader for Tree {
    fn load(&self, url: &str) -> Result<String, String> {
        self.load_from(url, None).map(|sheet| sheet.text)
    }

    fn load_from(&self, url: &str, base: Option<&str>) -> Result<LoadedSheet, String> {
        self.asked
            .borrow_mut()
            .push((url.to_string(), base.map(str::to_string)));
        let resolved = resolve(url, base);
        let text = self
            .files
            .get(resolved.as_str())
            .ok_or(format!("no {resolved}"))?;
        Ok(LoadedSheet::new(resolved, text.to_string()))
    }
}

fn has_cycle_warning(r: &rdom_css::ParseResult, url: &str) -> bool {
    r.warnings
        .iter()
        .any(|w| matches!(&w.kind, WarningKind::ImportCycle(u) if u == url))
}

/// CSS Cascade 5 §3 / CSSOM: an `@import` URL is relative to the sheet
/// that holds it, so the loader is given the importing sheet's resolved
/// URL as the base — the root's own URL for its imports.
#[test]
fn nested_relative_imports_resolve_against_the_importing_sheet() {
    let tree = Tree::new(&[
        ("css/parts/a.css", "@import 'b.css'; .a { width: 1 }"),
        ("css/parts/b.css", ".b { width: 1 }"),
    ]);
    let r = parse_with_loader_at(
        "@import 'parts/a.css'; .root { width: 1 }",
        "css/root.css",
        &tree,
    );
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    assert_eq!(texts(&r.stylesheet), vec![".b", ".a", ".root"]);
    assert_eq!(
        tree.asked.borrow().as_slice(),
        [
            ("parts/a.css".to_string(), Some("css/root.css".to_string())),
            ("b.css".to_string(), Some("css/parts/a.css".to_string())),
        ]
    );
}

/// The root sheet is on the cycle stack: `a → b → a` imports `a` once —
/// as the root — not a second time through `b`.
#[test]
fn the_root_sheet_is_on_the_cycle_stack() {
    let tree = Tree::new(&[
        ("a.css", "@import 'b.css'; .a { width: 1 }"),
        ("b.css", "@import 'a.css'; .b { width: 1 }"),
    ]);
    let r = parse_with_loader_at("@import 'b.css'; .a { width: 1 }", "a.css", &tree);
    assert_eq!(texts(&r.stylesheet), vec![".b", ".a"]);
    assert!(has_cycle_warning(&r, "a.css"), "{:?}", r.warnings);
}

/// A cycle is the same sheet by the URL the loader resolved, however
/// the `@import` spells it.
#[test]
fn cycles_compare_resolved_urls() {
    let tree = Tree::new(&[
        ("css/a.css", "@import './b.css'; .a { width: 1 }"),
        ("css/b.css", "@import '../css/a.css'; .b { width: 1 }"),
    ]);
    let r = parse_with_loader_at("@import 'css/a.css';", "index.css", &tree);
    assert_eq!(texts(&r.stylesheet), vec![".b", ".a"]);
    assert!(has_cycle_warning(&r, "../css/a.css"), "{:?}", r.warnings);
}

/// Imports nest at most `MAX_IMPORT_DEPTH` deep; a deeper one warns
/// (`ImportTooDeep`) and imports nothing, so an endless chain of
/// distinct URLs terminates.
#[test]
fn import_depth_is_capped() {
    struct Endless;
    impl ImportLoader for Endless {
        fn load(&self, url: &str) -> Result<String, String> {
            let n: usize = url[1..].parse().unwrap();
            Ok(format!("@import 'n{}'; .n{n} {{ width: 1 }}", n + 1))
        }
    }
    let r = parse_with_loader("@import 'n1';", &Endless);
    assert_eq!(r.stylesheet.rules().len(), MAX_IMPORT_DEPTH);
    let deep = format!("n{}", MAX_IMPORT_DEPTH + 1);
    assert!(
        matches!(&r.warnings[..], [w] if matches!(&w.kind, WarningKind::ImportTooDeep(u) if *u == deep)),
        "{:?}",
        r.warnings
    );
}

/// A plain closure is still a loader; the URL as written is its URL.
#[test]
fn a_closure_is_a_loader() {
    let loader = |url: &str| match url {
        "a.css" => Ok("@import 'a.css'; .a { width: 1 }".to_string()),
        other => Err(format!("no {other}")),
    };
    let r = parse_with_loader_at("@import 'a.css';", "root.css", &loader);
    assert_eq!(texts(&r.stylesheet), vec![".a"]);
    assert!(has_cycle_warning(&r, "a.css"), "{:?}", r.warnings);
}

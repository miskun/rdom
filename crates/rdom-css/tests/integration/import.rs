//! `@import` through a host-provided loader (CSS Cascade 5 §3): the
//! imported sheet's rules sit at the import's position, `layer` /
//! `layer(name)` put them in a layer, `supports()` and media conditions
//! are recorded, a late `@import` is ignored, cycles are cut, and a
//! missing loader or a failed load warns and imports nothing.

use std::cell::RefCell;
use std::collections::HashMap;

use rdom_css::{ImportLoader, WarningKind, parse, parse_with_loader};

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

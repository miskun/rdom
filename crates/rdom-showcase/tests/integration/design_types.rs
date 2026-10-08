//! C11G-DESIGN-TYPES — every public enum and struct a published crate
//! re-exports from its `lib.rs` is classified in `specs/DESIGN.md`
//! ("Which public types are `#[non_exhaustive]`"), checked mechanically:
//! the rule was missed for four phases running by review alone.
//!
//! It lives here because `rdom-showcase` is the one crate never
//! published: the check reads `specs/` and every sibling crate's `src/`,
//! which a published tarball would not have.
//!
//! The rule CLAUDE.md records:
//! - the **surface** is each published crate's `lib.rs`: every name its
//!   `pub use` items bring in (the alias of an `as`; a whole-crate alias
//!   such as `rdom_core as core_api` is not a type), and every
//!   `pub struct` / `pub enum` written in `lib.rs` itself;
//! - a name is **a type** when a `pub struct` or `pub enum` of that name
//!   is defined in a published crate's production sources (traits, type
//!   aliases, functions and constants are not);
//! - it is **classified** when it appears inside backticks between
//!   DESIGN.md's `<!-- type-classification: begin -->` and
//!   `<!-- type-classification: end -->` markers — the section's
//!   bullets, each of which names a kind and its reason.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// The published crates (`rdom-showcase` is not).
const PUBLISHED: &[&str] = &[
    "rdom-core",
    "rdom-style",
    "rdom-css",
    "rdom-parser",
    "rdom-tui",
];

const BEGIN: &str = "<!-- type-classification: begin -->";
const END: &str = "<!-- type-classification: end -->";

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// The identifiers in `text`.
fn idents(text: &str) -> impl Iterator<Item = &str> {
    text.split(|c: char| !is_ident_char(c))
        .filter(|w| w.starts_with(|c: char| c.is_ascii_alphabetic()))
}

/// The names `source` (a `lib.rs`) exports: what each `pub use` item
/// brings in, and the structs and enums it defines itself.
fn surface(source: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut rest = source;
    while let Some(at) = rest.find("\npub use ") {
        let item = &rest[at + "\npub use ".len()..];
        let end = item.find(';').expect("a `pub use` item ends with `;`");
        let body = &item[..end];
        rest = &item[end..];
        let words: Vec<&str> = idents(body).collect();
        if words.len() == 3 && words[1] == "as" && !body.contains("::") {
            continue; // `pub use rdom_core as core_api;` — a crate alias.
        }
        // An item's exported names are the last segment of each path:
        // an identifier not followed by `::` (and the alias after `as`).
        let mut i = 0;
        let bytes = body.as_bytes();
        while i < bytes.len() {
            if !is_ident_char(bytes[i] as char) {
                i += 1;
                continue;
            }
            let start = i;
            while i < bytes.len() && is_ident_char(bytes[i] as char) {
                i += 1;
            }
            let word = &body[start..i];
            let after = body[i..].trim_start();
            let followed_by_path = after.starts_with("::");
            let followed_by_as = after.starts_with("as ");
            if !followed_by_path && !followed_by_as && word != "as" && word != "self" {
                names.insert(word.to_string());
            }
        }
    }
    names.extend(defined_types(source));
    names
}

/// The `pub struct` / `pub enum` names defined in `source`.
fn defined_types(source: &str) -> impl Iterator<Item = String> + '_ {
    source.lines().filter_map(|line| {
        let line = line.trim_start();
        let rest = line
            .strip_prefix("pub struct ")
            .or_else(|| line.strip_prefix("pub enum "))?;
        idents(rest).next().map(str::to_string)
    })
}

/// Every production `.rs` file under `dir` (test files by name excluded,
/// as `file_sizes.rs` counts them).
fn production_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("readable source dir") {
        let path = entry.expect("dir entry").path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if path.is_dir() {
            production_files(&path, out);
        } else if name.ends_with(".rs")
            && name != "tests.rs"
            && !name.ends_with("_tests.rs")
            && !name.starts_with("test_")
        {
            out.push(path);
        }
    }
}

/// The identifiers inside backticks between the markers of `design`.
fn classified(design: &str) -> BTreeSet<String> {
    let start = design.find(BEGIN).expect("DESIGN.md has the begin marker");
    let end = design.find(END).expect("DESIGN.md has the end marker");
    assert!(start < end, "the markers are in order");
    design[start..end]
        .split('`')
        .skip(1)
        .step_by(2)
        .flat_map(idents)
        .map(str::to_string)
        .collect()
}

/// The rule's own cases.
#[test]
fn the_surface_reads_pub_use_items_and_local_types() {
    let lib = "//! doc\npub mod m;\npub use a::{B, c::D as E, f};\npub use rdom_core as core_api;\n\
               pub use g::H;\npub struct Local;\npub(crate) struct Hidden;\n";
    let names: Vec<String> = surface(lib).into_iter().collect();
    assert_eq!(names, ["B", "E", "H", "Local", "f"]);
    let design = format!("`Out` {BEGIN} a `Foo<Bar>` and `Baz::Qux` {END} `After`");
    let got: Vec<String> = classified(&design).into_iter().collect();
    assert_eq!(got, ["Bar", "Baz", "Foo", "Qux"]);
}

#[test]
fn every_reexported_type_is_classified_in_design() {
    let root = workspace();
    let mut exported = BTreeSet::new();
    let mut types = BTreeSet::new();
    for krate in PUBLISHED {
        let src = root.join("crates").join(krate).join("src");
        let lib = std::fs::read_to_string(src.join("lib.rs")).expect("lib.rs");
        exported.extend(surface(&lib));
        let mut files = Vec::new();
        production_files(&src, &mut files);
        for file in files {
            let text = std::fs::read_to_string(&file).expect("source file");
            types.extend(defined_types(&text));
        }
    }
    let design = std::fs::read_to_string(root.join("specs/DESIGN.md")).expect("DESIGN.md");
    let classified = classified(&design);
    let checked: Vec<&String> = exported.iter().filter(|n| types.contains(*n)).collect();
    assert!(
        checked.len() > 200,
        "found the re-exported types ({})",
        checked.len()
    );
    let missing: Vec<&&String> = checked
        .iter()
        .filter(|n| !classified.contains(n.as_str()))
        .collect();
    assert!(
        missing.is_empty(),
        "public types with no classification in specs/DESIGN.md §\"Which public types are \
         `#[non_exhaustive]`\" — name each under its kind, with the reason: {missing:#?}"
    );
}

/// Directionality, `AttrCase` and `NthKind` are closed by their specs
/// (HTML §3.2.6.4: `ltr` / `rtl`; Selectors 4 §6.3: no flag, `i`, `s`;
/// Selectors 4 §13.3–§13.4: the four child-indexed forms), so a consumer
/// matches them without a wildcard arm.
#[test]
fn closed_by_spec_enums_match_exhaustively() {
    use rdom_tui::core_api::Directionality;
    use rdom_tui::core_api::selectors::{AttrCase, NthKind};
    let dir = |d: Directionality| match d {
        Directionality::Ltr => "ltr",
        Directionality::Rtl => "rtl",
    };
    let case = |c: AttrCase| match c {
        AttrCase::Default => "",
        AttrCase::AsciiInsensitive => "i",
        AttrCase::Sensitive => "s",
    };
    let kind = |k: NthKind| match k {
        NthKind::Child => "nth-child",
        NthKind::LastChild => "nth-last-child",
        NthKind::OfType => "nth-of-type",
        NthKind::LastOfType => "nth-last-of-type",
    };
    assert_eq!(dir(Directionality::Rtl), "rtl");
    assert_eq!(case(AttrCase::AsciiInsensitive), "i");
    assert_eq!(kind(NthKind::OfType), "nth-of-type");
}

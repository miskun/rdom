//! C11G-DESIGN-TYPES — every public enum and struct a published crate
//! re-exports from its `lib.rs` is classified in `specs/DESIGN.md`
//! ("Which public types are `#[non_exhaustive]`"), checked mechanically:
//! the rule was missed for four phases running by review alone.
//!
//! It lives here because `rdom-showcase` is the one crate never
//! published: the check reads `specs/` and every sibling crate's `src/`,
//! which a published tarball would not have.
//!
//! The rule CLAUDE.md records (hardened by C12G-DESIGN-TEST):
//! - the **surface** is each published crate's `lib.rs`: every name its
//!   `pub use` items bring in (the alias of an `as`; a whole-crate alias
//!   such as `rdom_core as core_api` is not a type), every name a glob
//!   `pub use path::*` brings in (the module's own surface, followed),
//!   every `pub struct` / `pub enum` written in `lib.rs` itself, and the
//!   surface of each `pub mod` it declares, followed down every `pub mod`
//!   path (`rdom_core::table::CellSpan` is public; C13G-MISC);
//! - a name is **a type** when a `pub struct` or `pub enum` of that name
//!   is defined in a published crate's production sources (traits, type
//!   aliases, functions and constants are not) — written out, or by an
//!   invocation of a type-defining macro, which [`TYPE_MACROS`] lists
//!   (a new `macro_rules!` that defines a type fails the check until it
//!   is listed, so no macro hides a type from it);
//! - it is **classified** when a kind's bullet between DESIGN.md's
//!   `<!-- type-classification: begin -->` / `end` markers lists it as an
//!   **entry**: a backtick span that is exactly its name (`` `Name` `` or
//!   `` `Name<T>` ``) — a path such as `` `Name::Variant` `` or a call is
//!   a mention, not an entry;
//! - the kind fixes the attribute: a type listed under an open kind (open
//!   web vocabularies, error and outcome sets, options bags, the one open
//!   CSS value) is `#[non_exhaustive]`, one under a closed kind (CSS value
//!   and style-record types, sets fixed by their definition) is not, and
//!   a type may not be listed under both. The handles, sealed and
//!   crate-private bullets are outside the attribute rule, as DESIGN says.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// The published crates (`rdom-showcase` is not).
const PUBLISHED: &[&str] = &[
    "rdom-core",
    "rdom-style",
    "rdom-css",
    "rdom-parser",
    "rdom-tui",
];

/// The `macro_rules!` that define a public type from their input — each
/// invocation spells `pub struct Name` / `pub enum Name`, which the
/// source scan reads.
const TYPE_MACROS: &[&str] = &["bitflags_like"];

const BEGIN: &str = "<!-- type-classification: begin -->";
const END: &str = "<!-- type-classification: end -->";

/// A kind's polarity under the `#[non_exhaustive]` rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Kind {
    Open,
    Closed,
    /// Handles, sealed and crate-private records: outside the rule.
    Exempt,
}

/// Each kind's bullet, by the bold text it starts with.
const KINDS: &[(&str, Kind)] = &[
    ("Open web vocabularies", Kind::Open),
    ("Error, warning and outcome sets", Kind::Open),
    ("Options bags", Kind::Open),
    ("The one open CSS value is", Kind::Open),
    ("CSS value and style-record types", Kind::Closed),
    ("Sets fixed by their definition", Kind::Closed),
    ("Handles and runtime internals", Kind::Exempt),
    ("Sealed by private fields", Kind::Exempt),
    ("Crate-private records", Kind::Exempt),
];

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

/// The names `source` (a `lib.rs`, or a module a glob names) exports:
/// what each `pub use` item brings in — a glob's through `module`, which
/// gives the source of the module a path names — and the structs and
/// enums it defines itself.
fn surface(source: &str, here: &str, module: &dyn Fn(&str) -> Option<String>) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    // `pub mod x;`: a public path to `x`'s own surface (C13G-MISC), from
    // the module it is declared in.
    let mut rest = source;
    while let Some(at) = rest.find("\npub mod ") {
        let item = &rest[at + "\npub mod ".len()..];
        rest = item;
        let Some(name) = idents(item).next() else {
            continue;
        };
        if !item[name.len()..].trim_start().starts_with(';') {
            continue; // An inline `pub mod x { … }`: scanned with `source`.
        }
        let path = if here.is_empty() {
            name.to_string()
        } else {
            format!("{here}::{name}")
        };
        let source = module(&path)
            .unwrap_or_else(|| panic!("`pub mod {name};` in `{here}` names a module file"));
        names.extend(surface(&source, &path, module));
    }
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
        if let Some(path) = body.trim().strip_suffix("::*") {
            let source = module(path.trim())
                .unwrap_or_else(|| panic!("the glob `pub use {body}` names a module file"));
            names.extend(surface(&source, "", module));
            continue;
        }
        assert!(
            !body.contains('*'),
            "a glob inside a `pub use` group is not followed: `pub use {body}` — \
             give it a `pub use` item of its own"
        );
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
    names.extend(defined_types(source).into_iter().map(|(name, _)| name));
    names
}

/// The `pub struct` / `pub enum` names defined in `source`, each with
/// whether its attributes carry `#[non_exhaustive]` (the attribute and
/// doc lines right above it).
fn defined_types(source: &str) -> Vec<(String, bool)> {
    let lines: Vec<&str> = source.lines().collect();
    let mut out = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let line = line.trim_start();
        let Some(rest) = line
            .strip_prefix("pub struct ")
            .or_else(|| line.strip_prefix("pub enum "))
        else {
            continue;
        };
        let Some(name) = idents(rest).next() else {
            continue;
        };
        let non_exhaustive = lines[..i]
            .iter()
            .rev()
            .map(|l| l.trim())
            .take_while(|l| l.starts_with("#[") || l.starts_with("///") || l.starts_with("$("))
            .any(|l| l.starts_with("#[non_exhaustive]"));
        out.push((name.to_string(), non_exhaustive));
    }
    out
}

/// The `macro_rules!` in `source` whose body defines a struct or enum
/// from its input (`struct $name`, `enum $name`).
fn type_macros(source: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = source;
    while let Some(at) = rest.find("macro_rules!") {
        let after = &rest[at + "macro_rules!".len()..];
        let name = idents(after).next().unwrap_or("").to_string();
        let open = after.find('{').unwrap_or(after.len());
        let mut depth = 0usize;
        let mut end = after.len();
        for (i, c) in after[open..].char_indices() {
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = open + i;
                        break;
                    }
                }
                _ => {}
            }
        }
        let body = &after[open..end];
        if body.contains("struct $") || body.contains("enum $") {
            out.push(name);
        }
        rest = &after[end..];
    }
    out
}

/// The entries of each kind's bullet between the markers of `design`:
/// every backtick span that is exactly a name (`Name`, `Name<T>`), with
/// the kinds of the bullets that list it.
fn classified(design: &str) -> BTreeMap<String, BTreeSet<Kind>> {
    let start = design.find(BEGIN).expect("DESIGN.md has the begin marker");
    let end = design.find(END).expect("DESIGN.md has the end marker");
    assert!(start < end, "the markers are in order");
    let mut entries: BTreeMap<String, BTreeSet<Kind>> = BTreeMap::new();
    for block in design[start..end].lines() {
        let lead = block.trim_start_matches("- ").trim_start_matches("**");
        let Some(&(_, kind)) = KINDS.iter().find(|(name, _)| lead.starts_with(name)) else {
            continue;
        };
        for span in block.split('`').skip(1).step_by(2) {
            let name = span.split('<').next().unwrap_or("");
            let generic_ok = span.len() == name.len() || span.ends_with('>');
            if generic_ok
                && name.starts_with(|c: char| c.is_ascii_alphabetic())
                && name.chars().all(is_ident_char)
            {
                entries.entry(name.to_string()).or_default().insert(kind);
            }
        }
    }
    entries
}

/// The source of the module `path` names, from `krate`'s `lib.rs`: a
/// path starting with a published crate's name (`rdom_style::calc`) is
/// in that crate, any other (`crate::`, `self::`, a local module) in
/// `krate`; `a::b` is `src/a/b.rs` or `src/a/b/mod.rs`.
fn module_source(root: &Path, krate: &str, path: &str) -> Option<String> {
    let mut segments: Vec<&str> = path.split("::").map(str::trim).collect();
    let mut krate = krate.to_string();
    if let Some(first) = segments.first() {
        let as_crate = first.replace('_', "-");
        if PUBLISHED.contains(&as_crate.as_str()) {
            krate = as_crate;
            segments.remove(0);
        } else if *first == "crate" || *first == "self" {
            segments.remove(0);
        }
    }
    let src = root.join("crates").join(&krate).join("src");
    let rel = segments.join("/");
    std::fs::read_to_string(src.join(format!("{rel}.rs")))
        .or_else(|_| std::fs::read_to_string(src.join(&rel).join("mod.rs")))
        .ok()
}

/// C13G-MISC (Phase 13 architect N10): a type reachable through a
/// `pub mod` path — `rdom_core::table::CellSpan` — is as public as a
/// re-exported one: the surface follows each `pub mod` (from the module
/// it is declared in) and takes that module's own surface.
#[test]
fn the_surface_follows_pub_mod_paths() {
    let lib = "//! doc\npub mod m;\nmod private;\npub(crate) mod hidden;\n";
    let module = |path: &str| match path {
        "m" => Some("\npub mod n;\npub struct InMod;\npub use x::Used;\n".to_string()),
        "m::n" => Some("\npub enum Deeper {}\n".to_string()),
        _ => None,
    };
    let names: Vec<String> = surface(lib, "", &module).into_iter().collect();
    assert_eq!(names, ["Deeper", "InMod", "Used"]);
}

/// The rule's own cases.
#[test]
fn the_surface_reads_pub_use_items_globs_and_local_types() {
    let lib = "//! doc\npub mod m;\npub use a::{B, c::D as E, f};\npub use rdom_core as core_api;\n\
               pub use g::H;\npub use m::*;\npub struct Local;\npub(crate) struct Hidden;\n";
    let module =
        |path: &str| (path == "m").then(|| "\npub use x::Deep;\npub enum Inner {}\n".into());
    let names: Vec<String> = surface(lib, "", &module).into_iter().collect();
    assert_eq!(names, ["B", "Deep", "E", "H", "Inner", "Local", "f"]);
}

#[test]
fn defined_types_read_the_non_exhaustive_attribute() {
    let src = "/// Doc.\n#[derive(Debug)]\n#[non_exhaustive]\npub enum Open { A }\n\n\
               #[derive(Debug)]\npub struct Closed;\n";
    assert_eq!(
        defined_types(src),
        [("Open".to_string(), true), ("Closed".to_string(), false)]
    );
    let macros = "macro_rules! makes { ($v:vis struct $n:ident) => { $v struct $n; }; }\n\
                  macro_rules! plain { () => { fn f() {} }; }\n";
    assert_eq!(type_macros(macros), ["makes"]);
}

#[test]
fn entries_are_exact_names_under_a_kind() {
    let design = format!(
        "`Out`\n{BEGIN}\n- **Open web vocabularies**: `Foo<T>` (unlike `Bar::X`), `Baz`.\n\
         - **Sets fixed by their definition**: `Baz`, `Qux::new(..)`.\nOther `Free`.\n{END}"
    );
    let got = classified(&design);
    let names: Vec<&String> = got.keys().collect();
    assert_eq!(names, ["Baz", "Foo"]);
    assert_eq!(got["Baz"], BTreeSet::from([Kind::Open, Kind::Closed]));
}

#[test]
fn every_reexported_type_is_classified_in_design() {
    let root = workspace();
    let mut exported = BTreeSet::new();
    let mut types: BTreeMap<String, Vec<bool>> = BTreeMap::new();
    let mut macros = BTreeSet::new();
    for krate in PUBLISHED {
        let src = root.join("crates").join(krate).join("src");
        let lib = std::fs::read_to_string(src.join("lib.rs")).expect("lib.rs");
        exported.extend(surface(&lib, "", &|path| module_source(&root, krate, path)));
        let mut files = Vec::new();
        production_files(&src, &mut files);
        for file in files {
            let text = std::fs::read_to_string(&file).expect("source file");
            for (name, non_exhaustive) in defined_types(&text) {
                types.entry(name).or_default().push(non_exhaustive);
            }
            macros.extend(type_macros(&text));
        }
    }
    let listed: BTreeSet<String> = TYPE_MACROS.iter().map(|m| m.to_string()).collect();
    assert_eq!(
        macros, listed,
        "a `macro_rules!` defines a public type: list it in `TYPE_MACROS` once its \
         invocations spell `pub struct` / `pub enum`, so the scan reads the types it makes"
    );
    let design = std::fs::read_to_string(root.join("specs/DESIGN.md")).expect("DESIGN.md");
    let classified = classified(&design);
    let checked: Vec<&String> = exported.iter().filter(|n| types.contains_key(*n)).collect();
    assert!(
        checked.len() > 200,
        "found the re-exported types ({})",
        checked.len()
    );
    let missing: Vec<&&String> = checked
        .iter()
        .filter(|n| !classified.contains_key(n.as_str()))
        .collect();
    assert!(
        missing.is_empty(),
        "public types with no entry in specs/DESIGN.md §\"Which public types are \
         `#[non_exhaustive]`\" — list each, as `Name`, under its kind with the reason: \
         {missing:#?}"
    );
    let mut wrong = Vec::new();
    for name in checked {
        let kinds: BTreeSet<Kind> = classified[name.as_str()]
            .iter()
            .copied()
            .filter(|k| *k != Kind::Exempt)
            .collect();
        let attrs = &types[name.as_str()];
        let open = attrs.iter().all(|&a| a);
        let closed = attrs.iter().all(|&a| !a);
        let ok = match kinds.iter().collect::<Vec<_>>().as_slice() {
            [] => true,
            [Kind::Open] => open,
            [Kind::Closed] => closed,
            _ => false,
        };
        if !ok {
            wrong.push(format!(
                "{name}: listed {kinds:?}, #[non_exhaustive] {attrs:?}"
            ));
        }
    }
    assert!(
        wrong.is_empty(),
        "a type's DESIGN kind and its `#[non_exhaustive]` disagree (open ⇔ the attribute; \
         one kind per type): {wrong:#?}"
    );
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

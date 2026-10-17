//! C7G-SIZES — the workspace's production files stay under 600 lines
//! (CLAUDE.md §Architecture Hygiene: "files growing past a few hundred
//! lines because multiple responsibilities are accumulating"), checked
//! mechanically so drift fails the gate instead of waiting for a review.
//!
//! It lives here because `rdom-showcase` is the one crate never
//! published: the check reads every sibling crate's `src/`, which a
//! published tarball would not have.
//!
//! What counts as production, the rule CLAUDE.md records:
//! - every `.rs` file under `crates/*/src/`;
//! - except test files — `tests.rs`, `*_tests.rs`, `test_*.rs`;
//! - and except the lines of a `#[cfg(test)]` module written inline
//!   (`#[cfg(test)]` then `mod name {` at the start of a line, through
//!   its closing `}` at the start of a line);
//! - `rdom-parser/src/entities.rs`, the generated WHATWG named
//!   character reference table, is data and exempt.

use std::path::{Path, PathBuf};

/// The most lines a production file may have.
const LIMIT: usize = 600;

/// The bar: a production file past it is listed in TECH_DEBT `SIZE-1`.
const BAR: usize = 500;

/// Generated data tables, exempt from the limit (paths under `crates/`).
const EXEMPT: &[&str] = &["rdom-parser/src/entities.rs"];

/// Every `.rs` file under `dir`, recursively.
fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("readable source dir") {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// Whether `name` is a test file by name.
fn is_test_file(name: &str) -> bool {
    name == "tests.rs" || name.ends_with("_tests.rs") || name.starts_with("test_")
}

/// The lines of `source` outside its inline `#[cfg(test)]` modules.
fn production_lines(source: &str) -> usize {
    let lines: Vec<&str> = source.lines().collect();
    let opens_module = |l: &str| {
        let l = l.strip_prefix("pub ").unwrap_or(l);
        let l = l
            .strip_prefix("pub(crate) ")
            .or_else(|| l.strip_prefix("pub(super) "))
            .unwrap_or(l);
        l.starts_with("mod ") && l.ends_with('{')
    };
    let (mut count, mut i) = (0, 0);
    while i < lines.len() {
        if lines[i] == "#[cfg(test)]" && lines.get(i + 1).is_some_and(|l| opens_module(l)) {
            i += 2;
            while i < lines.len() && lines[i] != "}" {
                i += 1;
            }
            i += 1;
            continue;
        }
        count += 1;
        i += 1;
    }
    count
}

/// The rule's own cases: an inline test module is not counted, the code
/// after it is.
#[test]
fn production_lines_skip_inline_test_modules() {
    let src = "fn a() {}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn t() {}\n}\nfn b() {}\n";
    assert_eq!(production_lines(src), 3);
    let src = "#[cfg(test)]\npub(super) mod probe {\n}\nfn c() {}\n";
    assert_eq!(production_lines(src), 1);
    assert!(is_test_file("apply_tests.rs") && is_test_file("tests.rs"));
    assert!(!is_test_file("walk.rs"));
}

/// Every production file (path under `crates/`) with its production
/// line count.
fn production_files() -> Vec<(String, usize)> {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut files = Vec::new();
    for krate in std::fs::read_dir(&crates).expect("the crates directory") {
        let src = krate.expect("dir entry").path().join("src");
        if src.is_dir() {
            rust_files(&src, &mut files);
        }
    }
    assert!(files.len() > 100, "found the workspace's sources");
    let mut out: Vec<(String, usize)> = files
        .iter()
        .filter_map(|path| {
            let rel = path
                .strip_prefix(&crates)
                .ok()?
                .to_string_lossy()
                .replace('\\', "/");
            let name = path.file_name()?.to_str()?;
            if is_test_file(name) || EXEMPT.contains(&rel.as_str()) {
                return None;
            }
            let n = production_lines(&std::fs::read_to_string(path).ok()?);
            Some((rel, n))
        })
        .collect();
    out.sort();
    out
}

#[test]
fn no_production_file_passes_the_limit() {
    let over: Vec<String> = production_files()
        .into_iter()
        .filter(|(_, n)| *n > LIMIT)
        .map(|(rel, n)| format!("{rel}: {n}"))
        .collect();
    assert!(
        over.is_empty(),
        "production files past {LIMIT} lines — split them by concern (TECH_DEBT SIZE-1): {over:#?}"
    );
}

/// The files TECH_DEBT `SIZE-1`'s table names, as paths under `crates/`
/// (a path without a crate is rdom-tui's `src/`).
fn size_one_listed(tech_debt: &str) -> Vec<String> {
    let section = tech_debt
        .split("- **`SIZE-1`")
        .nth(1)
        .expect("TECH_DEBT has SIZE-1")
        .split("\n- **")
        .next()
        .unwrap();
    let mut listed: Vec<String> = section
        .lines()
        .filter_map(|l| {
            let path = l.trim().strip_prefix("| `")?.split('`').next()?;
            Some(if path.starts_with("rdom-") {
                path.to_string()
            } else {
                format!("rdom-tui/src/{path}")
            })
        })
        .collect();
    listed.sort();
    listed
}

/// CLAUDE.md §Architecture Hygiene: a production file past the 500-line
/// bar is listed in TECH_DEBT `SIZE-1`, and the table lists no file that
/// is back under it — checked here so the list cannot drift (C16G-HYGIENE;
/// Phase 16 gate architect N9). The counts in the table are as of its
/// last recount.
#[test]
fn size_one_lists_every_file_past_the_bar() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let tech_debt =
        std::fs::read_to_string(root.join("specs/TECH_DEBT.md")).expect("TECH_DEBT.md readable");
    let listed = size_one_listed(&tech_debt);
    let past: Vec<String> = production_files()
        .into_iter()
        .filter(|(_, n)| *n > BAR)
        .map(|(rel, _)| rel)
        .collect();
    let missing: Vec<&String> = past.iter().filter(|p| !listed.contains(p)).collect();
    let stale: Vec<&String> = listed.iter().filter(|p| !past.contains(p)).collect();
    assert!(
        missing.is_empty() && stale.is_empty(),
        "TECH_DEBT SIZE-1 out of date — past {BAR} lines but not listed: {missing:#?}; \
         listed but not past it: {stale:#?}"
    );
}

/// Whether the Markdown file `name` is pulled in by `include_str!` from
/// one of `sources`, the Rust files beside it — a doctest page.
fn is_included_markdown(name: &str, sources: &[String]) -> bool {
    let needle = format!("include_str!(\"{name}\")");
    sources.iter().any(|s| s.contains(&needle))
}

/// Every file under `dir` that is neither Rust nor Markdown a Rust file
/// beside it includes (`include_str!`, a doctest page). Anything else —
/// a mutation-run backup like `content.rs.m7`, an editor swap file, a
/// stray note — would ship in the crate tarball (C15G-HYGIENE,
/// C16G-HYGIENE).
fn stray_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .expect("readable source dir")
        .map(|e| e.expect("dir entry").path())
        .collect();
    let sources: Vec<String> = entries
        .iter()
        .filter(|p| p.extension().is_some_and(|e| e == "rs"))
        .filter_map(|p| std::fs::read_to_string(p).ok())
        .collect();
    for path in entries {
        if path.is_dir() {
            stray_files(&path, out);
            continue;
        }
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let source = match path.extension().and_then(|e| e.to_str()) {
            Some("rs") => true,
            Some("md") => is_included_markdown(name, &sources),
            _ => false,
        };
        if !source {
            out.push(path);
        }
    }
}

/// The rule's own cases: Markdown is a source only where a Rust file
/// beside it includes it.
#[test]
fn markdown_under_src_must_be_included() {
    let sources = vec!["#[doc = include_str!(\"doctests.md\")]".to_string()];
    assert!(is_included_markdown("doctests.md", &sources));
    assert!(!is_included_markdown("NOTES.md", &sources));
}

#[test]
fn no_stray_files_under_src() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut stray = Vec::new();
    for krate in std::fs::read_dir(&crates).expect("the crates directory") {
        let src = krate.expect("dir entry").path().join("src");
        if src.is_dir() {
            stray_files(&src, &mut stray);
        }
    }
    let mut stray: Vec<String> = stray
        .iter()
        .filter_map(|p| {
            Some(
                p.strip_prefix(&crates)
                    .ok()?
                    .to_string_lossy()
                    .replace('\\', "/"),
            )
        })
        .collect();
    stray.sort();
    assert!(
        stray.is_empty(),
        "files under crates/*/src that are neither Rust nor Markdown a Rust file beside them \
         includes would ship in the crate tarball — delete them: {stray:#?}"
    );
}

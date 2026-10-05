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

#[test]
fn no_production_file_passes_the_limit() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut files = Vec::new();
    for krate in std::fs::read_dir(&crates).expect("the crates directory") {
        let src = krate.expect("dir entry").path().join("src");
        if src.is_dir() {
            rust_files(&src, &mut files);
        }
    }
    assert!(files.len() > 100, "found the workspace's sources");
    let mut over: Vec<String> = files
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
            (n > LIMIT).then(|| format!("{rel}: {n}"))
        })
        .collect();
    over.sort();
    assert!(
        over.is_empty(),
        "production files past {LIMIT} lines — split them by concern (TECH_DEBT SIZE-1): {over:#?}"
    );
}

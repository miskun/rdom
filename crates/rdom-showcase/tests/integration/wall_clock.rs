//! CAPTURE phase 1 (`specs/CAPTURE.md` §Deterministic environment): the
//! scheduler's clock is the only clock a frame reads, so a capture session
//! that drives it (`App::advance`) is deterministic. Production code reads
//! the wall clock (`Instant::now()`, `SystemTime::now()`) only where this
//! list says, each for its reason; a new read fails here until it is
//! listed with one — or, better, reads the scheduler's clock instead.
//!
//! Production means what `file_sizes` counts: every `.rs` file under
//! `crates/*/src/` but test files, with each `#[cfg(test)]` item (a module,
//! an `impl`, a function) skipped through its closing brace.

use std::path::{Path, PathBuf};

/// Each place production code reads the wall clock: the file under
/// `crates/`, how many reads it has, and why each is safe.
const ALLOWED: &[(&str, usize, &str)] = &[
    (
        "rdom-tui/src/runtime/trace.rs",
        1,
        "the RDOM_TRACE debug log's timestamps; never reach paint",
    ),
    (
        "rdom-tui/src/runtime/app/mod.rs",
        1,
        "the scheduler's epoch; only differences from it are observable",
    ),
    (
        "rdom-tui/src/runtime/app/frame.rs",
        1,
        "frame_now: the wall clock only when no virtual clock drives the App — App::run; a headless driver sets with_virtual_clock or calls advance",
    ),
    (
        "rdom-tui/src/runtime/app/event_loop.rs",
        3,
        "App::run's live loop: the per-event clock sync, the poll timeout, the scheduler pump",
    ),
    (
        "rdom-tui/src/runtime/input/reader.rs",
        2,
        "the live terminal reader's poll deadline",
    ),
    (
        "rdom-tui/src/runtime/color_scheme/mod.rs",
        2,
        "the live startup query's wait for the terminal's reply",
    ),
];

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

/// `source` without its `#[cfg(test)]` items: from the attribute through
/// the brace that closes the item's first `{` (or its `;`, for a
/// `mod name;` declaration). Braces in strings and comments are rare
/// enough in this codebase's test items to count as code.
fn production_source(source: &str) -> String {
    let mut out = String::new();
    let mut lines = source.lines();
    while let Some(line) = lines.next() {
        if line.trim() != "#[cfg(test)]" {
            out.push_str(line);
            out.push('\n');
            continue;
        }
        let mut depth = 0i32;
        let mut opened = false;
        for item in lines.by_ref() {
            for c in item.chars() {
                match c {
                    '{' => {
                        depth += 1;
                        opened = true;
                    }
                    '}' => depth -= 1,
                    _ => {}
                }
            }
            if (opened && depth <= 0) || (!opened && item.trim_end().ends_with(';')) {
                break;
            }
        }
    }
    out
}

/// How many wall-clock reads `source` has, outside `//` comments (doc
/// comments name the clock without reading it).
fn reads(source: &str) -> usize {
    source
        .lines()
        .map(|l| l.split_once("//").map_or(l, |(code, _)| code))
        .map(|code| {
            code.matches("Instant::now()").count() + code.matches("SystemTime::now()").count()
        })
        .sum()
}

#[test]
fn production_lines_skip_cfg_test_items() {
    let src = "fn a() { Instant::now() }\n/// reads `Instant::now()`\n#[cfg(test)]\nimpl X {\n    fn t() { Instant::now(); }\n}\nfn b() {}\n#[cfg(test)]\nmod tests;\nfn c() {}\n";
    let p = production_source(src);
    assert_eq!(reads(&p), 1);
    assert!(p.contains("fn b()") && p.contains("fn c()"), "{p}");
}

#[test]
fn production_code_reads_the_wall_clock_only_where_listed() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let mut files = Vec::new();
    for entry in std::fs::read_dir(crates).unwrap() {
        let src = entry.unwrap().path().join("src");
        if src.is_dir() {
            rust_files(&src, &mut files);
        }
    }
    files.sort();
    let mut found: Vec<(String, usize)> = Vec::new();
    for path in files {
        let name = path.file_name().unwrap().to_str().unwrap();
        if is_test_file(name) {
            continue;
        }
        let source = std::fs::read_to_string(&path).unwrap();
        let n = reads(&production_source(&source));
        if n > 0 {
            let rel = path
                .strip_prefix(crates)
                .unwrap()
                .to_str()
                .unwrap()
                .replace('\\', "/");
            found.push((rel, n));
        }
    }
    let mut allowed: Vec<(String, usize)> = ALLOWED
        .iter()
        .map(|(p, n, _)| ((*p).to_string(), *n))
        .collect();
    allowed.sort();
    assert_eq!(
        found, allowed,
        "\nwall-clock reads in production code differ from the list in this file;\n\
         read the scheduler's clock instead, or list the read with its reason"
    );
}

//! C9G-MISC-CORRECTNESS — CLAUDE.md §Architecture Hygiene: the cascade
//! computes styles and knows nothing of boxes, layout or paint. No
//! production module under `style/` reaches into `crate::render` (a
//! predicate both need lives on `ComputedStyle`, in rdom-style).

/// Every production `.rs` file under `dir`, recursively (test modules —
/// `tests.rs`, `*_tests.rs`, `tests/` — aside).
fn production_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("style/ is readable") {
        let path = entry.expect("a directory entry").path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if path.is_dir() {
            if name != "tests" {
                production_files(&path, out);
            }
        } else if name.ends_with(".rs") && name != "tests.rs" && !name.ends_with("_tests.rs") {
            out.push(path);
        }
    }
}

#[test]
fn the_cascade_does_not_depend_on_render_code() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/style");
    let mut files = Vec::new();
    production_files(&dir, &mut files);
    assert!(files.len() > 10, "found the style modules");
    let offenders: Vec<String> = files
        .iter()
        .flat_map(|f| {
            let text = std::fs::read_to_string(f).expect("readable source");
            text.lines()
                .enumerate()
                .filter(|(_, l)| l.contains("crate::render") && !l.trim_start().starts_with("//"))
                .map(|(i, l)| format!("{}:{}: {}", f.display(), i + 1, l.trim()))
                .collect::<Vec<_>>()
        })
        .collect();
    assert!(offenders.is_empty(), "{offenders:#?}");
}

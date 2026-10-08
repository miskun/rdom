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

/// The paths in `source` (the file of module `module`, from the crate
/// root) that lead into `crate::render`, as Rust resolves them: every
/// `use` tree flattened (groups, nested groups, `self`), each path in code,
/// `crate::` / `super::` / `self::` resolved against `module`. Comments
/// and string literals are not code. (An inline `mod` block would shift
/// the module path; no production file under `style/` has one.)
fn offenders(module: &[&str], source: &str) -> Vec<String> {
    let tokens = tokens(&code_only(source));
    let mut paths = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        if tokens[i] == "use" {
            let mut prefix = Vec::new();
            i = use_tree(&tokens, i + 1, &mut prefix, &mut paths);
        } else if tokens[i].starts_with(|c: char| c.is_alphabetic() || c == '_') {
            let mut path = vec![tokens[i].clone()];
            while tokens.get(i + 1).is_some_and(|t| t == "::")
                && tokens
                    .get(i + 2)
                    .is_some_and(|t| t.starts_with(|c: char| c.is_alphabetic()))
            {
                path.push(tokens[i + 2].clone());
                i += 2;
            }
            paths.push(path);
        }
        i += 1;
    }
    paths
        .into_iter()
        .filter(|p| {
            resolve(module, p).is_some_and(|abs| abs.first().is_some_and(|s| s == "render"))
        })
        .map(|p| p.join("::"))
        .collect()
}

/// Parse the `use` tree starting at `tokens[i]` under `prefix`, pushing
/// each flattened path; returns the index of the token that ends it (`,`,
/// `}` or `;`).
fn use_tree(
    tokens: &[String],
    mut i: usize,
    prefix: &mut Vec<String>,
    out: &mut Vec<Vec<String>>,
) -> usize {
    let depth = prefix.len();
    loop {
        match tokens.get(i).map(String::as_str) {
            Some("{") => {
                i += 1;
                loop {
                    i = use_tree(tokens, i, prefix, out);
                    match tokens.get(i).map(String::as_str) {
                        Some(",") => i += 1,
                        _ => break,
                    }
                    if tokens.get(i).is_some_and(|t| t == "}") {
                        break;
                    }
                }
                i += 1; // the `}`
                prefix.truncate(depth);
                return i;
            }
            Some("::") => i += 1,
            Some("as") => i += 2,
            Some("*") => i += 1,
            Some(t) if t.starts_with(|c: char| c.is_alphabetic() || c == '_') => {
                if t != "self" {
                    prefix.push(t.to_string());
                }
                i += 1;
            }
            _ => {
                out.push(prefix.clone());
                prefix.truncate(depth);
                return i;
            }
        }
    }
}

/// `path`, written in module `module`, from the crate root; `None` for a
/// path that does not start at the crate (an extern crate, a local name).
fn resolve(module: &[&str], path: &[String]) -> Option<Vec<String>> {
    let mut base: Vec<String> = module.iter().map(|s| s.to_string()).collect();
    let mut rest = path;
    match rest.first().map(String::as_str)? {
        "crate" => {
            base.clear();
            rest = &rest[1..];
        }
        "super" => {
            while rest.first().is_some_and(|s| s == "super") {
                base.pop()?;
                rest = &rest[1..];
            }
        }
        "self" => rest = &rest[1..],
        _ => return None,
    }
    base.extend(rest.iter().cloned());
    Some(base)
}

/// `source` with its comments and string and character literals blanked.
fn code_only(source: &str) -> String {
    let chars: Vec<char> = source.chars().collect();
    let mut out = String::with_capacity(source.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        if c == '/' && next == Some('/') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
        } else if c == '/' && next == Some('*') {
            i += 2;
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            i += 2;
        } else if c == '"' || (c == 'r' && matches!(next, Some('"' | '#'))) {
            // A string: `"…"` with escapes, or raw `r#"…"#`.
            let raw = c == 'r';
            let mut hashes = 0;
            if raw {
                i += 1;
                while chars.get(i) == Some(&'#') {
                    hashes += 1;
                    i += 1;
                }
            }
            i += 1; // the opening quote
            while i < chars.len() {
                if !raw && chars[i] == '\\' {
                    i += 2;
                    continue;
                }
                if chars[i] == '"' && (0..hashes).all(|k| chars.get(i + 1 + k) == Some(&'#')) {
                    i += 1 + hashes;
                    break;
                }
                i += 1;
            }
            out.push(' ');
        } else if c == '\'' && (chars.get(i + 2) == Some(&'\'') || next == Some('\\')) {
            // A character literal (a lifetime has no closing quote).
            i += 1;
            while i < chars.len() && chars[i] != '\'' {
                i += if chars[i] == '\\' { 2 } else { 1 };
            }
            i += 1;
            out.push(' ');
        } else {
            out.push(c);
            i += 1;
        }
    }
    out
}

/// The tokens a path is made of: identifiers, `::`, and the punctuation
/// of a `use` tree; everything else is a separator.
fn tokens(code: &str) -> Vec<String> {
    let chars: Vec<char> = code.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            out.push(chars[start..i].iter().collect());
            continue;
        }
        if c == ':' && chars.get(i + 1) == Some(&':') {
            out.push("::".to_string());
            i += 2;
            continue;
        }
        if matches!(c, '{' | '}' | ',' | ';' | '*') {
            out.push(c.to_string());
        } else if !c.is_whitespace() {
            out.push(String::from("·"));
        }
        i += 1;
    }
    out
}

/// The module path of `file` under `src/`: `style/cascade/walk.rs` is
/// `style::cascade::walk`, `style/cascade/mod.rs` `style::cascade`.
fn module_of(src: &std::path::Path, file: &std::path::Path) -> Vec<String> {
    let rel = file.strip_prefix(src).expect("under src/");
    let mut parts: Vec<String> = rel
        .iter()
        .map(|p| p.to_string_lossy().trim_end_matches(".rs").to_string())
        .collect();
    if parts.last().is_some_and(|p| p == "mod") {
        parts.pop();
    }
    parts
}

#[test]
fn the_cascade_does_not_depend_on_render_code() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    production_files(&src.join("style"), &mut files);
    assert!(files.len() > 10, "found the style modules");
    let offenders: Vec<String> = files
        .iter()
        .flat_map(|f| {
            let module = module_of(&src, f);
            let module: Vec<&str> = module.iter().map(String::as_str).collect();
            let text = std::fs::read_to_string(f).expect("readable source");
            offenders(&module, &text)
                .into_iter()
                .map(|l| format!("{}: {l}", f.display()))
                .collect::<Vec<_>>()
        })
        .collect();
    assert!(offenders.is_empty(), "{offenders:#?}");
}

/// C10G-MINOR — the scan reads paths as Rust does, so no spelling of a
/// path into `crate::render` evades it: a grouped import, a nested group,
/// a path relative to the file's module (`super`), a path in code; and a
/// name that only starts with `render`, a comment or a string is no path.
#[test]
fn every_spelling_of_a_render_path_is_caught() {
    let walk = ["style", "cascade", "walk"];
    let caught = |src: &str| !offenders(&walk, src).is_empty();
    assert!(caught("use crate::render::Rect;"));
    assert!(caught("use crate::{render::Rect, style::X};"));
    assert!(caught("use crate::{style::X, render::{inline::A, B}};"));
    assert!(caught("pub(crate) use crate::{\n    render,\n};"));
    assert!(caught("use super::super::super::render::Rect;"));
    assert!(caught("let r = crate::render::Rect::default();"));
    assert!(caught(
        "let r = super::super::super::render::Rect::default();"
    ));
    assert!(!caught("use crate::{style::{render_x, Y}};"));
    assert!(
        !caught("use super::super::render::X;"),
        "style::render, not crate::render"
    );
    assert!(!caught("// use crate::render::Rect;"));
    assert!(!caught("let s = \"crate::render\";"));
}

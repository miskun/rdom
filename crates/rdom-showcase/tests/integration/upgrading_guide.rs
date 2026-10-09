//! C15G-UPGRADING — `UPGRADING-0.6.md`, the 0.5 → 0.6 guide, stays
//! navigable: every behaviour change keeps one stable `sc-*` anchor, every
//! `#sc-*` link in the repository's documents resolves to one, and the
//! "Top 15" lists fifteen. Its Rust snippets compile as doctests
//! (`rdom-showcase/src/lib.rs`, `UpgradingDoctests`).
//!
//! It lives here because `rdom-showcase` is never published: the check
//! reads files outside any crate.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The `sc-*` names after `marker` in `text`.
fn names_after<'a>(text: &'a str, marker: &str) -> Vec<&'a str> {
    text.match_indices(marker)
        .map(|(i, _)| {
            let rest = &text[i + marker.len()..];
            let end = rest
                .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
                .unwrap_or(rest.len());
            &rest[..end]
        })
        .collect()
}

/// Every `.md` file of the repository outside `target/` and `.dev/`.
fn markdown(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("readable dir") {
        let path = entry.expect("dir entry").path();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        if path.is_dir() {
            if !matches!(name.as_str(), "target" | ".dev" | ".git") {
                markdown(&path, out);
            }
        } else if name.ends_with(".md") {
            out.push(path);
        }
    }
}

#[test]
fn every_change_has_one_anchor_and_every_link_resolves() {
    let guide = read(&repo().join("UPGRADING-0.6.md"));
    let mut anchors = BTreeMap::new();
    for name in names_after(&guide, "<a id=\"sc-") {
        *anchors.entry(name).or_insert(0) += 1;
    }
    assert!(anchors.len() >= 100, "{} anchors", anchors.len());
    let doubled: Vec<_> = anchors.iter().filter(|(_, n)| **n > 1).collect();
    assert!(doubled.is_empty(), "anchors defined twice: {doubled:?}");

    let mut files = Vec::new();
    markdown(&repo(), &mut files);
    let mut dangling = Vec::new();
    for file in files {
        let text = read(&file);
        // A link's fragment: `(…#sc-name)`; prose naming the form
        // (`#sc-…`) has no name.
        for name in names_after(&text, "#sc-")
            .into_iter()
            .filter(|n| !n.is_empty())
        {
            if !anchors.contains_key(name) {
                dangling.push(format!("{}: sc-{name}", file.display()));
            }
        }
    }
    assert!(dangling.is_empty(), "links to no anchor: {dangling:#?}");
}

#[test]
fn the_top_list_has_fifteen_entries() {
    let guide = read(&repo().join("UPGRADING-0.6.md"));
    let top = guide
        .split("\n## ")
        .find(|s| s.starts_with("Top 15"))
        .expect("a \"## Top 15\" section");
    let entries = top
        .lines()
        .filter(|l| l.starts_with("| ") && l.contains("](#sc-"))
        .count();
    assert_eq!(entries, 15);
}

/// The CHANGELOG points at the guide instead of carrying it.
#[test]
fn the_changelog_points_at_the_guide() {
    let changelog = read(&repo().join("CHANGELOG.md"));
    let unreleased = changelog
        .split("\n## [0.5.0]")
        .next()
        .expect("an [Unreleased] section");
    assert!(unreleased.contains("(UPGRADING-0.6.md)"));
    assert!(
        !unreleased.contains("<a id=\"sc-"),
        "the behaviour-change list moved to UPGRADING-0.6.md"
    );
}

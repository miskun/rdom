//! C16G-HARDENING — the published crates ship no `unsafe` code, and say
//! so to the compiler: `#![forbid(unsafe_code)]` at each crate root, so a
//! later `unsafe` block fails the build instead of a review. rdom-tui's
//! unit tests count allocations through a `GlobalAlloc`
//! (`test_alloc.rs`, `cfg(test)` only), which needs `unsafe`: there the
//! forbid holds outside `cfg(test)`, and under it `unsafe_code` is denied
//! everywhere but that one module.
//!
//! Lives here, in the unpublished crate, because it reads the siblings'
//! sources.

use std::path::Path;

/// Each published crate's root and the attributes it must carry.
const ROOTS: &[(&str, &[&str])] = &[
    ("rdom-core/src/lib.rs", &["#![forbid(unsafe_code)]"]),
    ("rdom-style/src/lib.rs", &["#![forbid(unsafe_code)]"]),
    ("rdom-css/src/lib.rs", &["#![forbid(unsafe_code)]"]),
    ("rdom-parser/src/lib.rs", &["#![forbid(unsafe_code)]"]),
    (
        "rdom-tui/src/lib.rs",
        &[
            "#![cfg_attr(not(test), forbid(unsafe_code))]",
            "#![cfg_attr(test, deny(unsafe_code))]",
            "#[allow(unsafe_code)]\nmod test_alloc;",
        ],
    ),
];

#[test]
fn every_published_crate_forbids_unsafe_code() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut missing = Vec::new();
    for (root, attributes) in ROOTS {
        let text = std::fs::read_to_string(crates.join(root)).expect("a crate root");
        for attribute in *attributes {
            if !text.contains(attribute) {
                missing.push(format!("{root}: {attribute}"));
            }
        }
    }
    assert!(missing.is_empty(), "{missing:#?}");
}

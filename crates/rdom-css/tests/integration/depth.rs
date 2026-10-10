//! Hostile depth (C16G-DEPTH-CAPS, Phase 16 gate architect B4): the
//! parser is bounded against hostile depth as it is against hostile
//! values. Blocks nest at most [`MAX_BLOCK_DEPTH`] deep — style rules
//! (CSS Nesting 1 §2) and the at-rules that hold blocks (`@media`,
//! `@supports`, `@layer`, `@scope`, `@starting-style`) counted together;
//! a deeper block is skipped whole, with `WarningKind::BlockTooDeep`,
//! and the rest of the sheet parses. Each test parses input far past the
//! cap on a small thread stack: the parser recurses at most the cap deep,
//! so it cannot overflow.

use rdom_css::{MAX_BLOCK_DEPTH, ParseResult, WarningKind, parse};

/// The stack the deep-input tests run on: well below a test thread's 2
/// MiB default, and above what the parser needs at its caps.
pub(crate) const SMALL_STACK: usize = 256 * 1024;

/// Run `f` on a thread with a [`SMALL_STACK`].
pub(crate) fn on_small_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(SMALL_STACK)
        .spawn(f)
        .expect("spawn")
        .join()
        .expect("no stack overflow")
}

/// Far past every cap.
const DEEP: usize = 100_000;

fn too_deep(r: &ParseResult) -> usize {
    r.warnings
        .iter()
        .filter(|w| matches!(w.kind, WarningKind::BlockTooDeep))
        .count()
}

/// CSS nesting `a { a { a { … } } }` 100 000 deep: the rules down to the
/// cap stand, the block past it is skipped with one warning, and the
/// rule after the nest parses.
#[test]
fn style_rules_nested_past_the_cap_are_skipped() {
    let css = format!("{}{} q {{ width: 2 }}", "a{".repeat(DEEP), "}".repeat(DEEP));
    let r = on_small_stack(move || parse(&css));
    assert_eq!(
        too_deep(&r),
        1,
        "{:?}",
        &r.warnings[..r.warnings.len().min(3)]
    );
    let rules = r.stylesheet.rules();
    assert_eq!(rules.len(), MAX_BLOCK_DEPTH + 1);
}

/// Nested `@media` 100 000 deep — the conditional rules count against the
/// same cap.
#[test]
fn conditional_rules_nested_past_the_cap_are_skipped() {
    let css = format!(
        "{}p {{ width: 1 }}{} q {{ width: 2 }}",
        "@media screen{".repeat(DEEP),
        "}".repeat(DEEP)
    );
    let r = on_small_stack(move || parse(&css));
    assert_eq!(too_deep(&r), 1);
    assert_eq!(r.stylesheet.rules().len(), 1, "only q: p sits past the cap");
}

/// Style rules and nested at-rules alternating count together.
#[test]
fn mixed_nesting_counts_every_block() {
    let css = format!(
        "{}{} q {{ width: 2 }}",
        ".a{@supports (width: 1){".repeat(DEEP / 2),
        "}".repeat(DEEP)
    );
    let r = on_small_stack(move || parse(&css));
    assert_eq!(too_deep(&r), 1);
    assert!(r.stylesheet.rules().len() <= MAX_BLOCK_DEPTH + 1);
}

/// At the cap itself nothing is dropped.
#[test]
fn nesting_to_the_cap_is_kept() {
    let css = format!(
        "{}color: red{}",
        "a{".repeat(MAX_BLOCK_DEPTH),
        "}".repeat(MAX_BLOCK_DEPTH)
    );
    let r = on_small_stack(move || parse(&css));
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    assert_eq!(r.stylesheet.rules().len(), MAX_BLOCK_DEPTH);
}

/// A condition's parentheses nest at most
/// [`MAX_CONDITION_NESTING`](rdom_style::conditional::MAX_CONDITION_NESTING)
/// deep: past it the prelude does not parse — `@media` takes the
/// malformed query as `not all` (Media Queries 4 §3.2), `@supports` and
/// `@container` are invalid rules (Conditional 3 §6, Conditional 5 §2) —
/// and nothing recurses past the cap.
#[test]
fn condition_parentheses_nested_past_the_cap_do_not_parse() {
    let deep = |at: &str| {
        format!(
            "{at} {}(width > 1){} {{ p {{ width: 1 }} }} q {{ width: 2 }}",
            "(".repeat(DEEP),
            ")".repeat(DEEP)
        )
    };
    for (at, rules) in [("@media", 2), ("@supports", 1), ("@container", 1)] {
        let css = deep(at);
        let r = on_small_stack(move || parse(&css));
        assert_eq!(r.stylesheet.rules().len(), rules, "{at}: {:?}", r.warnings);
    }
}

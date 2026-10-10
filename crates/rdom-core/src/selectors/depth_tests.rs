//! Hostile selector depth (C16G-DEPTH-CAPS, Phase 16 gate architect B4):
//! selector arguments (`:is()`, `:not()`, `:where()`, `:has()`,
//! `:nth-child(… of S)`) nest at most [`MAX_SELECTOR_NESTING`] deep, and
//! a nested rule's `&` expansions grow a selector to at most
//! [`MAX_SELECTOR_SIZE`] simple selectors — `& &` would double it at
//! every level. Past either the selector is invalid (a forgiving
//! `:is()` drops the argument), and nothing recurses past the cap: each
//! deep test runs on a 256 KiB thread stack.

use super::{MAX_SELECTOR_NESTING, MAX_SELECTOR_SIZE, SelectorList, parse, parse_nested};

/// Far past the caps.
const DEEP: usize = 100_000;

/// Run `f` on a thread with a 256 KiB stack.
fn on_small_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(f)
        .expect("spawn")
        .join()
        .expect("no stack overflow")
}

fn nested(function: &str, levels: usize, inner: &str) -> String {
    format!("{}{inner}{}", function.repeat(levels), ")".repeat(levels))
}

/// `:not(:not(…))`, `:where(`, `:nth-child(1 of …` 100 000 deep: an
/// invalid selector, not a stack overflow.
#[test]
fn unforgiving_arguments_past_the_cap_are_invalid() {
    for function in [":not(", ":where(", ":nth-child(1 of "] {
        let text = nested(function, DEEP, "a");
        let parsed = on_small_stack(move || parse(&text).is_ok());
        assert!(!parsed, "{function}");
    }
}

/// `:is()` is forgiving: past the cap its argument is dropped — the
/// selector parses and matches nothing.
#[test]
fn forgiving_arguments_past_the_cap_are_dropped() {
    let text = nested(":is(", DEEP, "a");
    let parsed = on_small_stack(move || parse(&text).map(|l| l.nesting()));
    assert!(
        matches!(parsed, Ok(n) if n <= MAX_SELECTOR_NESTING),
        "{parsed:?}"
    );
}

/// To the cap itself a selector parses; one more level does not.
#[test]
fn nesting_to_the_cap_parses() {
    assert!(parse(&nested(":not(", MAX_SELECTOR_NESTING, "a")).is_ok());
    assert!(parse(&nested(":not(", MAX_SELECTOR_NESTING + 1, "a")).is_err());
}

/// CSS Nesting 1 §2: `& &` nested rule after nested rule doubles the
/// expanded selector each level (`:is(parent) :is(parent)`). The growth
/// stops at the size cap with an invalid selector, long before memory
/// runs out.
#[test]
fn ampersand_doubling_stops_at_the_size_cap() {
    let mut parent: SelectorList = parse("a, b").unwrap();
    let mut refused = false;
    for _ in 0..64 {
        match parse_nested("& &", &parent) {
            Ok(list) => {
                assert!(
                    list.size() <= MAX_SELECTOR_SIZE,
                    "{} simple selectors",
                    list.size()
                );
                parent = list;
            }
            Err(_) => {
                refused = true;
                break;
            }
        }
    }
    assert!(refused, "the doubling never reached the cap");
}

/// An `&` deep in arguments adds its parent's nesting: the sum is capped
/// too, so nesting rules cannot stack nesting past the cap.
#[test]
fn ampersand_nesting_counts_against_the_cap() {
    let parent = parse(&nested(":not(", MAX_SELECTOR_NESTING - 1, "a")).unwrap();
    assert!(parse_nested(":not(&)", &parent).is_ok() || parse_nested("&", &parent).is_ok());
    assert!(parse_nested(":not(:not(&))", &parent).is_err());
}

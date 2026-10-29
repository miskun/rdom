//! Hostile selector depth (C16G-DEPTH-CAPS, Phase 16 gate architect B4):
//! selector arguments (`:is()`, `:not()`, `:where()`, `:has()`,
//! `:nth-child(… of S)`) nest at most [`MAX_SELECTOR_NESTING`] deep, and
//! a nested rule's `&` expansions grow a selector to at most
//! [`MAX_SELECTOR_SIZE`] simple selectors — `& &` would double it at
//! every level. Past either the selector is invalid (a forgiving
//! `:is()` drops the argument), and nothing recurses past the cap: each
//! deep test runs on a 1 MiB thread stack, the smallest main-thread stack
//! rdom runs on (Windows'), where an uncapped parse of [`DEEP`] levels
//! would need hundreds of megabytes.
//!
//! The budget is the real stack, not a tight one: parsing to the cap is
//! bounded but its frames are a build's, not the parser's — measured on
//! macOS (aarch64) at about 5.3 KiB a level in a debug build (6.8 KiB for
//! `:nth-child(… of S)`), 1 KiB in a release build: 195–227 KiB at the
//! cap in debug, 33 KiB in release. A Windows debug build's frames are
//! larger still; a 256 KiB budget overflowed there.

use super::{MAX_SELECTOR_NESTING, MAX_SELECTOR_SIZE, SelectorList, parse, parse_nested};

/// Far past the caps.
const DEEP: usize = 100_000;

/// The smallest main-thread stack rdom runs on: Windows' 1 MiB.
const MAIN_STACK: usize = 1024 * 1024;

/// Run `f` on a thread with a [`MAIN_STACK`] stack.
fn on_small_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(MAIN_STACK)
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

/// To the cap itself a selector parses — within the same stack, the
/// deepest legal selector of each nesting function; one more level does
/// not.
#[test]
fn nesting_to_the_cap_parses() {
    for function in [":not(", ":is(", ":where(", ":nth-child(1 of "] {
        let at_cap = nested(function, MAX_SELECTOR_NESTING, "a");
        let past = nested(function, MAX_SELECTOR_NESTING + 1, "a");
        let (ok, err) = on_small_stack(move || (parse(&at_cap).is_ok(), parse(&past).is_err()));
        assert!(ok, "{function} to the cap");
        if function != ":is(" && function != ":where(" {
            assert!(err, "{function} past the cap");
        }
    }
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

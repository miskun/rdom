//! Selector-text preprocessing before `rdom_core::selectors::parse`:
//! splitting a selector list on its top-level commas, and stripping the
//! pseudo-element suffix (`::before`, `::scrollbar-thumb:vertical`, …)
//! that rdom-core's structural grammar does not accept.

use std::borrow::Cow;

use super::PseudoElementTarget;

// ── Pseudo-element suffix extraction ────────────────────────────────

/// The supported pseudo-element suffixes. Longer suffixes go first:
/// `::scrollbar` is a prefix of `::scrollbar-thumb`, which is a prefix
/// of the axis forms.
const SUFFIXES: [(&str, PseudoElementTarget); 9] = [
    ("::before", PseudoElementTarget::Before),
    ("::after", PseudoElementTarget::After),
    ("::backdrop", PseudoElementTarget::Backdrop),
    ("::placeholder", PseudoElementTarget::Placeholder),
    ("::selection", PseudoElementTarget::Selection),
    (
        "::scrollbar-thumb:vertical",
        PseudoElementTarget::ScrollbarThumbVertical,
    ),
    (
        "::scrollbar-thumb:horizontal",
        PseudoElementTarget::ScrollbarThumbHorizontal,
    ),
    ("::scrollbar-thumb", PseudoElementTarget::ScrollbarThumb),
    ("::scrollbar", PseudoElementTarget::Scrollbar),
];

/// Strip a trailing pseudo-element (`::before`, `::scrollbar-thumb:vertical`,
/// …) if present. Returns the core selector + pseudo target. Errors if
/// more than one pseudo-element is present (not allowed in a single
/// selector) or the pseudo-element is unsupported.
///
/// The pseudo-element attaches to the last compound of the core. When
/// there is none — the core is empty, or ends in whitespace or a
/// combinator (`::before`, `div ::before`, `div > ::after`) — that
/// compound is the implicit universal selector (Selectors 4 §5.2: a
/// compound without a type selector has an implied `*`), so the core
/// returned is the source with `*` appended: `*`, `div *`, `div > *`.
pub(super) fn extract_pseudo_suffix(
    selector: &str,
) -> Result<(Cow<'_, str>, PseudoElementTarget), String> {
    let s = selector.trim_end();

    // Disallow multiple `::` pseudo-elements (`::before::after` is invalid).
    let pseudo_count = s.matches("::").count();
    if pseudo_count > 1 {
        return Err(format!(
            "at most one pseudo-element suffix allowed per selector, found {pseudo_count}"
        ));
    }

    for (suffix, target) in SUFFIXES {
        if let Some(core) = s.strip_suffix(suffix) {
            return Ok((with_compound(core), target));
        }
    }

    // A bare `::other` anywhere is rejected (unsupported pseudo-element).
    if pseudo_count == 1 {
        return Err(
            "unsupported pseudo-element; only ::before, ::after, ::backdrop, ::selection, ::placeholder, ::scrollbar, ::scrollbar-thumb (optionally :vertical / :horizontal) allowed"
                .to_string(),
        );
    }

    Ok((Cow::Borrowed(s), PseudoElementTarget::None))
}

/// `core` (the text before a pseudo-element) with a compound for the
/// pseudo-element to attach to: unchanged when it ends in one, else
/// with the implicit `*` (Selectors 4 §5.2) appended.
fn with_compound(core: &str) -> Cow<'_, str> {
    let ends_in_compound = core.chars().next_back().is_some_and(|last| {
        (!last.is_whitespace() && !matches!(last, '>' | '+' | '~')) || escaped_at_end(core)
    });
    if ends_in_compound {
        Cow::Borrowed(core)
    } else {
        Cow::Owned(format!("{core}*"))
    }
}

/// True when `core`'s last character is escaped (`a\ ` — an escaped
/// space or combinator is part of an identifier, CSS Syntax 3 §4.3.7):
/// an odd run of backslashes precedes it.
fn escaped_at_end(core: &str) -> bool {
    let mut chars = core.chars();
    chars.next_back();
    chars.rev().take_while(|&c| c == '\\').count() % 2 == 1
}

// ── Top-level comma splitting ───────────────────────────────────────

/// Split on commas that are NOT inside `(...)` or `[...]`. Returns
/// borrowed slices into `input`. Empty input yields empty vec.
///
/// `:not(a, b)` has one internal comma; we preserve it as part of the
/// single selector item because it sits inside parens.
pub(super) fn split_top_level_commas(input: &str) -> Vec<&str> {
    if input.trim().is_empty() {
        return Vec::new();
    }
    let bytes = input.as_bytes();
    let mut depth_paren: i32 = 0;
    let mut depth_bracket: i32 = 0;
    let mut start = 0;
    let mut out = Vec::new();

    let mut escaped = false;
    for (i, &b) in bytes.iter().enumerate() {
        // `\,` is an escaped comma inside an identifier (CSS Syntax 3
        // §4.3.7), not a list separator.
        if escaped {
            escaped = false;
            continue;
        }
        match b {
            b'\\' => escaped = true,
            b'(' => depth_paren += 1,
            b')' => depth_paren = (depth_paren - 1).max(0),
            b'[' => depth_bracket += 1,
            b']' => depth_bracket = (depth_bracket - 1).max(0),
            b',' if depth_paren == 0 && depth_bracket == 0 => {
                out.push(&input[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(&input[start..]);
    out
}
